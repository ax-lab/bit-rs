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
	Module(Source<'a>),
	Token(Token),
	Let(Var<'a>),
	Var(Var<'a>),
	BinaryOp(OpKey),
	Group { scoped: bool },
	Sequence { scoped: bool, indented: bool },
	Print,
	Indent(bool),
	If,
	ElseIf,
	Else,
}

impl<'a> Value<'a> {
	pub fn as_bool(&self) -> Result<bool> {
		let value = match self {
			&Value::None => false,
			&Value::Unit => false,
			&Value::Bool(v) => v,
			&Value::Str(v) => v.len() > 0,
			&Value::SInt(v) => v != 0,
			&Value::UInt(v) => v != 0,
			_ => format!("value is not a valid boolean: {self}").err()?,
		};
		Ok(value)
	}

	pub fn is_block(&self) -> bool {
		match self {
			Value::Group { .. } => true,
			Value::Sequence { .. } => true,
			_ => false,
		}
	}
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
			Value::Token(tok) => write!(f, "{tok}"),
			_ => write!(f, "{self:?}"),
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
			Value::Token(tok) => write!(f, "Token({tok:?})"),
			Value::Source(src) => write!(f, "Source({src:?})"),
			Value::Module(src) => write!(f, "Module({src:?})"),
			Value::Let(var) => write!(f, "Let({var:?})"),
			Value::Var(var) => write!(f, "Var({var:?})"),
			Value::BinaryOp(op) => write!(f, "BinaryOp({op})"),
			Value::Group { scoped } => write!(f, "Group(scoped={scoped})"),
			Value::Sequence { scoped, indented } => write!(f, "Sequence(scoped={scoped}, indented={indented})"),
			Value::Print => write!(f, "Print"),
			Value::Indent(up) => write!(f, "Ident({up})"),
			Value::If => write!(f, "If"),
			Value::ElseIf => write!(f, "ElseIf"),
			Value::Else => write!(f, "Else"),
		}
	}
}

impl<'a> Writable for Value<'a> {
	fn write(&self, f: &mut Writer) -> Result<()> {
		self.write_fmt(f)
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
		assert_eq!("()", format!("{a}"));
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
