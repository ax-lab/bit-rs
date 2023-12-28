use super::*;

pub mod data;
pub mod expr;

struct ValueContext<'a> {
	#[allow(unused)]
	ctx: ContextRef<'a>,
}

impl<'a> IsContext<'a> for ValueContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		Self { ctx }
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value<'a> {
	Unit,
	Bool(bool),
	Str(&'a str),
	SInt(i64),
	UInt(u64),
}

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Unit => write!(f, ""),
			Value::Bool(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v}"),
			Value::SInt(v) => write!(f, "{v}"),
			Value::UInt(v) => write!(f, "{v}"),
		}
	}
}

impl<'a> Debug for Value<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Unit => write!(f, "()"),
			Value::Bool(v) => write!(f, "{v:?}"),
			Value::Str(v) => write!(f, "{v:?}"),
			Value::SInt(v) => write!(f, "{v:?}"),
			Value::UInt(v) => write!(f, "{v:?}"),
		}
	}
}

impl<'a> ContextRef<'a> {
	pub fn unit(&self) -> Node<'a> {
		self.node(Value::Unit)
	}

	pub fn bool(&self, bool: bool) -> Node<'a> {
		self.node(Value::Bool(bool))
	}

	pub fn str<T: AsRef<str>>(&self, str: T) -> Node<'a> {
		let str = self.arena().str(str);
		self.node(Value::Str(str))
	}

	pub fn uint(&self, value: u64) -> Node<'a> {
		self.node(Value::UInt(value))
	}

	pub fn sint(&self, value: i64) -> Node<'a> {
		self.node(Value::SInt(value))
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
		assert_eq!(&Value::Unit, a.value());
		assert_eq!("", format!("{a}"));
		assert_eq!("()", format!("{a:?}"));

		let a = ctx.bool(true);
		assert_eq!(&Value::Bool(true), a.value());
		assert_eq!("true", format!("{a}"));
		assert_eq!("true", format!("{a:?}"));

		let a = ctx.bool(false);
		assert_eq!(&Value::Bool(false), a.value());
		assert_eq!("false", format!("{a}"));
		assert_eq!("false", format!("{a:?}"));

		let a = ctx.sint(42);
		assert_eq!(&Value::SInt(42), a.value());
		assert_eq!("42", format!("{a}"));
		assert_eq!("42", format!("{a:?}"));

		let a = ctx.uint(69);
		assert_eq!(&Value::UInt(69), a.value());
		assert_eq!("69", format!("{a}"));
		assert_eq!("69", format!("{a:?}"));

		let a = ctx.str("abc123");
		assert_eq!(&Value::Str("abc123"), a.value());
		assert_eq!("abc123", format!("{a}"));
		assert_eq!("\"abc123\"", format!("{a:?}"));
	}
}
