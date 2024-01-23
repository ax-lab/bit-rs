use super::*;

pub trait IsValue: 'static + Debug {
	fn process(&self, msg: Message) -> Result<bool> {
		let _ = msg;
		Ok(false)
	}

	fn say_id(&self, label: &str) {
		println!("{label} {} at {:?}", std::any::type_name::<Self>(), self as *const Self);
	}

	fn bind(&self, node: Node) {
		let _ = node;
	}

	fn as_writable(&self) -> Option<&dyn Writable> {
		None
	}

	fn value_type(&self) -> TypeId {
		TypeId::of::<Self>()
	}
}

impl<T: IsValue> From<T> for Value {
	fn from(value: T) -> Self {
		Value::new(value)
	}
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Value {
	data: NonNull<ValueData<()>>,
}

impl Value {
	#[inline(always)]
	pub fn new<T: IsValue>(value: T) -> Value {
		let data = ValueData { vtable: None, value };

		let mut data = Arena::get().alloc(data);
		unsafe {
			let data = data.as_mut();
			data.vtable = Some(&data.value);
		}

		let value = Value { data: data.cast() };
		value
	}

	#[inline(always)]
	pub fn get(&self) -> &'static dyn IsValue {
		unsafe {
			let data = self.data.as_ref();
			data.vtable.unwrap_unchecked()
		}
	}

	#[inline(always)]
	pub fn cast<T>(&self) -> Option<&'static T> {
		let value = self.get();
		if value.value_type() == TypeId::of::<T>() {
			unsafe {
				let data = self.data.cast::<ValueData<T>>().as_ref();
				Some(&data.value)
			}
		} else {
			None
		}
	}

	pub fn process(&self, msg: Message) -> Result<bool> {
		self.get().process(msg)
	}
}

impl Debug for Value {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let value = self.get();
		if let Some(value) = value.as_writable() {
			let mut writer = Writer::fmt(f).debug();
			value.write(&mut writer)?;
		} else {
			write!(f, "{value:?}")?;
		}
		Ok(())
	}
}

impl Display for Value {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let value = self.get();
		if let Some(value) = value.as_writable() {
			let mut writer = Writer::fmt(f);
			value.write(&mut writer)?;
		} else {
			write!(f, "{value:?}")?;
		}
		Ok(())
	}
}

impl Writable for Value {
	fn write(&self, f: &mut Writer) -> Result<()> {
		let value = self.get();
		if let Some(value) = value.as_writable() {
			value.write(f)
		} else {
			value.write_debug(f)
		}
	}
}

pub(crate) struct ValueCell {
	data: AtomicPtr<ValueData<()>>,
}

impl ValueCell {
	#[inline(always)]
	pub fn new<T: Into<Value>>(value: T) -> Self {
		let value = value.into();
		Self {
			data: AtomicPtr::new(value.data.as_ptr()),
		}
	}

	#[inline(always)]
	pub fn get(&self) -> Value {
		let data = self.data.load(Order::Relaxed);
		let data = unsafe { NonNull::new_unchecked(data) };
		Value { data }
	}

	#[inline(always)]
	pub fn set(&self, value: Value) {
		self.data.store(value.data.as_ptr(), Order::Relaxed);
	}
}

#[repr(C)]
struct ValueData<T> {
	vtable: Option<&'static dyn IsValue>,
	value: T,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_value() {
		let a = Value::new(TestValue(42));
		let b = Value::new(TestValue(69));

		assert_eq!(None, a.cast::<i32>());
		assert_eq!(None, a.cast::<&str>());

		assert_eq!(&TestValue(42), a.cast().unwrap());
		assert_eq!(&TestValue(69), b.cast().unwrap());

		assert_eq!("TestValue(42)", format!("{a:?}"));
		assert_eq!("Value(69)", format!("{b}"));
	}

	#[derive(Debug, Eq, PartialEq)]
	struct TestValue(i32);

	impl IsValue for TestValue {
		fn as_writable(&self) -> Option<&dyn Writable> {
			Some(self)
		}
	}

	impl Display for TestValue {
		fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
			write!(f, "Value({})", self.0)
		}
	}

	writable!(TestValue);
}
