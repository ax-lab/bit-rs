use super::*;

impl From<bool> for XValue {
	fn from(value: bool) -> Self {
		XValue::Bool(value)
	}
}
