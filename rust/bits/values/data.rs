use super::*;

#[derive(Copy, Clone)]
pub union Data<'a> {
	pub bool: bool,
	pub char: char,

	pub u8: u8,
	pub u16: u16,
	pub u32: u32,
	pub u64: u64,

	pub i8: i8,
	pub i16: i16,
	pub i32: i32,
	pub i64: i64,

	pub f32: f32,
	pub f64: f64,

	pub isize: isize,
	pub usize: usize,

	pub ptr: *const u8,
	pub typ: RuntimeType<'a>,
	pub sym: Symbol,
}

unsafe impl<'a> Send for Data<'a> {}
unsafe impl<'a> Sync for Data<'a> {}

impl<'a> Data<'a> {
	#[inline]
	pub fn zero() -> Self {
		unsafe { std::mem::MaybeUninit::zeroed().assume_init() }
	}

	#[inline]
	pub fn bool(&self) -> bool {
		unsafe { self.bool }
	}

	#[inline]
	pub fn char(&self) -> char {
		unsafe { self.char }
	}

	#[inline]
	pub fn u8(&self) -> u8 {
		unsafe { self.u8 }
	}

	#[inline]
	pub fn u16(&self) -> u16 {
		unsafe { self.u16 }
	}

	#[inline]
	pub fn u32(&self) -> u32 {
		unsafe { self.u32 }
	}

	#[inline]
	pub fn u64(&self) -> u64 {
		unsafe { self.u64 }
	}

	#[inline]
	pub fn i8(&self) -> i8 {
		unsafe { self.i8 }
	}

	#[inline]
	pub fn i16(&self) -> i16 {
		unsafe { self.i16 }
	}

	#[inline]
	pub fn i32(&self) -> i32 {
		unsafe { self.i32 }
	}

	#[inline]
	pub fn i64(&self) -> i64 {
		unsafe { self.i64 }
	}

	#[inline]
	pub fn f32(&self) -> f32 {
		unsafe { self.f32 }
	}

	#[inline]
	pub fn f64(&self) -> f64 {
		unsafe { self.f64 }
	}

	#[inline]
	pub fn usize(&self) -> usize {
		unsafe { self.usize }
	}

	#[inline]
	pub fn isize(&self) -> isize {
		unsafe { self.isize }
	}

	#[inline]
	pub fn ptr(&self) -> *const u8 {
		unsafe { self.ptr }
	}

	#[inline]
	pub fn ptr_of<T>(&self) -> *const T {
		unsafe { self.ptr as *const T }
	}

	#[inline]
	pub unsafe fn as_ref<T>(&self) -> &T {
		unsafe { &*(self.ptr as *const T) }
	}

	#[inline]
	pub unsafe fn str(&self) -> &'static str {
		let chunk = ChunkOf::<u8>::from_ptr(self.ptr);
		let bytes = chunk.as_slice();
		std::str::from_utf8_unchecked(bytes)
	}

	#[inline]
	pub unsafe fn typ(&self) -> RuntimeType<'a> {
		unsafe { self.typ }
	}

	#[inline]
	pub unsafe fn sym(&self) -> Symbol {
		unsafe { self.sym }
	}
}
