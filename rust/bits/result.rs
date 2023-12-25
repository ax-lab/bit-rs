use std::any::TypeId;

use super::*;

/// Default result type for this library.
pub type Result<T> = std::result::Result<T, Error>;

/// Default error type for this library.
#[derive(Clone)]
pub struct Error {
	info: ErrorInfo,
	file: FileInfo,
	show_details: bool,
}

impl Error {
	pub fn new<T: IsError>(error: T) -> Self {
		let kind = error.into();
		Self {
			info: kind,
			file: FileInfo::default(),
			show_details: false,
		}
	}

	pub fn is<T: IsError>(&self) -> bool {
		self.info.type_id == TypeId::of::<T>()
	}

	pub fn get<T: IsError>(&self) -> Option<&T> {
		if self.is::<T>() {
			Some(unsafe { &*(self.info.value.as_ref() as *const dyn IsError as *const T) })
		} else {
			None
		}
	}

	pub fn raise<T>(self) -> Result<T> {
		Err(self)
	}

	pub fn at_file(mut self, file: &'static str, line: u32) -> Self {
		self.file = FileInfo { name: file, line };
		self
	}

	pub fn detailed(mut self) -> Self {
		self.show_details = true;
		self
	}

	pub fn output_details<T: std::fmt::Write>(&self, out: &mut T) -> std::fmt::Result {
		if self.file.name != "" {
			let FileInfo { name, line } = self.file;
			write!(out, " -- at {name}:{line}")?;
		}
		Ok(())
	}
}

mod macros {
	/// Return an error result with a formatted message.
	#[macro_export]
	macro_rules! err {
		($msg:literal $($args:tt)*) => {{
			err!(= $msg $($args)*).raise()
		}};

		($expr:expr) => {{
			err!(= $expr).raise()
		}};

		(= $msg:literal $($args:tt)*) => {{
			let msg = format!($msg $($args)*);
			err!(= msg)
		}};

		(= $expr:expr) => {{
			const FILE: &'static str = file!();
			const LINE: u32 = line!();
			$crate::result::Error::new($expr).at_file(FILE, LINE)
		}};
	}

	/// Convert a [std::result::Result] to an error result.
	#[macro_export]
	macro_rules! chk {
		($expr:expr) => {{
			const FILE: &'static str = file!();
			const LINE: u32 = line!();
			match $expr {
				Ok(v) => Ok(v),
				Err(err) => $crate::result::Error::new(err).at_file(FILE, LINE).raise(),
			}
		}};
	}
}

pub use macros::*;

pub trait ResultExtension<T> {
	fn raise(self) -> Result<T>;
}

impl<T, E: std::error::Error + 'static> ResultExtension<T> for std::result::Result<T, E> {
	fn raise(self) -> Result<T> {
		match self {
			Ok(v) => Ok(v),
			Err(err) => Err(err.to_error()),
		}
	}
}

#[derive(Default, Clone)]
struct FileInfo {
	name: &'static str,
	line: u32,
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "Error({:?}", self.info)?;
		self.output_details(f)?;
		write!(f, ")")?;
		Ok(())
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		self.info.display(f)?;
		if self.show_details {
			self.output_details(f)?;
		}
		Ok(())
	}
}

pub trait IsError: Display + Debug + std::any::Any {
	fn to_error(self) -> Error
	where
		Self: Sized,
	{
		Error::new(self)
	}

	fn err<T>(self) -> Result<T>
	where
		Self: Sized,
	{
		self.to_error().raise()
	}
}

impl<T: Display + Debug + std::any::Any> IsError for T {}

#[derive(Clone)]
pub struct ErrorInfo {
	type_id: TypeId,
	value: Arc<dyn IsError>,
	debug_fn: fn(&dyn IsError, &mut Formatter<'_>) -> std::fmt::Result,
	display_fn: fn(&dyn IsError, &mut Formatter<'_>) -> std::fmt::Result,
}

impl ErrorInfo {
	fn display(&self, f: &mut Formatter) -> std::fmt::Result {
		(self.display_fn)(&self.value, f)
	}
}

impl<T: IsError> From<T> for ErrorInfo {
	fn from(value: T) -> ErrorInfo {
		return ErrorInfo {
			type_id: TypeId::of::<T>(),
			value: Arc::new(value),
			debug_fn: |v: &dyn IsError, f: &mut Formatter<'_>| write!(f, "{v:?}"),
			display_fn: |v: &dyn IsError, f: &mut Formatter<'_>| write!(f, "{v}"),
		};
	}
}

impl Debug for ErrorInfo {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		(self.debug_fn)(&self.value, f)
	}
}
