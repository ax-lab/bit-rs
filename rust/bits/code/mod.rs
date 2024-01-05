use std::collections::hash_map::Entry;

use super::*;

mod vars;

pub use vars::*;

pub struct Runtime<'a, 'b> {
	ctx: ContextRef<'a>,
	out: Writer<'b>,
	vars: HashMap<Var<'a>, Value<'a>>,
}

impl<'a, 'b> Runtime<'a, 'b> {
	pub fn new(ctx: ContextRef<'a>, out: Writer<'b>) -> Self {
		Self {
			ctx,
			out,
			vars: Default::default(),
		}
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
	Let(Var<'a>, &'a Code<'a>),
	Var(Var<'a>),
}

impl<'a> Expr<'a> {}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Code<'a> {
	expr: Expr<'a>,
	span: Span<'a>,
	node: Option<Node<'a>>,
}

impl<'a> Code<'a> {
	pub fn expr(&self) -> &Expr<'a> {
		&self.expr
	}

	pub fn span(&self) -> &Span<'a> {
		&self.span
	}

	pub fn node(&self) -> Option<Node<'a>> {
		self.node
	}

	pub fn execute<'b>(&self, rt: &mut Runtime<'a, 'b>) -> Result<Value<'a>> {
		let span = self.span;
		let value = match self.expr {
			Expr::None => Value::None,
			Expr::Seq(list) => {
				let mut output = Value::None;
				for it in list {
					output = it.execute(rt)?;
				}
				output
			}
			Expr::Unit => Value::Unit,
			Expr::Bool(v) => Value::Bool(v),
			Expr::SInt(v) => Value::SInt(v),
			Expr::UInt(v) => Value::UInt(v),
			Expr::Str(str) => Value::Str(str),
			Expr::Print(args) => {
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
			Expr::Let(var, code) => {
				let value = code.execute(rt)?;
				let entry = rt.vars.entry(var);
				match entry {
					Entry::Occupied(_) => err!("variable {var} is already defined (code at {span})")?,
					Entry::Vacant(entry) => {
						entry.insert(value);
					}
				}
				value
			}
			Expr::Var(var) => {
				if let Some(value) = rt.vars.get(&var) {
					*value
				} else {
					err!("variable {var} has not been initialized (code at {span})")?
				}
			}
		};
		Ok(value)
	}
}

struct NodeChain<'a, 'b> {
	value: Node<'a>,
	prev: Option<&'b NodeChain<'a, 'b>>,
}

impl<'a, 'b> NodeChain<'a, 'b> {
	pub fn contains(&self, node: Node<'a>) -> bool {
		if let Some(prev) = self.prev {
			if prev.value == node {
				true
			} else {
				prev.contains(node)
			}
		} else {
			false
		}
	}
}

impl<'a> Node<'a> {
	pub fn eval_type(self) -> Result<Type<'a>> {
		let head = NodeChain {
			value: self,
			prev: None,
		};
		self.do_eval_type(&head)
	}

	fn do_eval_type<'b>(self, chain: &NodeChain<'a, 'b>) -> Result<Type<'a>> {
		if chain.contains(self) {
			let span = self.span();
			err!("at {span}: node type depends on itself: {self}")?;
		}

		let chain = NodeChain {
			value: self,
			prev: Some(chain),
		};
		let chain = &chain;

		let types = self.context().types();
		let seq_type = || {
			self.nodes()
				.last()
				.map(|x| x.do_eval_type(chain))
				.unwrap_or(Ok(types.none()))
		};
		let child_type = || {
			self.nodes()
				.first()
				.map(|x| x.do_eval_type(chain))
				.unwrap_or(Ok(types.none()))
		};
		let typ = match self.value() {
			Value::None => types.none(),
			Value::Unit => types.unit(),
			Value::Bool(_) => types.bool(),
			Value::Str(_) => types.str(),
			Value::SInt(_) => types.sint(),
			Value::UInt(_) => types.uint(),
			Value::Source(_) => types.invalid(),
			Value::Module(_) => seq_type()?,
			Value::Token(Token::Integer) => types.sint(),
			Value::Token(Token::Literal) => types.str(),
			Value::Token(_) => types.invalid(),
			Value::Let(_) => child_type()?,
			Value::Var(var) => var.node().do_eval_type(chain)?,
			Value::Group { .. } => child_type()?,
			Value::Print => types.unit(),
			Value::BinaryOp(_op) => {
				// TODO: implement this
				types.invalid()
			}
		};
		Ok(typ)
	}

	pub fn compile(self) -> Result<Code<'a>> {
		let span = self.span();
		let ctx = self.context();
		let expr = match self.value() {
			Value::None => Expr::None,
			Value::Unit => Expr::Unit,
			Value::Bool(v) => Expr::Bool(v),
			Value::Str(v) => Expr::Str(v),
			Value::SInt(v) => Expr::SInt(v),
			Value::UInt(v) => Expr::UInt(v),
			Value::Source(_) => Expr::None,
			Value::Token(Token::Integer) => {
				let val = span.text();
				let val = parse_int(val, 10)?;
				Expr::SInt(val)
			}
			Value::Token(Token::Literal) => {
				// TODO: properly parse string
				let str = span.text();
				let str = &str[1..str.len() - 1];
				Expr::Str(str)
			}
			Value::Token(_) => Expr::None,
			Value::Let(var) => {
				let code = self.compile_child()?;
				let code = ctx.store(code);
				Expr::Let(var, code)
			}
			Value::Var(var) => Expr::Var(var),
			Value::Module(_) => self.compile_seq()?,
			Value::Group { .. } => return self.compile_child(),
			Value::Print => {
				let args = self.compile_nodes()?;
				Expr::Print(args)
			}
			Value::BinaryOp(op) => {
				if self.len() != 2 {
					err!("at {span}: binary operator must have exactly two children: {self}")?;
				}
				let nodes = self.nodes();
				let lhs = nodes[0].eval_type()?;
				let rhs = nodes[1].eval_type()?;
				err!("at {span}: operator {op} for {lhs} and {rhs} is not defined: {self}")?
			}
		};

		if expr == Expr::None && self.value() != Value::None {
			err!("at {span}: node cannot be compiled: {self}")?;
		}

		let code = Code {
			expr,
			span: self.span(),
			node: Some(self),
		};
		Ok(code)
	}

	fn compile_child(self) -> Result<Code<'a>> {
		let span = self.span();
		let nodes = self.nodes();
		match nodes.len() {
			0 => Ok(Code {
				expr: Expr::None,
				span,
				node: Some(self),
			}),
			1 => nodes[0].compile(),
			_ => err!("at {span}: single expression node with multiple children: {self}")?,
		}
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

pub fn parse_int<T: AsRef<str>>(value: T, base: i64) -> Result<i64> {
	let mut out = 0;
	for chr in value.as_ref().chars() {
		let val = chr as i64;
		let digit = match chr {
			'_' => continue,
			'0'..='9' => val - ('0' as i64),
			'a'..='f' => 0xA + (val - ('a' as i64)),
			'A'..='F' => 0xA + (val - ('A' as i64)),
			_ => err!("invalid digit `{chr}` in numeric literal")?,
		};
		out = out * base + digit;
	}
	Ok(out)
}
