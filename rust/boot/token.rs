use super::*;

#[derive(Copy, Clone, Debug)]
pub struct TokenList(&'static [Token]);

impl TokenList {
	pub fn new<T: IntoIterator<Item = Token>>(tokens: T) -> Self
	where
		T::IntoIter: ExactSizeIterator,
	{
		let tokens = Arena::get().slice(tokens);
		Self(tokens)
	}

	#[inline(always)]
	pub fn len(&self) -> usize {
		self.list().len()
	}

	#[inline(always)]
	pub fn list(&self) -> &'static [Token] {
		self.0
	}

	pub fn range<T: RangeBounds<usize>>(&self, range: T) -> Self {
		let sta = match range.start_bound() {
			std::ops::Bound::Included(&n) => n,
			std::ops::Bound::Excluded(&n) => n + 1,
			std::ops::Bound::Unbounded => 0,
		};
		let end = match range.end_bound() {
			std::ops::Bound::Included(&n) => n + 1,
			std::ops::Bound::Excluded(&n) => n,
			std::ops::Bound::Unbounded => self.len(),
		};
		Self(&self.0[sta..end])
	}
}

impl std::ops::Index<usize> for TokenList {
	type Output = Token;

	#[inline(always)]
	fn index(&self, index: usize) -> &Self::Output {
		self.list().index(index)
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Token {
	Break(Span),
	Symbol(Symbol, Span),
	Word(Symbol, Span),
	Integer(Span),
	Float(Span),
	Literal(Span),
	Comment(Span),
}

impl Token {
	pub fn symbol(&self) -> Symbol {
		match self {
			Token::Symbol(s, ..) => *s,
			Token::Word(s, ..) => *s,
			Token::Break(..) => Symbol::empty(),
			Token::Integer(..) => Symbol::empty(),
			Token::Float(..) => Symbol::empty(),
			Token::Literal(..) => Symbol::empty(),
			Token::Comment(..) => Symbol::empty(),
		}
	}

	pub fn span(&self) -> Span {
		match self {
			Token::Break(span) => *span,
			Token::Symbol(.., span) => *span,
			Token::Word(.., span) => *span,
			Token::Integer(span) => *span,
			Token::Float(span) => *span,
			Token::Literal(span) => *span,
			Token::Comment(span) => *span,
		}
	}
}

impl Display for Token {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Token::Break(..) => write!(f, "break"),
			Token::Symbol(s, ..) => {
				write!(f, "symbol(")?;
				s.write_name(f)?;
				write!(f, ")")
			}
			Token::Word(s, ..) => {
				write!(f, "word(")?;
				s.write_name(f)?;
				write!(f, ")")
			}
			Token::Integer(..) => write!(f, "int"),
			Token::Float(..) => write!(f, "float"),
			Token::Literal(..) => write!(f, "literal"),
			Token::Comment(..) => write!(f, "comment"),
		}
	}
}

#[allow(non_snake_case)]
pub struct Symbols {
	pub COMMA: Symbol,
	pub DOT: Symbol,
	pub SEMICOLON: Symbol,
	pub COLON: Symbol,
	pub STA_PAREN: Symbol,
	pub END_PAREN: Symbol,
	pub STA_BRACE: Symbol,
	pub END_BRACE: Symbol,
	pub STA_BRACKET: Symbol,
	pub END_BRACKET: Symbol,
}

impl Symbols {
	pub fn get() -> &'static Self {
		static SYMBOLS: Init<Symbols> = Init::new(|| Symbols {
			COMMA: ",".into(),
			DOT: ".".into(),
			SEMICOLON: ";".into(),
			COLON: ":".into(),
			STA_PAREN: "(".into(),
			END_PAREN: ")".into(),
			STA_BRACE: "{".into(),
			END_BRACE: "}".into(),
			STA_BRACKET: "[".into(),
			END_BRACKET: "]".into(),
		});
		SYMBOLS.get()
	}
}
