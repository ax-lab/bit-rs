use super::*;

pub trait Tokenizer: Clone + Default {
	fn tokenize<'a>(&mut self, cursor: &mut Cursor<'a>) -> Vec<(Token, Span<'a>)>;
}

pub trait Grammar: Clone + Default {
	fn is_space(c: char) -> bool;

	fn match_next(&self, text: &str) -> Option<(Token, usize)>;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Token {
	None,
	Break,
	Symbol(Symbol),
	Word(Symbol),
	Integer,
	Float,
	Literal,
	Comment,
}

#[derive(Clone, Default)]
pub struct DefaultGrammar;

impl DefaultGrammar {
	fn id(c: char, mid: bool) -> bool {
		match c {
			'a'..='z' => true,
			'A'..='Z' => true,
			'_' => true,
			'0'..='9' => mid,
			_ => false,
		}
	}

	fn is_digit(c: char) -> bool {
		c >= '0' && c <= '9'
	}

	fn alpha_num(text: &str) -> usize {
		for (pos, char) in text.char_indices() {
			if !Self::id(char, true) {
				return pos;
			}
		}
		text.len()
	}

	fn digits(text: &str) -> usize {
		for (pos, char) in text.char_indices() {
			if !Self::is_digit(char) && char != '_' {
				return pos;
			}
		}
		text.len()
	}
}

impl Grammar for DefaultGrammar {
	fn is_space(c: char) -> bool {
		is_space(c)
	}

	fn match_next(&self, text: &str) -> Option<(Token, usize)> {
		let next = text.chars().next().unwrap();
		if next == '#' {
			let mut len = text.len();
			for (pos, chr) in text.char_indices() {
				if chr == '\n' || chr == '\r' {
					len = pos;
					break;
				}
			}
			Some((Token::Comment, len))
		} else if next == '\'' || next == '"' {
			let quote = next;
			let can_escape = true;
			let mut escape = false;
			let mut len = text.len();
			for (pos, chr) in text.char_indices() {
				if chr == quote && pos > 0 && !escape {
					len = pos + chr.len_utf8();
					break;
				}
				if escape {
					escape = false;
				} else if can_escape && chr == '\\' {
					escape = true;
				}
			}
			Some((Token::Literal, len))
		} else if Self::is_digit(next) {
			let len = Self::digits(text);
			let (len, flt) = if text[len..].starts_with(".") {
				let pos = len + 1;
				let flt_len = Self::digits(&text[pos..]);
				if flt_len > 0 {
					let flt_len = flt_len + Self::digits(&text[pos + flt_len..]);
					(pos + flt_len, true)
				} else {
					(len, false)
				}
			} else {
				(len, false)
			};
			let rest = &text[len..];
			let (len, flt) = if let Some('e' | 'E') = rest.chars().next() {
				let (exp_len, rest) = (len + 1, &rest[1..]);
				let (exp_len, rest) = if let Some('+' | '-') = rest.chars().next() {
					(exp_len + 1, &rest[1..])
				} else {
					(exp_len, rest)
				};
				let len = Self::digits(rest);
				if len > 0 {
					(exp_len + len, true)
				} else {
					(len, flt)
				}
			} else {
				(len, flt)
			};
			let len = len + Self::alpha_num(&text[len..]);
			let kind = if flt { Token::Float } else { Token::Integer };
			Some((kind, len))
		} else {
			let mut word_len = 0;
			for (pos, char) in text.char_indices() {
				if !Self::id(char, pos > 0) {
					word_len = pos;
					break;
				} else {
					word_len = text.len();
				}
			}

			if word_len > 0 {
				let word = &text[..word_len];
				let word = Symbol::str(word);
				Some((Token::Word(word), word_len))
			} else {
				None
			}
		}
	}
}

#[derive(Clone, Default)]
pub struct Lexer<T: Grammar> {
	symbols: SymbolTable,
	grammar: T,
}

impl<T: Grammar> Lexer<T> {
	pub fn new(grammar: T) -> Self {
		Self {
			symbols: Default::default(),
			grammar,
		}
	}

	pub fn add_symbols<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, symbols: I) {
		for it in symbols.into_iter() {
			self.add_symbol(it.as_ref());
		}
	}

	pub fn add_symbol<S: AsRef<str>>(&mut self, symbol: S) {
		self.symbols.add_symbol(symbol.as_ref());
	}

