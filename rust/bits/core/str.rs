use super::*;

impl From<&str> for Value {
	fn from(value: &str) -> Self {
		Value::Str(value.into())
	}
}

impl From<String> for Value {
	fn from(value: String) -> Self {
		Value::Str(value.into())
	}
}
