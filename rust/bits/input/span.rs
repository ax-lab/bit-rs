use super::*;

/// Spans reference a slice of text from a [`Source`].
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Span<'a> {
	sta: usize,
	end: usize,
	src: Source<'a>,
}

impl<'a> Span<'a> {
	pub(crate) fn new(pos: usize, len: usize, src: Source<'a>) -> Self {
		let sta = pos;
		let end = sta + len;
		assert!(end <= src.len());
		Self { sta, end, src }
	}

	pub fn empty() -> Self {
		Self::default()
	}

	pub fn src(&self) -> Source<'a> {
		self.src
	}

	pub fn pos(&self) -> usize {
		self.sta
	}

	pub fn len(&self) -> usize {
		self.end - self.sta
	}

	pub fn end(&self) -> usize {
		self.end
	}

	pub fn text(&self) -> &'a str {
		unsafe { self.src.text().get_unchecked(self.pos()..self.end()) }
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0 && self.pos() == 0 && self.src == Source::empty()
	}
}
