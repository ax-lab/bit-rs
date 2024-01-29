use super::*;

#[derive(Copy, Clone)]
pub struct CodeContext {
	data: &'static CodeContextData,
}

impl CodeContext {
	pub fn new() -> Self {
		let data = Arena::get().store(CodeContextData::default());
		Self { data }
	}

	pub fn new_child(&self, span: Span) -> CodeContext {
		let data = Arena::get().store(CodeContextData {
			span,
			root: Some(self.root()),
			parent: Some(*self),
			..Default::default()
		});
		Self { data }
	}

	pub fn root(&self) -> CodeContext {
		self.data.root.unwrap_or_else(|| *self)
	}

	pub fn parent(&self) -> Option<CodeContext> {
		self.data.parent
	}
}

#[derive(Default)]
struct CodeContextData {
	span: Span,
	root: Option<CodeContext>,
	parent: Option<CodeContext>,
}

#[derive(Copy, Clone, Debug)]
pub enum Expr {
	None,
	Sequence(&'static [Code]),
	Print(&'static [Code]),
	Bool(bool),
	Int(i64),
	Float(f64),
	Str(&'static str),
}

#[derive(Copy, Clone, Debug)]
pub struct Code {
	pub expr: Expr,
	pub span: Span,
}

impl Code {
	pub fn list<T: IntoIterator<Item = U>, U: Compilable>(ctx: CodeContext, list: T) -> Result<&'static [Code]> {
		let list = list.into_iter().map(|x| x.compile(ctx));
		Error::unwrap_iter(list)
	}

	pub fn sequence<T: IntoIterator<Item = U>, U: Compilable>(ctx: CodeContext, sequence: T) -> Result<Code> {
		let mut output = Vec::new();
		for it in sequence {
			let code = it.compile(ctx)?;
			output.push(code);
		}

		match output.len() {
			0 => Ok(Code {
				span: ctx.span(),
				expr: Expr::None,
			}),
			1 => Ok(output[0]),
			_ => {
				let code = &*Arena::get().slice(output);
				Ok(Code {
					span: Span::for_range(code),
					expr: Expr::Sequence(code),
				})
			}
		}
	}
}

impl HasSpan for Code {
	fn span(&self) -> Span {
		self.span
	}
}

impl HasSpan for CodeContext {
	fn span(&self) -> Span {
		self.data.span
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

pub trait Compilable {
	fn compile(&self, ctx: CodeContext) -> Result<Code>;
}

impl Compilable for Node {
	fn compile(&self, ctx: CodeContext) -> Result<Code> {
		let node = *self;
		let value = self.value();
		value.output_code(ctx, node)
	}
}

impl<T: Compilable> Compilable for &T {
	fn compile(&self, ctx: CodeContext) -> Result<Code> {
		T::compile(self, ctx)
	}
}
