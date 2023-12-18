use super::*;

pub struct Writer<'a> {
	output: &'a mut dyn Write,
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
		self.output.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.output.flush()
	}
}
