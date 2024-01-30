use super::*;

pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;
pub const GB: usize = 1024 * MB;
pub const TB: usize = 1024 * GB;

pub const INDENT: &'static str = "    ";
pub const CR: u8 = '\r' as u8;
pub const LF: u8 = '\n' as u8;

pub fn text<T: AsRef<str>>(text: T) -> String {
	let mut output = String::new();
	let mut prefix = "";
	let mut first = true;
	let text = text.as_ref().trim_end();
	for (n, line) in text.lines().enumerate() {
		let line = line.trim_end();
		if n == 0 && line.len() == 0 {
			continue;
		}

		if !first {
			output.push('\n');
		}

		let mut line = if first {
			first = false;
			let len = line.len() - line.trim_start().len();
			prefix = &line[..len];
			&line[len..]
		} else if line.starts_with(prefix) {
			&line[prefix.len()..]
		} else {
			line
		};

		while line.len() > 0 && line.chars().next() == Some('\t') {
			line = &line[1..];
			output.push_str("    ");
		}
		output.push_str(line);
	}
	output
}

pub fn indent<T: Display>(value: T) -> String {
	indent_with(value, INDENT, INDENT)
}

pub fn indent_with<T: Display, U: Display, V: AsRef<str>>(value: T, prefix: U, indent: V) -> String {
	let mut output = String::new();

	{
		let writer = Writer::fmt(&mut output);
		let writer = &mut writer.indented_with(indent);
		let _ = write!(writer, "{prefix}{value}");
	}

	output
}

/// Helper trait for objects that allow output.
pub trait Writable {
	fn write(&self, f: &mut Writer) -> Result<()>;

	fn get_repr(&self) -> String {
		let mut out = String::new();
		let mut f = Writer::fmt(&mut out);
		let _ = self.write(&mut f);
		drop(f);
		out
	}

	fn format(&self, f: &mut Formatter) -> std::fmt::Result {
		let mut out = Writer::fmt(f);
		match self.write(&mut out) {
			Ok(_) => Ok(()),
			Err(_) => Err(std::fmt::Error),
		}
	}

	fn format_debug(&self, f: &mut Formatter) -> std::fmt::Result {
		let mut out = Writer::fmt(f).debug();
		match self.write(&mut out) {
			Ok(_) => Ok(()),
			Err(_) => Err(std::fmt::Error),
		}
	}
}

/// Adapter for using a [`std::fmt::Write`] with [`std::io::Write`].
pub struct FormatWriter<'a> {
	output: &'a mut dyn std::fmt::Write,
}

impl<'a> FormatWriter<'a> {
	pub fn new<T: std::fmt::Write>(output: &'a mut T) -> Self {
		Self { output }
	}
}

impl<'a> Write for FormatWriter<'a> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let buf = std::str::from_utf8(buf).expect("FormatWriter: invalid UTF-8 text");
		match self.output.write_str(buf) {
			Ok(_) => Ok(buf.len()),
			Err(err) => Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
		}
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

pub trait ToFormatWriter: std::fmt::Write + Sized {
	fn writer(&mut self) -> FormatWriter {
		FormatWriter::new(self)
	}
}

impl<T: std::fmt::Write + Sized> ToFormatWriter for T {}

pub fn to_bytes(bytes: usize) -> String {
	let mut output = String::new();
	{
		let mut output = output.writer();
		let _ = write_bytes(&mut output, bytes);
	}
	output
}

pub fn write_bytes<T: Write>(out: &mut T, bytes: usize) -> Result<()> {
	if bytes == 1 {
		write!(out, "1 byte")
	} else if bytes < KB {
		write!(out, "{bytes} bytes")
	} else if bytes < MB {
		write!(out, "{} KB", bytes / KB)
	} else if bytes < GB {
		write!(out, "{:.1} MB", (bytes as f64) / (MB as f64))
	} else {
		write!(out, "{:.2} GB", (bytes as f64) / (GB as f64))
	}?;
	Ok(())
}

#[derive(Clone)]
pub struct Writer<'a> {
	output: Arc<Mutex<dyn Write + 'a>>,
	indent: Arc<String>,
	state: Arc<WriteState>,
	debug: bool,
}

#[derive(Default)]
struct WriteState {
	new_line: AtomicBool,
	was_cr: AtomicBool,
	written: AtomicUsize,
}

impl WriteState {
	pub fn new_line(&self) -> bool {
		self.new_line.load(Order::Relaxed)
	}

	pub fn was_cr(&self) -> bool {
		self.was_cr.load(Order::Relaxed)
	}

	pub fn set_new_line(&self, value: bool) {
		self.new_line.store(value, Order::Relaxed)
	}

	pub fn set_was_cr(&self, value: bool) {
		self.was_cr.store(value, Order::Relaxed)
	}
}

impl<'a> Writer<'a> {
	pub fn new<T: Write + 'a>(input: T) -> Self {
		Self {
			output: Arc::new(Mutex::new(input)),
			indent: Default::default(),
			state: Default::default(),
			debug: false,
		}
	}

	pub fn written(&self) -> usize {
		self.state.written.load(Order::Relaxed)
	}

	pub fn fmt<T: std::fmt::Write + 'a>(output: &'a mut T) -> Self {
		let writer = FormatWriter { output };
		Self::new(writer)
	}

	pub fn stderr() -> Self {
		Self::new(std::io::stderr())
	}

	pub fn stdout() -> Self {
		Self::new(std::io::stdout())
	}

	pub fn indent(&mut self) {
		self.indent_with(INDENT)
	}

	pub fn dedent(&mut self) {
		self.dedent_with(INDENT)
	}

	pub fn indent_with<T: AsRef<str>>(&mut self, prefix: T) {
		let prefix = prefix.as_ref();
		if prefix.len() > 0 {
			let indent = Arc::make_mut(&mut self.indent);
			indent.push_str(prefix);
		}
	}

	pub fn dedent_with<T: AsRef<str>>(&mut self, suffix: T) {
		let suffix = suffix.as_ref();
		if suffix.len() > 0 && self.indent.ends_with(suffix) {
			let indent = Arc::make_mut(&mut self.indent);
			indent.truncate(indent.len() - suffix.len());
		}
	}

	pub fn indented(&self) -> Self {
		self.indented_with(INDENT)
	}

	pub fn is_debug(&self) -> bool {
		self.debug
	}

	pub fn debug(&self) -> Self {
		let mut out = self.clone();
		out.debug = true;
		out
	}

	pub fn indented_with<T: AsRef<str>>(&self, prefix: T) -> Self {
		let mut out = self.clone();
		out.indent_with(prefix);
		out
	}
}

