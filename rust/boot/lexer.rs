use std::{
	collections::hash_map::RandomState,
	hash::{BuildHasher, Hasher},
};

use super::*;

#[derive(Default)]
pub struct DefaultLexer {
	symbols: SymbolTable,
}

impl DefaultLexer {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_symbol<T: AsRef<str>>(&mut self, symbol: T) -> Symbol {
		self.symbols.add(symbol)
	}

	pub fn add_symbols<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, symbols: I) {
		for it in symbols.into_iter() {
			self.add_symbol(it.as_ref());
		}
	}

	pub fn tokenize(&mut self, cursor: &mut Cursor) -> Result<Vec<Token>> {
		let mut output = Vec::new();
		while cursor.len() > 0 {
			let text = cursor.text();

			let mut skip_spaces = text.len();
			for (pos, chr) in text.char_indices() {
				if !is_space(chr) {
					skip_spaces = pos;
					break;
				}
			}

			if skip_spaces > 0 {
				cursor.skip_len(skip_spaces);
				continue;
			}

			if cursor.len() == 0 {
				break;
			}

			let token = if let Some('\r' | '\n') = text.chars().next() {
				let len = if text.starts_with("\r\n") { 2 } else { 1 };
				Token::Break(cursor.span(len))
			} else if let Some(token) = self.match_next(cursor) {
				token
			} else {
				if let Some(symbol) = self.symbols.read(text) {
					Token::Symbol(symbol, cursor.span(symbol.len()))
				} else {
					let display = cursor.display_chars(5).text();
					let sep = if display.len() > 0 { " -- " } else { "" };
					return Err(err!("invalid token at {}{sep}{display}", cursor.span(0)));
				}
			};

			output.push(token);
			cursor.skip_len(token.span().len());
		}
		Ok(output)
	}

	fn match_next(&self, cursor: &Cursor) -> Option<Token> {
		let text = cursor.text();
		let next = text.chars().next().unwrap();
		let token = if next == '#' {
			let mut len = text.len();
			for (pos, chr) in text.char_indices() {
				if chr == '\n' || chr == '\r' {
					len = pos;
					break;
				}
			}
			Token::Comment(cursor.span(len))
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
			Token::Literal(cursor.span(len))
		} else if is_digit(next) {
			let len = count_digits(text);
			let (len, flt) = if text[len..].starts_with(".") {
				let pos = len + 1;
				let flt_len = count_digits(&text[pos..]);
				if flt_len > 0 {
					let flt_len = flt_len + count_digits(&text[pos + flt_len..]);
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
				let len = count_digits(rest);
				if len > 0 {
					(exp_len + len, true)
				} else {
					(len, flt)
				}
			} else {
				(len, flt)
			};
			let len = len + count_alpha_num(&text[len..]);
			let span = cursor.span(len);
			if flt {
				Token::Float(span)
			} else {
				Token::Integer(span)
			}
		} else {
			let mut word_len = 0;
			for (pos, char) in text.char_indices() {
				if !is_ident(char, pos > 0) {
					word_len = pos;
					break;
				} else {
					word_len = text.len();
				}
			}

			if word_len > 0 {
				let word = &text[..word_len];
				let word = Symbol::get(word);
				Token::Word(word, cursor.span(word_len))
			} else {
				return None;
			}
		};

		Some(token)
	}
}

const SYMBOL_SLOTS: usize = 1024;
const SYMBOL_MAX_LOAD: usize = SYMBOL_SLOTS / 16 * 10;

#[derive(Clone)]
pub struct SymbolTable {
	count: usize,
	max_len: usize,
	symbols: [SymbolCell; SYMBOL_SLOTS],
	state: RandomState,
}

static MAX_SYMBOL_QUERY: AtomicUsize = AtomicUsize::new(0);
static CNT_SYMBOL_QUERY: AtomicUsize = AtomicUsize::new(0);
static HIT_SYMBOL_QUERY: AtomicUsize = AtomicUsize::new(0);

impl SymbolTable {
	pub fn new() -> Self {
		const EMPTY: SymbolCell = SymbolCell::new();
		let symbols: [SymbolCell; SYMBOL_SLOTS] = [EMPTY; SYMBOL_SLOTS];
		Self {
			count: 0,
			max_len: 0,
			symbols,
			state: RandomState::new(),
		}
	}

	pub fn count(&self) -> usize {
		self.count
	}

