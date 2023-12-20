use super::*;

#[derive(Copy, Clone)]
pub struct Value {
	typ: Type,
	data: ValueData,
}

impl Value {
	pub fn unit() -> Self {
		let typ = Type::unit();
		let data = ValueData::zero();
		Self { typ, data }
	}

	pub fn bool(bool: bool) -> Self {
		let typ = Type::builtin(Primitive::Bool);
		let dat = ValueData { bool };
		Self { typ, data: dat }
	}

	pub fn u8(u8: u8) -> Self {
		let typ = Type::builtin(Primitive::UInt(8));
		let dat = ValueData { u8 };
		Self { typ, data: dat }
	}

	pub fn get_type(&self) -> Type {
		self.typ
	}

	pub fn data(&self) -> ValueData {
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
}

impl Debug for Value {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let typ = self.get_type();
		typ.debug_value(*self, f)?;
		write!(f, "<{typ:?}>")?;
		Ok(())
	}
}

impl Display for Value {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		self.get_type().display_value(*self, f)
	}
}

#[derive(Copy, Clone)]
pub union ValueData {
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

	pub ptr: *const (),
	pub typ: Type,
	pub sym: Symbol,
}

unsafe impl Send for ValueData {}
unsafe impl Sync for ValueData {}

impl ValueData {
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
	pub fn ptr(&self) -> *const () {
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
	pub unsafe fn typ(&self) -> Type {
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
		let a = Value::unit();
		assert_eq!("()", format!("{a}"));
		assert_eq!("()<()>", format!("{a:?}"));

		let a = Value::bool(true);
		assert_eq!(Some(true), a.get_bool());
		assert_eq!("true", format!("{a}"));
		assert_eq!("true<bool>", format!("{a:?}"));

		let a = Value::bool(false);
		assert_eq!(Some(false), a.get_bool());
		assert_eq!("false", format!("{a}"));
		assert_eq!("false<bool>", format!("{a:?}"));

		let a = Value::u8(42);
		assert_eq!(Some(42), a.get_u8());
		assert_eq!("42", format!("{a}"));
		assert_eq!("42<u8>", format!("{a:?}"));

		let a = Value::u8(69);
		assert_eq!(Some(69), a.get_u8());
		assert_eq!("69", format!("{a}"));
		assert_eq!("69<u8>", format!("{a:?}"));
	}
}
