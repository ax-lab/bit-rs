use super::*;

pub mod lexer;
pub use lexer::*;

impl<'a> Node<'a> {
	pub fn get_scope(self) -> Option<(Source<'a>, std::ops::Range<usize>)> {
		let mut cur = self.parent();
		while let Some(node) = cur {
			let span = node.span();
			let is_scope = !span.is_empty()
				&& match node.value() {
					Value::Group => true,
					Value::Source(..) => true,
					Value::Module(..) => true,
					_ => false,
				};

			if is_scope {
				return Some((span.src(), span.pos()..span.end()));
			}

			cur = node.parent();
		}

		None
	}
}

pub trait Evaluator<'a>: Debug {
	fn execute(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		if DEBUG_EVAL {
			let _ = self.print_op(ctx, &binding);
		}

		self.eval_nodes(ctx, binding)
	}

	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()>;

	fn print_op(&self, ctx: ContextRef<'a>, binding: &BoundNodes<'a>) -> Result<()> {
		let (pos, end, src) = (binding.pos(), binding.end(), binding.src());
		let _ = ctx;
		println!(
			"\n>>> Process {:?} -- {pos}:{end} @{src} / order = {} <<<",
			self,
			binding.order()
		);
		println!("{:#?}", binding.nodes());
		Ok(())
	}
}

#[derive(Debug)]
pub struct Output;

impl<'a> Evaluator<'a> for Output {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = (ctx, binding);
		Ok(())
	}
}

#[derive(Debug)]
pub struct DebugPrint<'a>(pub &'a str);

impl<'a> Evaluator<'a> for DebugPrint<'a> {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		if !DEBUG_EVAL {
			self.print_op(ctx, &binding)
		} else {
			Ok(())
		}
	}
}

#[derive(Debug)]
pub struct TokenizeSource;

impl<'a> Evaluator<'a> for TokenizeSource {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let mut errors = Vec::new();
		for it in binding.nodes() {
			if let Value::Source(source) = it.value() {
				let mut tokenizer = ctx.new_tokenizer()?;
				let tokens = tokenizer.parse_source(source);
				match tokens {
					Ok(tokens) => {
						let tokens = tokens
							.into_iter()
							.map(|(token, span)| ctx.node(Value::Token(token), span));
						it.set_value(Value::Module(source));
						it.flag_done();
						it.append_nodes(tokens);
					}
					Err(err) => errors.push(err),
				}
			} else {
				err!("invalid node for tokenizer operator: {it}")?;
			}
		}

		errors.combine("lexer ")?;
		Ok(())
	}
}

#[derive(Debug)]
pub struct SplitLine;

impl<'a> Evaluator<'a> for SplitLine {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		for (parent, targets) in binding.by_parent() {
			let old_nodes = parent.remove_nodes(..);
			let mut new_nodes = Vec::new();

			let mut push = |nodes: &[Node<'a>]| {
				if nodes.len() > 0 {
					let span = Span::range(nodes);
					let node = ctx.node(Value::Group, span);
					node.append_nodes(nodes);
					node.flag_done();
					new_nodes.push(node);
				}
			};

			let mut cur = 0;
			for it in targets {
				it.flag_silent();
				let index = it.index();
				let nodes = &old_nodes[cur..index];
				cur = index + 1;
				push(nodes);
			}

			push(&old_nodes[cur..]);
			parent.append_nodes(new_nodes);
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct Print;

impl<'a> Evaluator<'a> for Print {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		for (parent, targets) in binding.by_parent() {
			for it in targets.iter().rev() {
				it.flag_silent();
				let index = it.index();
				let nodes = parent.remove_nodes(index..);
				let span = Span::range(nodes);
				let node = ctx.node(Value::Print, span);
				node.set_nodes(&nodes[1..]);
				node.flag_done();
				parent.push_node(node);
			}
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct Let;

impl<'a> Evaluator<'a> for Let {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		for it in binding.nodes() {
			// keep alive by default to make loop easier
			it.keep_alive();

			let parent = if let Some(parent) = it.parent() {
				parent
			} else {
				continue;
			};

			if it.index() != 0 {
				continue;
			}

			let (name, expr, span) = if let Some(name) = it.next() {
				let has_eq = name.next().map(|x| x.value()) == Some(Value::Token(Token::Symbol(Symbol::str("="))));
				if !has_eq {
					continue;
				}

				if let Value::Token(Token::Word(name)) = name.value() {
					let nodes = parent.remove_nodes(..);
					nodes[0].done();
					nodes[1].done();
					nodes[2].done();
					let expr = &nodes[3..];
					let span = nodes[0].span().merged(nodes[1].span());
					(name, expr, span)
				} else {
					continue;
				}
			} else {
				continue;
			};

			if let Some((src, range)) = parent.get_scope() {
				ctx.bindings()
					.match_at(src, range, Match::word(name))
					.with_precedence(Value::SInt(i64::MAX))
					.bind(Var(name));
			} else {
				err!("let without scope at {span}")?;
			}

			let node = ctx.node(Value::Let(name), span);
			node.set_nodes(expr);
			node.flag_done();
			parent.push_node(node);
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct Var(Symbol);

impl<'a> Evaluator<'a> for Var {
	fn eval_nodes(&self, _ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		for it in binding.nodes() {
			it.set_value(Value::Var(self.0));
			it.done();
		}
		Ok(())
	}
}
