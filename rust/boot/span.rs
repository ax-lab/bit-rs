use super::*;

#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Span {
	source: Source,
	sta: usize,
	end: usize,
}

impl Span {
	pub fn empty() -> Self {
		Self::default()
	}

	#[inline(always)]
	pub(crate) fn new(src: Source, sta: usize, end: usize) -> Self {
		Self { source: src, sta, end }
	}

	#[inline(always)]
	pub fn source(&self) -> Source {
		self.source
	}

	#[inline(always)]
	pub fn sta(&self) -> usize {
		self.sta
	}

	#[inline(always)]
	pub fn end(&self) -> usize {
		self.end
	}

	#[inline(always)]
	pub fn len(&self) -> usize {
		self.end - self.sta
	}

	#[inline(always)]
	pub fn text(&self) -> &'static str {
		let text = self.source().text();
		&text[self.sta..self.end]
	}

	#[inline(always)]
	pub fn is_empty(&self) -> bool {
		self.sta == 0 && self.end == 0 && self.source == Source::empty()
	}

	#[inline(always)]
	pub fn truncated(self, len: usize) -> Self {
		assert!(len < self.len());
		Span::new(self.source, self.sta, self.sta + len)
	}

	pub fn merge(a: Self, b: Self) -> Self {
		if a.is_empty() {
			return b;
		}
		if b.is_empty() {
			return a;
		}

		assert_eq!(a.source, b.source);
		let (a, b) = if a.sta <= b.sta { (a, b) } else { (b, a) };
		Self {
			source: a.source,
			sta: a.sta,
			end: std::cmp::max(a.end, b.end),
		}
	}

	pub fn merged(self, other: Self) -> Self {
		Self::merge(self, other)
	}

	pub fn location(&self) -> Cursor {
		let mut cursor = Cursor::new(self.source());
		cursor.skip_len(self.sta);
		cursor
	}
}

impl Display for Span {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let loc = self.location();
		let len = self.len();
		write!(f, "{loc}")?;
		if len > 0 {
			write!(f, "+{len}")?;
		}
		Ok(())
	}
}

impl Debug for Span {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let src = self.source();
		let sta = self.sta();
		let len = self.len();
		write!(f, "{src}:{sta}")?;
		if len > 0 {
			write!(f, "+{len}")?;
		}
		Ok(())
	}
}