impl<'a> std::fmt::Write for Writer<'a> {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		if let Err(_) = self.write_all(s.as_bytes()) {
			Err(std::fmt::Error)
		} else {
			Ok(())
		}
	}
}

impl<'a> Write for Writer<'a> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let len = buf.len();
		if len == 0 {
			return Ok(0);
		}

		let mut output = self.output.lock().unwrap();
		let indent = self.indent.as_bytes();

		let mut push = |bytes: &[u8], indented: bool| -> std::io::Result<usize> {
			if let Some(&last) = bytes.last() {
				if indented && self.state.new_line() {
					output.write(indent)?;
					self.state.written.fetch_add(indent.len(), Order::Relaxed);
				}
				self.state.set_new_line(last == CR || last == LF);
				self.state.set_was_cr(last == CR);
				let len = output.write(bytes)?;
				self.state.written.fetch_add(bytes.len(), Order::Relaxed);
				Ok(len)
			} else {
				Ok(0)
			}
		};

		let mut cur = 0;
		for i in 0..len {
			let chr = buf[i];
			if chr == CR || chr == LF {
				if i > cur {
					push(&buf[cur..i], true)?;
				}
				if chr == CR || !self.state.was_cr() {
					push("\n".as_bytes(), false)?;
				}
				self.state.set_was_cr(chr == CR);
				self.state.set_new_line(true);
				cur = i + 1;
			}
		}
		push(&buf[cur..], true)?;
		Ok(len)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		let mut output = self.output.lock().unwrap();
		output.flush()
	}
}

mod macros {
	#[macro_export]
	macro_rules! writable {
		($typ:ty) => {
			impl $crate::Writable for $typ {
				fn write(&self, f: &mut Writer) -> Result<()> {
					$crate::WriteFormat::write_std_fmt(self, f)
				}
			}
		};
	}

	#[macro_export]
	macro_rules! formatted {
		($typ:ty) => {
			impl std::fmt::Display for $typ {
				fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
					$crate::Writable::format(self, f)
				}
			}

			impl std::fmt::Debug for $typ {
				fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
					$crate::Writable::format_debug(self, f)
				}
			}
		};
	}
}

pub use macros::*;

pub trait WriteFormat: Display + Debug {
	/// Write helper to invoke the debug [`Debug`] or [`Display`] implementation.
	fn write_std_fmt(&self, f: &mut Writer) -> Result<()> {
		if f.is_debug() {
			self.write_debug(f)
		} else {
			self.write_display(f)
		}
	}
}

pub trait WriteDisplay: Display {
	fn write_display(&self, f: &mut Writer) -> Result<()> {
		write!(f, "{self}")?;
		Ok(())
	}
}

pub trait WriteDebug: Debug {
	fn write_debug(&self, f: &mut Writer) -> Result<()> {
		write!(f, "{self:?}")?;
		Ok(())
	}
}

impl<T: Display + Debug> WriteFormat for T {}
impl<T: Display> WriteDisplay for T {}
impl<T: Debug> WriteDebug for T {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_write() -> std::io::Result<()> {
		let mut out = String::new();
		{
			let mut w = Writer::fmt(&mut out);
			write!(w, "hello world!!!")?;
		}
		assert_eq!("hello world!!!", out);
		Ok(())
	}

	#[test]
	fn indented_write() -> std::io::Result<()> {
		// full write
		let mut out = String::new();
		{
			let mut w = Writer::fmt(&mut out).indented();
			write!(w, "Head:\nLine 1\nLine 2\n")?;
		}
		assert_eq!("Head:\n    Line 1\n    Line 2\n", out);

		// split write after new-line
		let mut out = String::new();
		{
			let mut w = Writer::fmt(&mut out);
			write!(w, "Head(\n")?;
			{
				let mut w = w.indented();
				write!(w, "Line 1\nLine 2\n")?;
			}
			write!(w, ")")?;
		}
		assert_eq!("Head(\n    Line 1\n    Line 2\n)", out);

		Ok(())
	}

	#[test]
	fn write_crlf() -> std::io::Result<()> {
		let mut out = String::new();
		{
			let mut w = Writer::fmt(&mut out).indented();
			write!(w, "Head:\r\nLine 1\r\nLine 2\r")?;
		}
		assert_eq!("Head:\n    Line 1\n    Line 2\n", out);

		let mut out = String::new();
		{
			let mut w = Writer::fmt(&mut out);
			write!(w, "Head(\r\n")?;
			{
				let mut w = w.indented();
				write!(w, "Line 1\rLine 2\r\n")?;
			}
			write!(w, ")")?;
		}
		assert_eq!("Head(\n    Line 1\n    Line 2\n)", out);

		Ok(())
	}

	#[test]
	fn basic_indent() {
		let input = "line 1\nline 2";
		let output = indent(input);
		assert_eq!("    line 1\n    line 2", output);
	}
}
