use super::*;

pub struct Runtime<'a> {
	ctx: ContextRef<'a>,
	out: Writer<'a>,
}

impl<'a> Runtime<'a> {
	pub fn new(ctx: ContextRef<'a>, out: Writer<'a>) -> Self {
		Self { ctx, out }
	}

	pub fn context(&self) -> ContextRef<'a> {
		self.ctx
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Expr<'a> {
	None,
	Seq(&'a [Code<'a>]),
	Unit,
	Bool(bool),
	SInt(i64),
	UInt(u64),
	Str(&'a str),
	Print(&'a [Code<'a>]),
}

impl<'a> Expr<'a> {
	pub fn eval(&self, rt: &mut Runtime<'a>) -> Result<Value<'a>> {
		let value = match self {
			&Expr::None => Value::None,
			&Expr::Seq(list) => {
				let mut output = Value::None;
				for it in list {
					output = it.execute(rt)?;
				}
				output
			}
			&Expr::Unit => Value::Unit,
			&Expr::Bool(v) => Value::Bool(v),
			&Expr::SInt(v) => Value::SInt(v),
			&Expr::UInt(v) => Value::UInt(v),
			&Expr::Str(str) => Value::Str(str),
			&Expr::Print(args) => {
				let mut has_output = false;
				for it in args {
					let it = it.execute(rt)?;
					match it {
						Value::None => continue,
						Value::Unit => continue,
						Value::Str(s) => {
							if s.len() == 0 {
								continue;
							}
						}
						_ => {}
					}

					if has_output {
						write!(rt.out, " ")?;
					}
					write!(rt.out, "{it}")?;
					has_output = true;
				}
				write!(rt.out, "\n")?;
				Value::Unit
			}
		};
		Ok(value)
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Code<'a> {
	expr: Expr<'a>,
	span: Span<'a>,
	node: Option<Node<'a>>,
}

impl<'a> Code<'a> {
	pub fn execute(&self, rt: &mut Runtime<'a>) -> Result<Value<'a>> {
		self.expr.eval(rt)
	}
}

impl<'a> Node<'a> {
	pub fn compile(self) -> Result<Code<'a>> {
		let expr = match self.value() {
			Value::None => Expr::None,
			Value::Unit => Expr::Unit,
			Value::Bool(v) => Expr::Bool(v),
			Value::Str(v) => Expr::Str(v),
			Value::SInt(v) => Expr::SInt(v),
			Value::UInt(v) => Expr::UInt(v),
			Value::Source(_) => Expr::None,
			Value::Token(Token::Literal) => {
				// TODO: properly parse string
				let str = self.span().text();
				let str = &str[1..str.len() - 1];
				Expr::Str(str)
			}
			Value::Token(_) => Expr::None,
			Value::Module(_) => self.compile_seq()?,
			Value::Group => self.compile_seq()?,
			Value::Print => {
				let args = self.compile_nodes()?;
				Expr::Print(args)
			}
		};

		if expr == Expr::None && self.value() != Value::None {
			let span = self.span();
			err!("at {span}: node cannot be compiled: {self}")?;
		}

		let code = Code {
			expr,
			span: self.span(),
			node: Some(self),
		};
		Ok(code)
	}

	fn compile_seq(self) -> Result<Expr<'a>> {
		let seq = self.compile_nodes()?;
		Ok(Expr::Seq(seq))
	}

	fn compile_nodes(self) -> Result<&'a [Code<'a>]> {
		let mut sequence = Vec::new();
		for it in self.nodes() {
			let code = it.compile()?;
			sequence.push(code);
		}

		let sequence = self.arena().slice(sequence);
		Ok(sequence)
	}
}

impl<'a> Debug for Code<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let expr = &self.expr;
		let span = self.span;
		write!(f, "# from {span}:\n{expr:#?}")
	}
}
