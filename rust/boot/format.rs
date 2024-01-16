use super::*;

pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;
pub const GB: usize = 1024 * MB;
pub const TB: usize = 1024 * GB;

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
	fn write(&mut self) -> FormatWriter {
		FormatWriter::new(self)
	}
}

impl<T: std::fmt::Write + Sized> ToFormatWriter for T {}

pub fn to_bytes(bytes: usize) -> String {
	let mut output = String::new();
	{
		let mut output = output.write();
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
