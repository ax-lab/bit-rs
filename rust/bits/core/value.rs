use super::*;

#[derive(Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Data {
	pub val: Value,
}

impl Data {
	pub fn new<T: Into<Value>>(val: T) -> Self {
		Self { val: val.into() }
	}

	pub fn kind(&self) -> Kind {
		self.val.kind()
	}
}

impl Debug for Data {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let kind = self.kind();
		write!(f, "<{kind:?}>({})", self.val)
	}
}

impl Display for Data {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.val)
	}
}

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Value {
	None,
	Unit,
	Bool(bool),
	Int(Int),
	Float(Float),
	Str(Arc<str>),
	Array(Array),
}

impl Value {
	pub fn kind(&self) -> Kind {
		match self {
			Value::None => Kind::None,
			Value::Unit => Kind::Unit,
			Value::Bool(_) => Kind::Bool,
			Value::Int(v) => Kind::Int(v.kind()),
			Value::Float(v) => Kind::Float(v.kind()),
			Value::Str(_) => Kind::Str,
			Value::Array(v) => v.kind(),
		}
	}
}

impl Display for Value {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Value::None => write!(f, "(none)"),
			Value::Unit => write!(f, "()"),
			Value::Bool(v) => write!(f, "{v}"),
			Value::Int(v) => write!(f, "{v}"),
			Value::Float(v) => write!(f, "{v}"),
			Value::Str(v) => write!(f, "{v}"),
			Value::Array(v) => write!(f, "{v}"),
		}
	}
}

impl Default for Value {
	fn default() -> Self {
		Value::None
	}
}
