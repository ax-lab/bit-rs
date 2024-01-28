#[inline(always)]
pub fn is_space(char: char) -> bool {
	match char {
		'\t' => true,       // Tabs
		' ' => true,        // Space
		'\u{00A0}' => true, // No-Break Space
		'\u{1680}' => true, // Ogham Space Mark
		'\u{2000}' => true, // En Quad
		'\u{2001}' => true, // Em Quad
		'\u{2002}' => true, // En Space
		'\u{2003}' => true, // Em Space
		'\u{2004}' => true, // Three-Per-Em Space
		'\u{2005}' => true, // Four-Per-Em Space
		'\u{2006}' => true, // Six-Per-Em Space
		'\u{2007}' => true, // Figure Space
		'\u{2008}' => true, // Punctuation Space
		'\u{2009}' => true, // Thin Space
		'\u{200A}' => true, // Hair Space
		'\u{202F}' => true, // Narrow No-Break Space
		'\u{205F}' => true, // Medium Mathematical Space
		'\u{3000}' => true, // Ideographic Space
		_ => false,
	}
}

#[inline(always)]
pub fn is_ident(c: char, mid: bool) -> bool {
	match c {
		'a'..='z' => true,
		'A'..='Z' => true,
		'_' => true,
		'0'..='9' => mid,
		_ => false,
	}
}

#[inline(always)]
pub fn is_digit(c: char) -> bool {
	c >= '0' && c <= '9'
}

#[inline(always)]
pub fn count_alpha_num(text: &str) -> usize {
	for (pos, char) in text.char_indices() {
		if !is_ident(char, true) {
			return pos;
		}
	}
	text.len()
}

#[inline(always)]
pub fn count_digits(text: &str) -> usize {
	for (pos, char) in text.char_indices() {
		if !is_digit(char) && char != '_' {
			return pos;
		}
	}
	text.len()
}
