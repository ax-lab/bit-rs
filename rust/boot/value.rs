use super::*;

pub trait IsValue: 'static + Debug {
	fn as_writable(&self) -> Option<&dyn Writable> {
		None
	}

	fn value_type(&self) -> TypeId {
		TypeId::of::<Self>()
	}
}

#[derive(Copy, Clone)]
pub struct Value {
	data: *const ValueData<()>,
}

struct ValueData<T> {
	vtable: Option<&'static dyn IsValue>,
	value: T,
}

impl Value {
	#[inline(always)]
	pub fn new<T: IsValue>(value: T) -> Value {
		let data = ValueData { vtable: None, value };
		let data = Arena::get().store(data);
		data.vtable = Some(&data.value);

		let data = (data as *const ValueData<T>).cast::<ValueData<()>>();
		Value { data }
	}

	#[inline(always)]
	pub fn get(&self) -> &'static dyn IsValue {
		unsafe {
			let data = &*self.data;
			data.vtable.unwrap_unchecked()
		}
	}

	#[inline(always)]
	pub fn cast<T>(&self) -> Option<&'static T> {
		let value = self.get();
		if value.value_type() == TypeId::of::<T>() {
			unsafe {
				let data = &*(self.data as *const ValueData<T>);
				Some(&data.value)
			}
		} else {
			None
		}
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
