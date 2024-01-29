use super::*;

/// Default result type for the library.
pub type Result<T> = std::result::Result<T, Error>;

/// Default error type for the library.
#[derive(Clone)]
pub enum Error {
	Single(ErrorData),
	List(ErrorList),
}

#[derive(Clone)]
pub struct ErrorData {
	msg: Arc<str>,
	span: Option<Span>,
	file: Option<FileInfo>,
}

#[derive(Clone)]
pub struct ErrorList {
	head: ErrorData,
	next: Option<&'static ErrorList>,
}

impl Error {
	#[inline(always)]
	pub fn new<T: Display>(msg: T) -> Self {
		let data = ErrorData {
			msg: format!("{msg}").into(),
			span: None,
			file: None,
		};
		Error::Single(data)
	}

	pub fn unwrap_iter<T: IntoIterator<Item = Result<U>>, U>(elems: T) -> Result<&'static [U]> {
		let mut out = Vec::new();
		let mut err: Option<Error> = None;
		for it in elems {
			match it {
				Ok(val) => {
					if err.is_none() {
						out.push(val)
					}
				}
				Err(next) => {
					if let Some(curr) = err {
						err = Some(curr.append(next));
					} else {
						err = Some(next);
					}
				}
			}
		}
		if let Some(err) = err {
			Err(err)
		} else {
			let out = Arena::get().slice(out);
			Ok(out)
		}
	}

	pub fn at_file(mut self, file: &'static str, line: u32) -> Self {
		let file = Some(FileInfo { file, line });
		match self {
			Error::Single(ref mut data) => {
				data.file = file;
			}
			Error::List(ref mut list) => {
				list.head.file = file;
			}
		}
		self
	}

	pub fn at<T: HasSpan>(mut self, at: T) -> Self {
		let span = at.span();
		match self {
			Error::Single(ref mut data) => {
				data.span = Some(span);
			}
			Error::List(ref mut list) => {
				list.head.span = Some(span);
			}
		}
		self
	}

	pub fn append(self, next: Error) -> Error {
		let list = match next {
			Error::Single(head) => ErrorList { head, next: None },
			Error::List(list) => list,
		};

		let next_list = Arena::get().store(list);
		let list = match self {
			Error::Single(head) => ErrorList {
				head,
				next: Some(next_list),
			},
			Error::List(mut list) => {
				if let Some(next) = list.next {
					list.next = Some(next.append(next_list));
				} else {
					list.next = Some(next_list);
				}
				list
			}
		};

		Error::List(list)
	}

	fn data(&self) -> &ErrorData {
		match self {
			Error::Single(data) => data,
			Error::List(list) => &list.head,
		}
	}
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let data = self.data();
		write!(f, "{}", data.msg)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let out = &mut Writer::fmt(f);
		match self {
			Error::Single(data) => {
				data.output(out, false)?;
			}
			Error::List(list) => {
				write!(out, "Error: multiple errors:").map_err(|_| std::fmt::Error)?;

				let out = &mut out.indented();
				let mut next = Some(list);
				while let Some(node) = next {
					write!(out, "\n\n").map_err(|_| std::fmt::Error)?;
					node.head.output(out, true).map_err(|_| std::fmt::Error)?;
					next = node.next;
				}
			}
		}
		Ok(())
	}
}

impl ErrorData {
	fn output(&self, f: &mut Writer, list: bool) -> Result<()> {
		let indent = if list {
			write!(f, ">>> ")?;
			"    "
		} else {
			write!(f, "Error: ")?;
			"       "
		};

		if self.msg.contains("\n") {
			for (n, it) in self.msg.lines().enumerate() {
				if n == 0 {
					write!(f, "{it}\n")?;
				} else {
					write!(f, "\n{indent}||  {it}")?;
				}
			}
		} else {
			write!(f, "{}", self.msg)?;
		}

		if let Some(span) = self.span {
			write!(f, "\n\n{indent}@ {span}")?;
			if let Some(context) = span.display_text(0) {
				write!(f, "\n{indent}: {context}")?;
			}
		}
		if let Some(info) = self.file {
			write!(f, "\n\n{indent}(from {}:{})", info.file, info.line)?;
		}
		Ok(())
	}
}

impl ErrorList {
	fn append(&self, list: &'static ErrorList) -> &'static ErrorList {
		let head = self.head.clone();
		let list = if let Some(next) = self.next {
			let list = next.append(list);
			ErrorList { head, next: Some(list) }
		} else {
			ErrorList { head, next: Some(list) }
		};
		let list = Arena::get().store(list);
		list
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
