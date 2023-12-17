use super::*;

impl From<bool> for Value {
	fn from(value: bool) -> Self {
		Value::Bool(value)
	}
}
