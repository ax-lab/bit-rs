use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cursor {
	src: Source,
	pos: usize,
	row: usize,
	col: usize,
	ind: usize,
	was_cr: bool,
}

impl Cursor {
	pub fn new(src: Source) -> Self {
		Self {
			src,
			row: 0,
			col: 0,
			ind: 0,
			pos: 0,
			was_cr: false,
		}
	}

	#[inline(always)]
	pub fn span(&self, len: usize) -> Span {
		Span::new(self.src, self.pos, self.pos + len)
	}

	#[inline(always)]
	pub fn text(&self) -> &'static str {
		&self.src.text()[self.pos..]
	}

	#[inline(always)]
	pub fn len(&self) -> usize {
		self.src.len() - self.pos
	}

	pub fn line(&self) -> usize {
		self.row + 1
	}

	pub fn column(&self) -> usize {
		self.col + 1
	}

	#[inline(always)]
	pub fn indent(&self) -> usize {
		self.ind
	}

	#[inline(always)]
	pub fn peek(&self) -> Option<char> {
		self.text().chars().next()
	}

	#[inline(always)]
	pub fn read(&mut self) -> Option<char> {
		if let Some(next) = self.peek() {
			self.advance(next);
			Some(next)
		} else {
			None
		}
	}

	#[inline(always)]
	pub fn skip_len(&mut self, bytes: usize) {
		let text = self.text();
		for chr in text[..bytes].chars() {
			self.advance(chr);
		}
	}

	pub fn text_context(&self) -> &'static str {
		const MAX_CHARS: usize = 10;
		let text = self.text();
		let index = text.find(|chr| is_space(chr) || chr == '\r' || chr == '\n');
		let index = index.unwrap_or(text.len());
		let text = &text[..index];
		let index = text
			.char_indices()
			.nth(MAX_CHARS + 1)
			.map(|(index, _)| index)
			.unwrap_or(text.len());
		&text[..index]
	}

	fn advance(&mut self, char: char) {
		let is_indent = self.ind == self.col && is_space(char);
		match char {
			'\t' => {
				let tab = self.src.tab_size();
				self.col += tab - (self.col % tab);
			}
			'\r' => {
				self.row += 1;
				self.col = 0;
				self.ind = 0;
			}
			'\n' => {
				if !self.was_cr {
					self.row += 1;
					self.col = 0;
					self.ind = 0;
				}
			}
			_ => {
				self.col += 1;
			}
		}
		self.pos += char.len_utf8();
		self.was_cr = char == '\r';
		if is_indent {
			self.ind = self.col;
		}
	}
}

impl Display for Cursor {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let src = self.src;
		let line = self.row + 1;
		let column = self.col + 1;
		write!(f, "{src}:{line}:{column}")
	}
}