	pub fn tokenize<'a>(&mut self, cursor: &mut Cursor<'a>) -> Vec<(Token, Span<'a>)> {
		let mut output = Vec::new();
		while cursor.len() > 0 {
			let text = cursor.text();

			let mut skip_spaces = text.len();
			for (pos, chr) in text.char_indices() {
				if !T::is_space(chr) {
					skip_spaces = pos;
					break;
				}
			}

			if skip_spaces > 0 {
				cursor.skip_len(skip_spaces);
				continue;
			}

			let (token, len) = if let Some('\r' | '\n') = text.chars().next() {
				let len = if text.starts_with("\r\n") { 2 } else { 1 };
				(Token::Break, len)
			} else if let Some((token, len)) = self.grammar.match_next(text) {
				(token, len)
			} else if let Some(symbol) = self.symbols.read(text) {
				let symbol = Symbol::str(symbol);
				(Token::Symbol(symbol), symbol.len())
			} else {
				break; // stop at the first unrecognized token
			};

			output.push((token, cursor.span_with_len(len)));
			cursor.skip_len(len);
		}
		output
	}
}

impl<T: Grammar> Tokenizer for Lexer<T> {
	fn tokenize<'a>(&mut self, cursor: &mut Cursor<'a>) -> Vec<(Token, Span<'a>)> {
		Lexer::tokenize(self, cursor)
	}
}

const SYMBOL_SLOTS: usize = 257;

#[derive(Clone)]
pub struct SymbolTable {
	symbols: [Box<Vec<Box<str>>>; SYMBOL_SLOTS],
}

impl SymbolTable {
	pub fn new() -> Self {
		let mut symbols: [MaybeUninit<Box<Vec<Box<str>>>>; SYMBOL_SLOTS] =
			unsafe { MaybeUninit::uninit().assume_init() };
		for it in symbols.iter_mut() {
			it.write(Default::default());
		}
		Self {
			symbols: unsafe { std::mem::transmute(symbols) },
		}
	}

	pub fn add_symbol(&mut self, symbol: &str) {
		let char = symbol.chars().next().unwrap();
		let index = (char as usize) % self.symbols.len();
		let symbols = &mut self.symbols[index];

		if symbols.iter().any(|x| x.as_ref() == symbol) {
			return;
		}

		symbols.push(symbol.into());
		symbols.sort_by(|a, b| b.len().cmp(&a.len()));
	}

	pub fn read<'a>(&self, input: &'a str) -> Option<&'a str> {
		if let Some(char) = input.chars().next() {
			let index = (char as usize) % self.symbols.len();
			let symbols = &self.symbols[index];
			for it in symbols.iter() {
				if input.starts_with(it.as_ref()) {
					return Some(&input[..it.len()]);
				}
			}
		}
		None
	}
}

impl Default for SymbolTable {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string("test", "").span();
		let result = tokenize(input);
		assert_eq!(result, []);

