use super::*;

#[derive(Copy, Clone)]
pub struct Value<'a> {
	typ: Type<'a>,
	data: ValueData<'a>,
}

impl<'a> ContextRef<'a> {
	pub fn unit(&self) -> Value<'a> {
		let typ = self.types().unit();
		let data = ValueData::zero();
		Value { typ, data }
	}

	pub fn bool(&self, bool: bool) -> Value<'a> {
		let typ = self.types().builtin(Primitive::Bool);
		let dat = ValueData { bool };
		Value { typ, data: dat }
	}

	pub fn str<T: AsRef<str>>(&self, str: T) -> Value<'a> {
		let store = self.arena();
		let typ = self.types().builtin(Primitive::String);
		let str = store.chunk_from_slice(str.as_ref().as_bytes());
		let ptr = str.as_ptr();
		let dat = ValueData { ptr };
		Value { typ, data: dat }
	}

	pub fn u8(&self, u8: u8) -> Value<'a> {
		let typ = self.types().builtin(Primitive::UInt(8));
		let dat = ValueData { u8 };
		Value { typ, data: dat }
	}
}

impl<'a> Value<'a> {
	#[inline]
	pub fn context(&self) -> ContextRef<'a> {
		self.typ.context()
	}

	#[inline]
	pub fn store(&self) -> &'a Store {
		self.context().arena()
	}

	pub fn get_type(&self) -> Type<'a> {
		self.typ
	}

	pub fn data(&self) -> ValueData<'a> {
		self.data
	}

	pub fn get_bool(&self) -> Option<bool> {
		if self.typ.is_builtin(Primitive::Bool) {
			Some(self.data.bool())
		} else {
			None
		}
	}

	pub fn get_u8(&self) -> Option<u8> {
		if self.typ.is_builtin(Primitive::UInt(8)) {
			Some(self.data.u8())
		} else {
			None
		}
	}

	pub fn get_str(&self) -> Option<&'static str> {
		if self.typ.is_builtin(Primitive::String) {
			Some(unsafe { self.data.str() })
		} else {
			None
		}
	}
}

impl<'a> Debug for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let typ = self.get_type();
		typ.debug_value(*self, f)?;
		write!(f, "<{typ:?}>")?;
		Ok(())
	}
}

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		self.get_type().display_value(*self, f)
	}
}

#[derive(Copy, Clone)]
pub union ValueData<'a> {
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
	pub typ: Type<'a>,
	pub sym: Symbol,
}

unsafe impl<'a> Send for ValueData<'a> {}
unsafe impl<'a> Sync for ValueData<'a> {}

impl<'a> ValueData<'a> {
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
	pub unsafe fn typ(&self) -> Type<'a> {
		unsafe { self.typ }
	}

	#[inline]
	pub unsafe fn sym(&self) -> Symbol {
		unsafe { self.sym }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn builtin_values() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let a = ctx.unit();
		assert_eq!("()", format!("{a}"));
		assert_eq!("()<()>", format!("{a:?}"));

		let a = ctx.bool(true);
		assert_eq!(Some(true), a.get_bool());
		assert_eq!("true", format!("{a}"));
		assert_eq!("true<bool>", format!("{a:?}"));

		let a = ctx.bool(false);
		assert_eq!(Some(false), a.get_bool());
		assert_eq!("false", format!("{a}"));
		assert_eq!("false<bool>", format!("{a:?}"));

		let a = ctx.u8(42);
		assert_eq!(Some(42), a.get_u8());
		assert_eq!("42", format!("{a}"));
		assert_eq!("42<u8>", format!("{a:?}"));

		let a = ctx.u8(69);
		assert_eq!(Some(69), a.get_u8());
		assert_eq!("69", format!("{a}"));
		assert_eq!("69<u8>", format!("{a:?}"));
	}

	#[test]
	pub fn builtin_str() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let a = ctx.str("abc");
		assert_eq!(Some("abc"), a.get_str());

		let a = ctx.str("123456");
		assert_eq!(Some("123456"), a.get_str());
		assert_eq!("\"123456\"<string>", format!("{a:?}"));

		let a = ctx.str("");
		assert_eq!(Some(""), a.get_str());
	}
}
