use super::*;

pub mod data;
pub mod expr;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value<'a> {
	None,
	Unit,
	Bool(bool),
	Str(&'a str),
	SInt(i64),
	UInt(u64),
	Source(Source<'a>),
}

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::None => write!(f, ""),
			Value::Unit => write!(f, "()"),
			Value::Bool(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v}"),
			Value::SInt(v) => write!(f, "{v}"),
			Value::UInt(v) => write!(f, "{v}"),
			Value::Source(src) => write!(f, "Source({src})"),
		}
	}
}

impl<'a> Debug for Value<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::None => write!(f, "(none)"),
			Value::Unit => write!(f, "()"),
			Value::Bool(v) => write!(f, "{v:?}"),
			Value::Str(v) => write!(f, "{v:?}"),
			Value::SInt(v) => write!(f, "{v:?}"),
			Value::UInt(v) => write!(f, "{v:?}"),
			Value::Source(src) => write!(f, "{src:?}"),
		}
	}
}

impl<'a> Node<'a> {
	#[inline]
	pub fn key(self) -> Value<'a> {
		self.value()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn builtin_values() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let a = ctx.node(Value::Unit, Span::empty());
		assert_eq!(Value::Unit, a.value());
		assert_eq!("", format!("{a}"));
		assert_eq!("()", format!("{a:?}"));

		let a = ctx.node(Value::Bool(true), Span::empty());
		assert_eq!(Value::Bool(true), a.value());
		assert_eq!("true", format!("{a}"));
		assert_eq!("true", format!("{a:?}"));

		let a = ctx.node(Value::Bool(false), Span::empty());
		assert_eq!(Value::Bool(false), a.value());
		assert_eq!("false", format!("{a}"));
		assert_eq!("false", format!("{a:?}"));

		let a = ctx.node(Value::SInt(42), Span::empty());
		assert_eq!(Value::SInt(42), a.value());
		assert_eq!("42", format!("{a}"));
		assert_eq!("42", format!("{a:?}"));

		let a = ctx.node(Value::UInt(69), Span::empty());
		assert_eq!(Value::UInt(69), a.value());
		assert_eq!("69", format!("{a}"));
		assert_eq!("69", format!("{a:?}"));

		let a = ctx.node(Value::Str("abc123"), Span::empty());
		assert_eq!(Value::Str("abc123"), a.value());
		assert_eq!("abc123", format!("{a}"));
		assert_eq!("\"abc123\"", format!("{a:?}"));
	}
}
