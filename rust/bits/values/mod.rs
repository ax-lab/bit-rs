use super::*;

pub mod data;
pub mod expr;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NodeValue<'a> {
	None,
	Unit,
	Bool(bool),
	Str(&'a str),
	SInt(i64),
	UInt(u64),
	Source(Source<'a>),
	Module(Source<'a>),
	Token(Token),
	LetDecl(Symbol),
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
	For,
	While,
}

impl<'a> NodeValue<'a> {
	pub fn as_bool(&self) -> Result<bool> {
		let value = match self {
			&NodeValue::None => false,
			&NodeValue::Unit => false,
			&NodeValue::Bool(v) => v,
			&NodeValue::Str(v) => v.len() > 0,
			&NodeValue::SInt(v) => v != 0,
			&NodeValue::UInt(v) => v != 0,
			_ => format!("value is not a valid boolean: {self}").err()?,
		};
		Ok(value)
	}

	pub fn is_block(&self) -> bool {
		match self {
			NodeValue::Group { .. } => true,
			NodeValue::Sequence { .. } => true,
			_ => false,
		}
	}
}

impl<'a> Display for NodeValue<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			NodeValue::None => write!(f, ""),
			NodeValue::Unit => write!(f, "()"),
			NodeValue::Bool(v) => write!(f, "{v}"),
			NodeValue::Str(v) => write!(f, "{v}"),
			NodeValue::SInt(v) => write!(f, "{v}"),
			NodeValue::UInt(v) => write!(f, "{v}"),
			NodeValue::Token(tok) => write!(f, "{tok}"),
			_ => write!(f, "{self:?}"),
		}
	}
}

impl<'a> Debug for NodeValue<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			NodeValue::None => write!(f, "(none)"),
			NodeValue::Unit => write!(f, "()"),
			NodeValue::Bool(v) => write!(f, "{v:?}"),
			NodeValue::Str(v) => write!(f, "{v:?}"),
			NodeValue::SInt(v) => write!(f, "{v:?}"),
			NodeValue::UInt(v) => write!(f, "{v:?}"),
			NodeValue::Token(tok) => write!(f, "Token({tok:?})"),
			NodeValue::Source(src) => write!(f, "Source({src:?})"),
			NodeValue::Module(src) => write!(f, "Module({src:?})"),
			NodeValue::LetDecl(var) => write!(f, "LetDecl({var:?})"),
			NodeValue::Let(var) => write!(f, "Let({var:?})"),
			NodeValue::Var(var) => write!(f, "Var({var:?})"),
			NodeValue::BinaryOp(op) => write!(f, "BinaryOp({op})"),
			NodeValue::Group { scoped } => write!(f, "Group(scoped={scoped})"),
			NodeValue::Sequence { scoped, indented } => write!(f, "Sequence(scoped={scoped}, indented={indented})"),
			NodeValue::Print => write!(f, "Print"),
			NodeValue::Indent(up) => write!(f, "Ident({up})"),
			NodeValue::If => write!(f, "If"),
			NodeValue::ElseIf => write!(f, "ElseIf"),
			NodeValue::Else => write!(f, "Else"),
			NodeValue::For => write!(f, "For"),
			NodeValue::While => write!(f, "While"),
		}
	}
}

impl<'a> Writable for NodeValue<'a> {
	fn write(&self, f: &mut Writer) -> Result<()> {
		self.write_fmt(f)
	}
}

impl<'a> Node<'a> {
	#[inline]
	pub fn key(self) -> NodeValue<'a> {
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
		let a = ctx.node(NodeValue::Unit, Span::empty());
		assert_eq!(NodeValue::Unit, a.value());
		assert_eq!("()", format!("{a}"));
		assert_eq!("()", format!("{a:?}"));

		let a = ctx.node(NodeValue::Bool(true), Span::empty());
		assert_eq!(NodeValue::Bool(true), a.value());
		assert_eq!("true", format!("{a}"));
		assert_eq!("true", format!("{a:?}"));

		let a = ctx.node(NodeValue::Bool(false), Span::empty());
		assert_eq!(NodeValue::Bool(false), a.value());
		assert_eq!("false", format!("{a}"));
		assert_eq!("false", format!("{a:?}"));

		let a = ctx.node(NodeValue::SInt(42), Span::empty());
		assert_eq!(NodeValue::SInt(42), a.value());
		assert_eq!("42", format!("{a}"));
		assert_eq!("42", format!("{a:?}"));

		let a = ctx.node(NodeValue::UInt(69), Span::empty());
		assert_eq!(NodeValue::UInt(69), a.value());
		assert_eq!("69", format!("{a}"));
		assert_eq!("69", format!("{a:?}"));

		let a = ctx.node(NodeValue::Str("abc123"), Span::empty());
		assert_eq!(NodeValue::Str("abc123"), a.value());
		assert_eq!("abc123", format!("{a}"));
		assert_eq!("\"abc123\"", format!("{a:?}"));
	}
}
