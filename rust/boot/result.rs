use super::*;

/// Default result type for the library.
pub type Result<T> = std::result::Result<T, Error>;

/// Default error type for the library.
pub struct Error {
	msg: String,
}

impl Error {
	pub fn new<T: Into<String>>(msg: T) -> Self {
		Self { msg: msg.into() }
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
