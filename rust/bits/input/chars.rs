#[inline]
pub fn is_space(char: char) -> bool {
	match char {
		'\u{0020}' => true, // Space
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
