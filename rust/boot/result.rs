use super::*;

/// Default result type for the library.
pub type Result<T> = std::result::Result<T, Error>;

/// Default error type for the library.
pub struct Error {
	msg: String,
}

impl Error {
	pub fn new<T: std::error::Error>(msg: T) -> Self {
		Self { msg: format!("{msg}") }
	}
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.msg)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.msg)
	}
}

impl<T: std::error::Error> From<T> for Error {
	fn from(value: T) -> Self {
		Error::new(value)
	}
}
