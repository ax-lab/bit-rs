use super::*;

/// Default result type for the library.
pub type Result<T> = std::result::Result<T, Error>;

/// Default error type for the library.
#[derive(Clone)]
pub struct Error {
	msg: Arc<str>,
	span: Option<Span>,
	file: Option<FileInfo>,
}

impl Error {
	#[inline(always)]
	pub fn new<T: Display>(msg: T) -> Self {
		Self {
			msg: format!("{msg}").into(),
			span: None,
			file: None,
		}
	}

	pub fn at_file(mut self, file: &'static str, line: u32) -> Self {
		self.file = Some(FileInfo { file, line });
		self
	}

	pub fn at<T: HasSpan>(mut self, at: T) -> Self {
		self.span = Some(at.span());
		self
	}
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.msg)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if self.msg.contains("\n") {
			write!(f, "Error: ")?;
			for (n, it) in self.msg.lines().enumerate() {
				if n == 0 {
					write!(f, "{it}\n")?;
				} else {
					write!(f, "\n       ||  {it}")?;
				}
			}
		} else {
			write!(f, "Error: {}", self.msg)?;
		}

		if let Some(span) = self.span {
			write!(f, "\n\n       @ {span}")?;
			if let Some(context) = span.display_text(0) {
				write!(f, "\n       : {context}")?;
			}
		}
		if let Some(info) = self.file {
			write!(f, "\n\n       (from {}:{})", info.file, info.line)?;
		}
		Ok(())
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

#[derive(Copy, Clone)]
struct FileInfo {
	file: &'static str,
	line: u32,
}

mod macros {
	/// Return an error result with a formatted message.
	#[macro_export]
	macro_rules! err {
		($( @ $at:expr => )? $msg:literal $($args:tt)*) => {{
			let msg = format!($msg $($args)*);
			err!(= $(@ $at => )? msg)
		}};

		($( @ $at:expr => )? $expr:expr) => {{
			err!(= $(@ $at => )? $expr)
		}};

		(= $( @ $at:expr => )? $expr:expr) => {{
			let file = file!();
			let line = line!();
			let expr = $expr;
			let msg = format!("{expr}");
			Error::new(msg).at_file(file, line) $(.at($at))?
		}};
	}

	#[macro_export]
	macro_rules! raise {
		($($args:tt)+) => {
			Err(err!($($args)*))?
		}
	}
}

pub use macros::*;
