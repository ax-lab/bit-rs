use super::*;

pub trait HasSpan {
	fn span(&self) -> Span;
}

impl<T: HasSpan> HasSpan for &T {
	fn span(&self) -> Span {
		T::span(*self)
	}
}

impl<T: HasSpan> HasSpan for &mut T {
	fn span(&self) -> Span {
		T::span(*self)
	}
}

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

	pub fn range<T: IntoIterator<Item = U>, U: HasSpan>(elems: T) -> Span {
		let mut iter = elems.into_iter();
		if let Some(first) = iter.next() {
			let first = first.span();
			if let Some(last) = iter.last() {
				let last = last.span();
				Self::merge(first, last)
			} else {
				first
			}
		} else {
			Span::empty()
		}
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

	pub fn display_text(&self, max_len: usize) -> Option<Cow<'static, str>> {
		let max_len = if max_len == 0 { 30 } else { max_len };

		let text = self.text();
		let text = if let Some(index) = text.find(|chr| chr == '\r' || chr == '\n') {
			&text[..index]
		} else {
			text
		};
		let text = text.trim_end();
		let suffix = if text.len() < self.len() { "…" } else { "" };
		let (prefix, text) = {
			let trimmed = text.trim_start();
			let prefix = if trimmed.len() < text.len() { "…" } else { "" };
			(prefix, trimmed)
		};

		let (text, suffix) = if let Some((n, _)) = text.char_indices().nth(max_len) {
			(&text[..n], "…")
		} else {
			(text, suffix)
		};

		if text.len() > 0 {
			let text = if suffix.len() + prefix.len() > 0 {
				let text = format!("{prefix}{text}{suffix}");
				Cow::Owned(text)
			} else {
				Cow::Borrowed(text)
			};
			Some(text)
		} else {
			None
		}
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
		write!(f, "{src}[{sta}")?;
		if len > 0 {
			write!(f, "+{len}")?;
		}
		write!(f, "]")?;
		Ok(())
	}
}

impl<'a> Writer<'a> {
	pub fn write_location<T: AsRef<str>>(&mut self, label: T, span: Span) -> Result<()> {
		if !span.is_empty() {
			let label = label.as_ref();
			write!(self, "{label}{span}")?;
		}
		Ok(())
	}
}
