use std::alloc::Layout;

use super::*;

//====================================================================================================================//
// Arena
//====================================================================================================================//

const DEFAULT_ALIGN: usize = 64;
const DEFAULT_ARENA: usize = 512 * MB;

pub struct Arena {
	size: usize,
	next: AtomicUsize,
	data: NonNull<u8>,
}

unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}

impl Arena {
	#[inline(always)]
	pub fn get() -> &'static Self {
		static GLOBAL: ArenaInit = ArenaInit::new();
		GLOBAL.get()
	}

	pub fn new(size: usize) -> Self {
		let layout = Layout::from_size_align(size, DEFAULT_ALIGN).unwrap();
		let data = unsafe { std::alloc::alloc(layout) };
		let data = if let Some(data) = NonNull::new(data) {
			unsafe { data.as_ptr().write_bytes(0xBD, size) };
			data
		} else {
			panic!("Arena: failed to create new with size of {}", to_bytes(size));
		};
		Self {
			size,
			next: 0.into(),
			data,
		}
	}

	#[inline(always)]
	pub fn store<T>(&self, value: T) -> &mut T {
		unsafe { self.alloc(value).as_mut() }
	}

	#[inline(always)]
	pub fn alloc<T>(&self, value: T) -> NonNull<T> {
		let ptr = self.alloc_layout(Layout::for_value(&value)).cast::<T>();
		unsafe { ptr.as_ptr().write(value) }
		ptr
	}

	pub fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
		let align = layout.align();
		let size = std::cmp::max(1, layout.size()); // make sure every address is unique
		let data = self.data;
		loop {
			let next = self.next.load(Order::Relaxed);
			let addr = align_to(next, align);
			debug_assert!(addr % align == 0);
			let addr_end = addr + size;
			if addr_end > self.size {
				let size = to_bytes(size);
				let remaining = to_bytes(self.size - next);
				panic!("Arena: could not allocate {size} (remaining {remaining})");
			}

			// the allocation is valid, try to commit
			if self
				.next
				.compare_exchange_weak(next, addr_end, Order::Relaxed, Order::Relaxed)
				.is_ok()
			{
				unsafe {
					return NonNull::new_unchecked(data.as_ptr().add(addr));
				}
			}
		}
	}
}

impl Drop for Arena {
	fn drop(&mut self) {
		unsafe {
			let data = self.data.as_ptr();
			let layout = Layout::from_size_align(self.size, DEFAULT_ALIGN).unwrap();
			data.write_bytes(0xBF, self.size);
			std::alloc::dealloc(data, layout);
		}
	}
}

/// Supports lazy initialization for a static arena.
struct ArenaInit {
	data: AtomicPtr<Arena>,
	init: Once,
}

unsafe impl Send for ArenaInit {}
unsafe impl Sync for ArenaInit {}

impl ArenaInit {
	pub const fn new() -> Self {
		Self {
			data: AtomicPtr::new(std::ptr::null_mut()),
			init: Once::new(),
		}
	}

	#[inline(always)]
	pub fn get(&self) -> &'static Arena {
		let data = self.data.load(Order::Relaxed);
		let data = if data.is_null() { self.init() } else { data };
		debug_assert!(!data.is_null());
		unsafe { &*data }
	}

	fn init(&self) -> *mut Arena {
		self.init.call_once(|| {
			let data = Box::leak(Box::new(Arena::new(DEFAULT_ARENA)));
			self.data.store(data, Order::Relaxed);
		});
		let data = self.data.load(Order::Relaxed);
		debug_assert!(!data.is_null());
		data
	}
}

#[inline(always)]
fn align_to(value: usize, align_to: usize) -> usize {
	debug_assert!(align_to.is_power_of_two());
	(value + align_to - 1) & !(align_to - 1)
}

//====================================================================================================================//
// Init
//====================================================================================================================//

/// Provides a lazily initiated static value backed by the global [`Arena`].
pub struct Init<T> {
	data: AtomicPtr<T>,
	init: std::sync::Once,
	func: UnsafeCell<fn() -> T>,
}

unsafe impl<T> Sync for Init<T> {}

impl<T> Init<T> {
	pub const fn new(func: fn() -> T) -> Self {
		Self {
			data: AtomicPtr::new(std::ptr::null_mut()),
			init: std::sync::Once::new(),
			func: UnsafeCell::new(func),
		}
	}

	pub const fn default() -> Self
	where
		T: Default,
	{
		Self::new(|| T::default())
	}

	#[inline(always)]
	pub fn get(&self) -> &'static T {
		let data = self.data.load(Order::Relaxed);
		let data = if data.is_null() { self.init() } else { data };
		debug_assert!(!data.is_null());
		unsafe { &*data }
	}

	#[inline(always)]
	pub fn value(&self) -> T
	where
		T: Copy + 'static,
	{
		*self.get()
	}

	fn init(&self) -> *mut T {
		self.init.call_once(|| {
			let func = unsafe { &*self.func.get() };
			let data = Arena::get().alloc(func());
			self.data.store(data.as_ptr(), Order::Relaxed);
		});
		self.data.load(Order::Relaxed)
	}
}

//====================================================================================================================//
// Tests
//====================================================================================================================//

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn arena_works() {
		let arena = Arena::new(128 * KB);
		let ptr1 = arena.alloc_layout(Layout::from_size_align(128, 64).unwrap());
		unsafe { ptr1.as_ptr().write_bytes(0x01, 128) };

		let ptr2 = arena.alloc_layout(Layout::from_size_align(1, 64).unwrap());
		let ptr3 = arena.alloc_layout(Layout::from_size_align(1, 1).unwrap());
		let ptr4 = arena.alloc_layout(Layout::from_size_align(1, 1024).unwrap());

		let expect = unsafe { ptr1.as_ptr().add(128) };
		assert_eq!(expect, ptr2.as_ptr());

		let expect = unsafe { ptr2.as_ptr().add(1) };
		assert_eq!(expect, ptr3.as_ptr());

		let expect = unsafe { ptr1.as_ptr().add(1024) };
		assert_eq!(expect, ptr4.as_ptr());

		let value = arena.store(123);
		assert_eq!(123, *value);
	}

	#[test]
	fn global_arena_works() {
		let arena = Arena::get();
		let ptr = arena.alloc_layout(Layout::from_size_align(128, 64).unwrap());
		unsafe { ptr.as_ptr().write_bytes(0x01, 128) };
	}

	#[test]
	fn init_works() {
		static ANS: Init<X> = Init::default();

		assert_eq!(X(42), ANS.value());
		assert_eq!(X(42), ANS.value());
		assert_eq!(X(43), X::default());
		assert_eq!(X(44), X::default());

		#[derive(Copy, Clone, Eq, PartialEq, Debug)]
		struct X(usize);

		impl Default for X {
			fn default() -> Self {
				static CHANGE: AtomicUsize = AtomicUsize::new(42);
				Self(CHANGE.fetch_add(1, Order::Relaxed))
			}
		}
	}
}
