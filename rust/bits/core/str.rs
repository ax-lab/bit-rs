use super::*;

impl From<&str> for XValue {
	fn from(value: &str) -> Self {
		XValue::Str(value.into())
	}
}

impl From<String> for XValue {
	fn from(value: String) -> Self {
		XValue::Str(value.into())
	}
}
