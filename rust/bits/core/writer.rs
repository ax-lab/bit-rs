use super::*;

const DEFAULT_INDENT: &'static str = "    ";
const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

pub trait Writable {
	fn write(&self, f: &mut Writer) -> Result<()>;

	fn write_fmt(&self, f: &mut Writer) -> Result<()>
	where
		Self: Display + Debug,
	{
		if f.is_debug() {
			self.fmt_debug(f)
		} else {
			self.fmt_display(f)
		}
	}

	fn fmt_display(&self, f: &mut Writer) -> Result<()>
	where
		Self: Display,
	{
		write!(f, "{self}")?;
		Ok(())
	}

	fn fmt_debug(&self, f: &mut Writer) -> Result<()>
	where
		Self: Debug,
	{
		write!(f, "{self:?}")?;
		Ok(())
	}

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
		self.new_line.load(SyncOrder::Relaxed)
	}

	pub fn was_cr(&self) -> bool {
		self.was_cr.load(SyncOrder::Relaxed)
	}

	pub fn set_new_line(&self, value: bool) {
		self.new_line.store(value, SyncOrder::Relaxed)
	}

	pub fn set_was_cr(&self, value: bool) {
		self.was_cr.store(value, SyncOrder::Relaxed)
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
		self.state.written.load(SyncOrder::Relaxed)
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
		self.indent_with(DEFAULT_INDENT)
	}

	pub fn dedent(&mut self) {
		self.dedent_with(DEFAULT_INDENT)
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
		self.indented_with(DEFAULT_INDENT)
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
					self.state.written.fetch_add(indent.len(), SyncOrder::Relaxed);
				}
				self.state.set_new_line(last == CR || last == LF);
				self.state.set_was_cr(last == CR);
				let len = output.write(bytes)?;
				self.state.written.fetch_add(bytes.len(), SyncOrder::Relaxed);
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

struct FormatWriter<'a> {
	output: &'a mut dyn std::fmt::Write,
}

impl<'a> Write for FormatWriter<'a> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let buf = std::str::from_utf8(buf).expect("invalid UTF-8 for string writer");
		match self.output.write_str(buf) {
			Ok(_) => Ok(buf.len()),
			Err(err) => Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
		}
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn basic_write() -> std::io::Result<()> {
		let mut out = String::new();
		{
			let mut w = Writer::fmt(&mut out);
			write!(w, "hello world!!!")?;
		}
		assert_eq!("hello world!!!", out);
		Ok(())
	}

	#[test]
	pub fn indented_write() -> std::io::Result<()> {
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
	pub fn write_crlf() -> std::io::Result<()> {
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
}