	pub fn add<T: AsRef<str>>(&mut self, symbol: T) -> Symbol {
		let symbol = symbol.as_ref();

		self.max_len = self.max_len.max(symbol.len());

		debug_assert!(SYMBOL_SLOTS.is_power_of_two());
		debug_assert!(SYMBOL_MAX_LOAD < SYMBOL_SLOTS);
		if self.count >= SYMBOL_MAX_LOAD {
			panic!("too many lexer symbols (max {SYMBOL_MAX_LOAD})");
		}

		let hash = self.hash_symbol(symbol);
		let mut index = hash as usize;
		loop {
			index = Self::hash_next(hash, index);
			if let Some(slot) = self.symbols[index].get() {
				if slot.as_str() == symbol {
					return slot;
				}
			} else {
				let symbol = Symbol::get(symbol);
				if self.symbols[index].try_set(symbol) {
					self.count += 1;
					return symbol;
				}
			}
		}
	}

	pub fn query(&self, input: &str) -> Option<Symbol> {
		let hash = self.hash_symbol(input);
		let mut index = hash as usize;
		let mut cnt = 0;
		CNT_SYMBOL_QUERY.fetch_add(1, Order::Relaxed);
		loop {
			index = Self::hash_next(hash, index);

			cnt += 1;
			HIT_SYMBOL_QUERY.fetch_add(1, Order::Relaxed);

			let output = if let Some(symbol) = self.symbols[index].get() {
				if symbol.as_str() == input {
					Some(symbol)
				} else {
					continue;
				}
			} else {
				None
			};
			MAX_SYMBOL_QUERY.fetch_max(cnt, Order::Relaxed);
			return output;
		}
	}

	pub fn read(&self, text: &str) -> Option<Symbol> {
		let mut len = self.max_len.min(text.len());
		while len > 0 {
			if text.is_char_boundary(len) {
				if let Some(symbol) = self.query(&text[..len]) {
					return Some(symbol);
				}
			}
			len -= 1;
		}
		None
	}

	pub fn stat_max_symbol_query() -> usize {
		MAX_SYMBOL_QUERY.load(Order::Relaxed)
	}

	pub fn stat_cnt_symbol_query() -> usize {
		CNT_SYMBOL_QUERY.load(Order::Relaxed)
	}

	pub fn stat_hit_symbol_query() -> usize {
		HIT_SYMBOL_QUERY.load(Order::Relaxed)
	}

	const fn hash_next(hash: u64, index: usize) -> usize {
		const HASH_EXP: u32 = SYMBOL_SLOTS.ilog2();
		const HASH_MASK: usize = SYMBOL_SLOTS - 1;

		let step = (hash >> HASH_EXP) | 1;
		let index = index.wrapping_add(step as usize) & HASH_MASK;
		index
	}

