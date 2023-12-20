use super::*;

#[derive(Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct XValueCell {
	pub val: XValue,
}

impl XValueCell {
	pub fn new<T: Into<XValue>>(val: T) -> Self {
		Self { val: val.into() }
	}

	pub fn kind(&self) -> XKind {
		self.val.kind()
	}
}

impl Debug for XValueCell {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let kind = self.kind();
		write!(f, "<{kind:?}>({})", self.val)
	}
}

impl Display for XValueCell {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.val)
	}
}

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum XValue {
	None,
	Unit,
	Bool(bool),
	Int(Int),
	Float(Float),
	Str(Arc<str>),
	Array(Array),
}

impl XValue {
	pub fn kind(&self) -> XKind {
		match self {
			XValue::None => XKind::None,
			XValue::Unit => XKind::Unit,
			XValue::Bool(_) => XKind::Bool,
			XValue::Int(v) => XKind::Int(v.kind()),
			XValue::Float(v) => XKind::Float(v.kind()),
			XValue::Str(_) => XKind::Str,
			XValue::Array(v) => v.kind(),
		}
	}
}

impl Display for XValue {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			XValue::None => write!(f, "(none)"),
			XValue::Unit => write!(f, "()"),
			XValue::Bool(v) => write!(f, "{v}"),
			XValue::Int(v) => write!(f, "{v}"),
			XValue::Float(v) => write!(f, "{v}"),
			XValue::Str(v) => write!(f, "{v}"),
			XValue::Array(v) => write!(f, "{v}"),
		}
	}
}

impl Default for XValue {
	fn default() -> Self {
		XValue::None
	}
}
