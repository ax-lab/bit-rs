use super::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Expr {
	None,
	Sequence(&'static [Code]),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Code {
	pub expr: Expr,
	pub span: Span,
}

impl Code {
	pub fn sequence<T: IntoIterator<Item = U>, U: Compilable>(code: T) -> Result<Code> {
		let code: Result<Vec<_>> = code.into_iter().map(|x| x.compile()).collect();
		let code = code?;
		let code = Arena::get().slice(code);
		let span = Span::for_range(code.iter());
		let code = Code {
			expr: Expr::Sequence(code),
			span,
		};
		Ok(code)
	}
}

impl HasSpan for Code {
	fn span(&self) -> Span {
		self.span
	}
}

pub trait Compilable {
	fn compile(&self) -> Result<Code>;
}

impl<T: Compilable> Compilable for Result<T> {
	fn compile(&self) -> Result<Code> {
		match self {
			Ok(x) => x.compile(),
			Err(err) => Err(err.clone()),
		}
	}
}

impl Compilable for Node {
	fn compile(&self) -> Result<Code> {
		let value = self.get_value().get();
		value.output_code(*self)
	}
}

impl From<Code> for Span {
	fn from(value: Code) -> Self {
		value.span
	}
}

impl From<&Code> for Span {
	fn from(value: &Code) -> Self {
		value.span
	}
}

impl<T: Compilable> Compilable for &T {
	fn compile(&self) -> Result<Code> {
		(*self).compile()
	}
}
