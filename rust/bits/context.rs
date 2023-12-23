use std::any::type_name;

use super::*;

const FLAG_SYNC: SyncOrder = SyncOrder::Relaxed;

/// Trait for types that can be used as part of the [`Context`] data.
pub trait IsContext<'a>: Send + Sync + UnwindSafe {
	/// Return a new instance of the [`IsContext`] with an uninitialized
	/// reference to the context.
	///
	/// The reference should not be used before the [`IsContext::init`] method
	/// is called.
	fn new(ctx: ContextRef<'a>) -> Self;

	/// Called to initialize the [`IsContext`] once the parent [`Context`] is
	/// initialized.
	fn init(&mut self) {}
}

/// Main language context.
pub struct Context {
	ptr: *const InnerContext<'static>,
}

struct InnerContext<'a> {
	init: AtomicBool,
	data: ContextData<'a>,
}

impl Context {
	pub fn new() -> Self {
		let context = MaybeUninit::<InnerContext>::zeroed();
		let context = Box::leak(Box::new(unsafe { context.assume_init() }));

		let ctx = ContextRef { ptr: context };
		context.data.new(ctx);
		context.init.store(true, FLAG_SYNC);

		context.data.init();

		Self { ptr: context }
	}

	pub fn get<'a>(&'a self) -> ContextRef<'a> {
		unsafe {
			// SAFETY: change the 'static lifetime to the real self lifetime
			std::mem::transmute(ContextRef { ptr: self.ptr })
		}
	}
}

impl Drop for Context {
	fn drop(&mut self) {
		// SAFETY: reconstruct the box from `Context::new`
		let context = unsafe { Box::from_raw(self.ptr as *mut InnerContext) };
		context.init.store(false, FLAG_SYNC);
		drop(context);
		self.ptr = std::ptr::null_mut();
	}
}

/// Context reference.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ContextRef<'a> {
	ptr: *const InnerContext<'a>,
}

impl<'a> ContextRef<'a> {
	#[inline]
	pub fn data(&self) -> &ContextData<'a> {
		let ctx = unsafe { &*self.ptr };
		debug_assert!(ctx.init.load(FLAG_SYNC) == true, "trying to use uninitialized context");
		&ctx.data
	}
}

impl<'a> Deref for ContextRef<'a> {
	type Target = ContextData<'a>;

	fn deref(&self) -> &Self::Target {
		self.data()
	}
}

// SAFETY: we assert that `ContextData` is safe to Send + Sync + UnwindSafe.
impl<'a> UnwindSafe for ContextRef<'a> {}
unsafe impl<'a> Send for ContextRef<'a> {}
unsafe impl<'a> Sync for ContextRef<'a> {}

/// Provides safe initialization of an [`IsContext`] inside a [`ContextData`].
pub struct ContextCell<'a, T: IsContext<'a>> {
	state: AtomicU8,
	inner: UnsafeCell<MaybeUninit<T>>,
	tag: PhantomData<&'a ()>,
}

// SAFETY: the inner `IsContext` is Send + Sync + UnwindSafe by definition.
impl<'a, T: IsContext<'a>> UnwindSafe for ContextCell<'a, T> {}
unsafe impl<'a, T: IsContext<'a>> Send for ContextCell<'a, T> {}
unsafe impl<'a, T: IsContext<'a>> Sync for ContextCell<'a, T> {}

impl<'a, T: IsContext<'a>> Default for ContextCell<'a, T> {
	fn default() -> Self {
		Self {
			state: Default::default(),
			inner: MaybeUninit::zeroed().into(),
			tag: Default::default(),
		}
	}
}

impl<'a, T: IsContext<'a>> ContextCell<'a, T> {
	pub fn new(&self, ctx: ContextRef<'a>) {
		self.state
			.compare_exchange(0, 1, FLAG_SYNC, FLAG_SYNC)
			.expect("ContextCell: new called again after initialization");
		unsafe {
			self.mut_ptr().write(T::new(ctx));
		}
	}

	pub fn init(&self) {
		self.get(); // trigger the init logic in get
	}

	#[inline]
	pub fn get(&self) -> &T {
		// make sure we are initialized the first time we are used
		// (in case we are used during the context initialization)
		if self.state.compare_exchange(1, 2, FLAG_SYNC, FLAG_SYNC).is_ok() {
			unsafe { &mut *self.mut_ptr() }.init();
		}
		debug_assert_eq!(self.state.load(FLAG_SYNC), 2, "{} used before setup", type_name::<T>());
		unsafe { &*self.mut_ptr() }
	}

	unsafe fn mut_ptr(&self) -> *mut T {
		let inner = &mut *self.inner.get();
		inner.as_mut_ptr()
	}
}

impl<'a, T: IsContext<'a>> Deref for ContextCell<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.get()
	}
}
