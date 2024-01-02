use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cursor<'a> {
	src: Source<'a>,
	offset: usize,
	line: usize,
	column: usize,
	indent: usize,
	was_cr: bool,
}

impl<'a> Cursor<'a> {
	pub fn new(src: Source<'a>) -> Self {
		Self {
			line: 0,
			column: 0,
			indent: 0,
			offset: 0,
			was_cr: false,
			src,
		}
	}

	pub fn line(&self) -> usize {
		self.line
	}

	pub fn column(&self) -> usize {
		self.column
	}

	pub fn indent(&self) -> usize {
		self.indent
	}

	pub fn peek(&self) -> Option<char> {
		self.text().chars().next()
	}

	#[inline]
	pub fn text(&self) -> &'a str {
		&self.src.text()[self.offset..]
	}

	pub fn read(&mut self) -> Option<char> {
		if let Some(next) = self.peek() {
			self.advance(next);
			Some(next)
		} else {
			None
		}
	}

	pub fn skip_len(&mut self, bytes: usize) {
		let text = self.text();
		for chr in text[..bytes].chars() {
			self.advance(chr);
		}
	}

	fn advance(&mut self, char: char) {
		let is_indent = self.indent == self.column && is_space(char);
		match char {
			'\t' => {
				let tab = self.src.tab_size();
				self.column += tab - (self.column % tab);
			}
			'\r' => {
				self.line += 1;
				self.column = 0;
				self.indent = 0;
			}
			'\n' => {
				if !self.was_cr {
					self.line += 1;
					self.column = 0;
					self.indent = 0;
				}
			}
			_ => {
				self.column += 1;
			}
		}
		self.offset += char.len_utf8();
		self.was_cr = char == '\r';
		if is_indent {
			self.indent = self.column;
		}
	}
}

impl<'a> Display for Cursor<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let src = self.src;
		let line = self.line + 1;
		let column = self.column + 1;
		write!(f, "{src}:{line}:{column}")
	}
}
