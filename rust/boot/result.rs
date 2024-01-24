use super::*;

/// Default result type for the library.
pub type Result<T> = std::result::Result<T, Error>;

/// Default error type for the library.
#[derive(Clone)]
pub struct Error {
	msg: Arc<str>,
}

impl Error {
	#[inline(always)]
	pub fn new<T: Display>(msg: T) -> Self {
		Self {
			msg: format!("{msg}").into(),
		}
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

impl From<Error> for std::fmt::Error {
	fn from(_: Error) -> Self {
		Self
	}
}

mod macros {
	/// Return an error result with a formatted message.
	#[macro_export]
	macro_rules! err {
		($msg:literal $($args:tt)*) => {{
			let msg = format!($msg $($args)*);
			err!(= msg)
		}};

		($expr:expr) => {{
			err!(= $expr)
		}};

		(= $expr:expr) => {{
			let file = file!();
			let line = line!();
			let expr = $expr;
			let msg = format!("{expr}\n-- from {file}:{line}");
			Error::new(msg)
		}};
	}
}

pub use macros::*;