	fn hash_symbol(&self, symbol: &str) -> u64 {
		let mut hasher = self.state.build_hasher();
		for char in symbol.chars().take(32) {
			char.hash(&mut hasher);
		}
		let hash = hasher.finish();
		hash
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
	fn symbol_table() {
		const SHOW_STATS: bool = false;

		let mut symbols = SymbolTable::default();

		assert_eq!(0, symbols.count());
		assert_eq!(None, symbols.query(""));
		assert_eq!(None, symbols.query("a"));
		assert_eq!(None, symbols.query("abc"));

		let a1 = symbols.add("a1");
		let a2 = symbols.add("a2");
		let a3 = symbols.add("a3");

		assert_eq!(3, symbols.count());
		assert_eq!("a1", a1.as_str());
		assert_eq!("a2", a2.as_str());
		assert_eq!("a3", a3.as_str());

		assert_eq!(Some(a1), symbols.query("a1"));
		assert_eq!(Some(a2), symbols.query("a2"));
		assert_eq!(Some(a3), symbols.query("a3"));

		assert_eq!(None, symbols.query(""));
		assert_eq!(None, symbols.query("a"));
		assert_eq!(None, symbols.query("a4"));

		assert_eq!(a1, symbols.add("a1"));
		assert_eq!(a2, symbols.add("a2"));
		assert_eq!(a3, symbols.add("a3"));

		assert_eq!(3, symbols.count());

		while symbols.count() < SYMBOL_MAX_LOAD {
			let next = symbols.count() + 1;
			let next = format!("a{next}");
			symbols.add(next);
		}

		for i in 0..SYMBOL_MAX_LOAD {
			let n = i + 1;
			let symbol = Symbol::get(format!("a{n}"));
			assert_eq!(Some(symbol), symbols.query(symbol.as_str()));
		}

		assert_eq!(None, symbols.query("none"));
		assert_eq!(None, symbols.query("some"));
		assert_eq!(None, symbols.query("dummy"));

		if SHOW_STATS {
			println!(
				"MAX = {} / AVG = {}",
				SymbolTable::stat_max_symbol_query(),
				SymbolTable::stat_hit_symbol_query() as f64 / SymbolTable::stat_cnt_symbol_query() as f64
			);
		}
	}

	#[test]
	fn empty() -> Result<()> {
		let input = source("");
		let result = tokenize(input)?;
		assert_eq!(0, result.len());
		Ok(())
	}

	#[test]
	fn line_break() -> Result<()> {
		let input = source("\n\r\r\n\n");
		let result = tokenize(input)?;
		assert_eq!(result, ["eol", "eol", "eol", "eol"]);
		Ok(())
	}
	#[test]
	fn symbols() -> Result<()> {
		let input = source("+++-+\n<<<<< <\n,,\n");
		let result = tokenize(input)?;

		assert_eq!(
			vec![
				"symbol(++)",
				"symbol(+)",
				"symbol(-)",
				"symbol(+)",
				"eol",
				"symbol(<<<)",
				"symbol(<<)",
				"symbol(<)",
				"eol",
				"symbol(,)",
				"symbol(,)",
				"eol",
			],
			result
		);

		Ok(())
	}

	#[test]
	fn words() -> Result<()> {
		let input = source("a ab abc a1 a2 _ __ _a _0 abc_123");
		let result = tokenize(input)?;

		assert_eq!(
			vec![
				"word(a)",
				"word(ab)",
				"word(abc)",
				"word(a1)",
				"word(a2)",
				"word(_)",
				"word(__)",
				"word(_a)",
				"word(_0)",
				"word(abc_123)",
			],
			result,
		);

		Ok(())
	}

	#[test]
	fn numbers() -> Result<()> {
		let input = source(
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
		let result = tokenize(input)?;

		assert_eq!(
			vec![
				"int(0)",
				"int(123)",
				"eol",
				"float(1.2)",
				"float(3.45)",
				"float(10e1)",
				"float(10E20)",
				"eol",
				"float(1e+23)",
				"float(1E-23)",
				"float(1.45e2)",
				"float(1.23E-45)",
				"eol",
				"float(1_000_000_.56_78_e+1_2_3_)",
				"eol",
				"int(1abc)",
				"float(1.0abc)",
				"float(1e1abc)",
				"float(1.0e+1abc)",
				"int(1eee)",
				"eol",
				"int(1)",
				"symbol(.)",
				"word(abc)",
			],
			result
		);

		Ok(())
	}

	#[test]
	fn comments() -> Result<()> {
		let input = source(["# simple comment", "1# C1\r2# C2", "3# C3\r\n4# C4", "#"].join("\n"));
		let result = tokenize(input)?;

		assert_eq!(
			vec![
				"comment(# simple comment)",
				"eol",
				"int(1)",
				"comment(# C1)",
				"eol",
				"int(2)",
				"comment(# C2)",
				"eol",
				"int(3)",
				"comment(# C3)",
				"eol",
				"int(4)",
				"comment(# C4)",
				"eol",
				"comment(#)",
			],
			result,
		);

		Ok(())
	}

	#[test]
	fn strings() -> Result<()> {
		let input = source(
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

		let result = tokenize(input)?;
		assert_eq!(
			vec![
				r#"literal('')"#,
				r#"literal('hello world')"#,
				"eol",
				r#"literal('a')"#,
				r#"literal('b')"#,
				r#"literal('c')"#,
				"eol",
				r#"literal('abc\'def')"#,
				"eol",
				r#"literal('\\\'')"#,
				"eol",
				r#"literal("")"#,
				r#"literal("hello world")"#,
				"eol",
				r#"literal("a")"#,
				r#"literal("b")"#,
				r#"literal("c")"#,
				"eol",
				r#"literal("abc\"def")"#,
				"eol",
				r#"literal("\\\"")"#,
			],
			result
		);

		Ok(())
	}

	fn tokenize(src: Source) -> Result<Vec<&'static str>> {
		let mut lexer = DefaultLexer::new();
		lexer.add_symbols(["+", "++", "-", "--", "<", "<<", "<<<", "=", "==", ",", "."]);

		let mut cursor = Cursor::new(src);
		let tokens = lexer.tokenize(&mut cursor)?;

		assert!(cursor.len() == 0, "failed to parse: {:?}", cursor.text());

		let mut out = Vec::new();
		for it in tokens {
			let span = it.span().text();
			let text = match it {
				Token::Break(_) => format!("eol"),
				Token::Symbol(s, _) => format!("symbol({})", s.as_str()),
				Token::Word(w, _) => format!("word({})", w.as_str()),
				Token::Integer(_) => format!("int({span})"),
				Token::Float(_) => format!("float({span})"),
				Token::Literal(_) => format!("literal({span})"),
				Token::Comment(_) => format!("comment({span})"),
			};
			out.push(Box::leak(Box::new(text)).as_str());
		}
		Ok(out)
	}

	fn source<T: AsRef<str>>(text: T) -> Source {
		static SOURCES: Init<SourceMap> = Init::new(|| SourceMap::new(".").unwrap());
		let sources = SOURCES.get();
		sources.from_string("test", text)
	}
}
