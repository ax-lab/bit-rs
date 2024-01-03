use std::any::type_name;

use super::*;

#[derive(Default)]
struct ContextData<'a> {
	types: ContextCell<'a, TypeContext<'a>>,
	sources: ContextCell<'a, SourceContext<'a>>,
	nodes: ContextCell<'a, NodeContext<'a>>,
	lexer: ContextCell<'a, LexerContext<'a>>,
	ops: ContextCell<'a, OpContext<'a>>,
}

impl<'a> ContextData<'a> {
	fn new(&self, ctx: ContextRef<'a>) {
		self.types.new(ctx);
		self.sources.new(ctx);
		self.nodes.new(ctx);
		self.lexer.new(ctx);
		self.ops.new(ctx);
	}

	fn init(&self) {
		self.types.init();
		self.sources.init();
		self.nodes.init();
		self.lexer.init();
		self.ops.init();
	}
}

impl<'a> ContextRef<'a> {
	pub fn types(&self) -> &'a TypeContext<'a> {
		self.data().types.get()
	}

	pub fn sources(&self) -> &'a SourceContext<'a> {
		self.data().sources.get()
	}

	pub fn nodes(&self) -> &'a NodeContext<'a> {
		self.data().nodes.get()
	}

	pub fn lexer(&self) -> &'a LexerContext<'a> {
		self.data().lexer.get()
	}

	pub fn ops(&self) -> &'a OpContext<'a> {
		self.data().ops.get()
	}
}

/// Trait for types that can be used as part of the [`Context`] data while
/// also storing a reference to the context.
pub trait IsContext<'a> {
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

/// Global context for the language.
///
/// The [`Context`] owns all data (e.g. types, values) for the compiler
/// and runtime.
///
/// A context can only be used and passed around through a [`ContextRef`].
pub struct Context {
	// the lifetime of this data is actually the struct's own lifetime
	ptr: *const InnerContext<'static>,
}

struct InnerContext<'a> {
	init: AtomicBool,
	data: ContextData<'a>,
	arena: Arena,
}

impl Context {
	pub fn new() -> Self {
		let context = InnerContext {
			init: false.into(),
			arena: Arena::new(),
			data: unsafe { MaybeUninit::zeroed().assume_init() },
		};
		let context = Box::leak(Box::new(context));

		let ctx = ContextRef { ptr: context };
		context.data.new(ctx);
		context.init.store(true, FLAG_SYNC);

		context.data.init();

		Self { ptr: context }
	}

	/// Get a reference to the context.
	#[inline]
	pub fn get<'a>(&'a self) -> ContextRef<'a> {
		ContextRef {
			// SAFETY: the lifetime of InnerContext is the same as self
			ptr: unsafe { std::mem::transmute(self.ptr) },
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

/// Stores a [`Context`] reference.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ContextRef<'a> {
	ptr: *const InnerContext<'a>,
}

impl<'a> ContextRef<'a> {
	pub fn store<T>(&self, value: T) -> &'a T {
		self.arena().store(value)
	}

	#[inline]
	pub fn arena(&self) -> &'a Arena {
		&self.inner().arena
	}

	#[inline]
	fn data(&self) -> &'a ContextData<'a> {
		&self.inner().data
	}

	#[inline]
	fn inner(&self) -> &'a InnerContext<'a> {
		let ctx = unsafe { &*self.ptr };
		debug_assert!(ctx.init.load(FLAG_SYNC) == true, "trying to use uninitialized context");
		ctx
	}
}

const FLAG_SYNC: SyncOrder = SyncOrder::Relaxed;

/// Provides safe initialization of an [`IsContext`] inside a [`ContextData`].
struct ContextCell<'a, T: IsContext<'a>> {
	state: AtomicU8,
	inner: UnsafeCell<MaybeUninit<T>>,
	tag: PhantomData<&'a ()>,
}

impl<'a, T: IsContext<'a>> Default for ContextCell<'a, T> {
	fn default() -> Self {
		Self {
			state: Default::default(),
			inner: MaybeUninit::zeroed().into(),
			tag: Default::default(),
		}
	}
}

impl<'a, T: IsContext<'a> + 'a> ContextCell<'a, T> {
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

impl<'a, T: IsContext<'a> + 'a> Deref for ContextCell<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.get()
	}
}
