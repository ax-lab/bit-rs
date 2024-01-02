use super::*;

/// Spans reference a slice of text from a [`Source`].
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Span<'a> {
	sta: usize,
	end: usize,
	src: Source<'a>,
}

impl<'a> Span<'a> {
	pub(crate) fn new(sta: usize, end: usize, src: Source<'a>) -> Self {
		assert!(sta <= end && end <= src.len());
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

impl<'a> Ord for Span<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.src
			.cmp(&other.src)
			.then_with(|| self.sta.cmp(&other.sta))
			.then_with(|| self.end.cmp(&other.end))
	}
}

impl<'a> PartialOrd for Span<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> Display for Span<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut cursor = Cursor::new(self.src());
		let len = self.len();
		cursor.skip_len(self.sta);
		write!(f, "{cursor}")?;
		if len > 0 {
			write!(f, "+{len}")?;
		}
		Ok(())
	}
}

impl<'a> Debug for Span<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let src = self.src();
		let pos = self.pos();
		let len = self.len();
		write!(f, "{src}:{pos}")?;
		if len > 0 {
			write!(f, "+{len}")?;
		}
		Ok(())
	}
}