		let input = sources.from_string("test", "\t\t  ").span();
		let result = tokenize(input);
		assert_eq!(result, []);
	}

	#[test]
	fn line_break() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string("test", "\n\r\r\n\n").span();
		let result = tokenize(input);
		assert_eq!(result, [Token::Break, Token::Break, Token::Break, Token::Break]);
	}

	#[test]
	fn symbols() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string("test", "+++-+\n<<<<< <\n,,\n").span();
		let result = tokenize(input);

		let inc = Symbol::str("++");
		let add = Symbol::str("+");
		let sub = Symbol::str("-");
		let sl3 = Symbol::str("<<<");
		let sl2 = Symbol::str("<<");
		let sl1 = Symbol::str("<");
		let comma = Symbol::str(",");

		assert_eq!(
			result,
			[
				Token::Symbol(inc),
				Token::Symbol(add),
				Token::Symbol(sub),
				Token::Symbol(add),
				Token::Break,
				Token::Symbol(sl3),
				Token::Symbol(sl2),
				Token::Symbol(sl1),
				Token::Break,
				Token::Symbol(comma),
				Token::Symbol(comma),
				Token::Break,
			]
		)
	}

	#[test]
	fn words() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string("test", "a ab abc a1 a2 _ __ _a _0 abc_123");
		let result = tokenize(input.span());

		let s_a = Symbol::str("a");
		let s_ab = Symbol::str("ab");
		let s_abc = Symbol::str("abc");
		let s_a1 = Symbol::str("a1");
		let s_a2 = Symbol::str("a2");
		let s_u = Symbol::str("_");
		let s_uu = Symbol::str("__");
		let s_u_a = Symbol::str("_a");
		let s_u_0 = Symbol::str("_0");
		let s_abc_123 = Symbol::str("abc_123");

		assert_eq!(
			result,
			[
				Token::Word(s_a),
				Token::Word(s_ab),
				Token::Word(s_abc),
				Token::Word(s_a1),
				Token::Word(s_a2),
				Token::Word(s_u),
				Token::Word(s_uu),
				Token::Word(s_u_a),
				Token::Word(s_u_0),
				Token::Word(s_abc_123),
			]
		)
	}

	#[test]
	fn numbers() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string(
			"test",
			[
				"0 123",
				"1.2 3.45 10e1 10E20",
				"1e+23 1E-23 1.45e2 1.23E-45",
				"1_000_000_.56_78_e+1_2_3_",
				"1abc 1.0abc 1e1abc 1.0e+1abc 1eee",
				"1.abc",
			]
			.join("\n"),
		);
		let result = tokenize_str(input.span());

		let dot = Symbol::str(".");
		let abc = Symbol::str("abc");

		assert_eq!(
			result,
			[
				(Token::Integer, "0"),
				(Token::Integer, "123"),
				(Token::Break, "\n"),
				(Token::Float, "1.2"),
				(Token::Float, "3.45"),
				(Token::Float, "10e1"),
				(Token::Float, "10E20"),
				(Token::Break, "\n"),
				(Token::Float, "1e+23"),
				(Token::Float, "1E-23"),
				(Token::Float, "1.45e2"),
				(Token::Float, "1.23E-45"),
				(Token::Break, "\n"),
				(Token::Float, "1_000_000_.56_78_e+1_2_3_"),
				(Token::Break, "\n"),
				(Token::Integer, "1abc"),
				(Token::Float, "1.0abc"),
				(Token::Float, "1e1abc"),
				(Token::Float, "1.0e+1abc"),
				(Token::Integer, "1eee"),
				(Token::Break, "\n"),
				(Token::Integer, "1"),
				(Token::Symbol(dot), "."),
				(Token::Word(abc), "abc"),
			]
		)
	}

	#[test]
	fn comments() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string(
			"test",
			["# simple comment", "1# C1\r2# C2", "3# C3\r\n4# C4", "#"].join("\n"),
		);
		let result = tokenize_str(input.span());

		assert_eq!(
			result,
			[
				(Token::Comment, "# simple comment"),
				(Token::Break, "\n"),
				(Token::Integer, "1"),
				(Token::Comment, "# C1"),
				(Token::Break, "\r"),
				(Token::Integer, "2"),
				(Token::Comment, "# C2"),
				(Token::Break, "\n"),
				(Token::Integer, "3"),
				(Token::Comment, "# C3"),
				(Token::Break, "\r\n"),
				(Token::Integer, "4"),
				(Token::Comment, "# C4"),
				(Token::Break, "\n"),
				(Token::Comment, "#"),
			]
		)
	}

	#[test]
	fn strings() {
		let ctx = Context::new();
		let ctx = ctx.get();
		let sources = ctx.sources();

		let input = sources.from_string(
			"test",
			[
				r#"'' 'hello world'"#,
				r#"'a''b''c'"#,
				r#"'abc\'def'"#,
				r#"'\\\''"#,
				r#""" "hello world""#,
				r#""a""b""c""#,
				r#""abc\"def""#,
				r#""\\\"""#,
			]
			.join("\n"),
		);

		let result = tokenize_str(input.span());
		assert_eq!(
			result,
			[
				(Token::Literal, r#"''"#),
				(Token::Literal, r#"'hello world'"#),
				(Token::Break, "\n"),
				(Token::Literal, r#"'a'"#),
				(Token::Literal, r#"'b'"#),
				(Token::Literal, r#"'c'"#),
				(Token::Break, "\n"),
				(Token::Literal, r#"'abc\'def'"#),
				(Token::Break, "\n"),
				(Token::Literal, r#"'\\\''"#),
				(Token::Break, "\n"),
				(Token::Literal, r#""""#),
				(Token::Literal, r#""hello world""#),
				(Token::Break, "\n"),
				(Token::Literal, r#""a""#),
				(Token::Literal, r#""b""#),
				(Token::Literal, r#""c""#),
				(Token::Break, "\n"),
				(Token::Literal, r#""abc\"def""#),
				(Token::Break, "\n"),
				(Token::Literal, r#""\\\"""#),
			]
		)
	}

	fn tokenize<'a>(span: Span<'a>) -> Vec<Token> {
		tokenize_str(span).into_iter().map(|x| x.0).collect()
	}

	fn tokenize_str<'a>(span: Span<'a>) -> Vec<(Token, &'a str)> {
		let mut lexer = Lexer::new(DefaultGrammar);
		lexer.add_symbols(["+", "++", "-", "--", "<", "<<", "<<<", "=", "==", ",", "."]);

		let mut cursor = Cursor::new(span.src());
		let out = lexer.tokenize(&mut cursor);
		assert!(cursor.len() == 0, "failed to parse: {:?}", cursor.text());
		let out = out.into_iter().map(|(token, span)| (token, span.text()));
		out.collect()
	}
}
