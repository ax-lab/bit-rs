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
					Value::Group { scoped, .. } => scoped,
					Value::Sequence { scoped, .. } => scoped,
					Value::Source(..) => true,
					Value::Module(..) => true,
					_ => false,
				};

			if is_scope {
				return Some((span.src(), span.pos()..span.end()));
			}

			cur = node.parent();
		}

		let src = self.span().src();
		if src == Source::default() {
			None
		} else {
			Some((src, 0..src.len()))
		}
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
pub struct EvalLineBreak;

impl<'a> Evaluator<'a> for EvalLineBreak {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		for (parent, targets) in binding.by_parent() {
			let old_nodes = parent.remove_nodes(..);
			let mut new_nodes = Vec::new();
			let mut indent = Vec::new();
			let mut span_last = Span::empty();

			let mut push = |nodes: &[Node<'a>]| -> Result<()> {
				if nodes.len() > 0 {
					let span = Span::range(nodes);
					span_last = span.to_end();
					let level = span.indent();
					let span_indent = span.truncated(0);
					if let Some(&current) = indent.last() {
						if level > current {
							indent.push(level);
							let inc = ctx.node(Value::Indent(true), span_indent);
							new_nodes.push(inc);
						} else if level < current {
							let mut current = current;
							while level < current {
								indent.pop();
								if let Some(&last) = indent.last() {
									current = last;
								} else {
									let span = span.truncated(0);
									err!("at {span}: invalid indentation (dedent is less than the base indentation)")?;
								}

								let dec = ctx.node(Value::Indent(false), span_indent);
								new_nodes.push(dec);
							}
						}
					} else {
						indent.push(level);
					}
					let node = ctx.node(Value::Group { scoped: false }, span);
					node.append_nodes(nodes);
					node.flag_done();
					new_nodes.push(node);
				}
				Ok(())
			};

			let mut cur = 0;
			for it in targets {
				it.ignore();
				let index = it.index();
				let nodes = &old_nodes[cur..index];
				cur = index + 1;
				push(nodes)?;
			}

			push(&old_nodes[cur..])?;

			while indent.len() > 1 {
				indent.pop();
				let dec = ctx.node(Value::Indent(false), span_last);
				new_nodes.push(dec);
			}

			parent.append_nodes(new_nodes);
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalPrint;

impl<'a> Evaluator<'a> for EvalPrint {
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
pub struct EvalLet;

impl<'a> Evaluator<'a> for EvalLet {
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
					nodes[0].ignore();
					nodes[1].ignore();
					nodes[2].ignore();
					let expr = &nodes[3..];
					let span = nodes[0].span().merged(nodes[1].span());
					(name, expr, span)
				} else {
					continue;
				}
			} else {
				continue;
			};

			let node = ctx.node(Value::None, span);
			let let_value = if let Some((src, mut range)) = it.get_scope() {
				range.start = if let Some(last) = expr.last() {
					last.span().end()
				} else {
					span.end()
				};
				let var = ctx.variables().declare(name, node);
				ctx.bindings()
					.match_at(src, range, Match::word(name))
					.with_precedence(Value::SInt(i64::MAX))
					.bind(EvalVar(var));

				let expr_span = Span::range(expr);
				ctx.bindings()
					.match_at(src, expr_span.pos()..expr_span.end(), Match::word(Symbol::str("this")))
					.with_precedence(Value::SInt(i64::MAX))
					.bind(EvalVar(var));

				Value::Let(var)
			} else {
				err!("let without scope at {span}")?
			};

			node.set_value(let_value);
			node.set_nodes(expr);
			node.flag_done();
			parent.push_node(node);
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalVar<'a>(Var<'a>);

impl<'a> Evaluator<'a> for EvalVar<'a> {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = ctx;
		for it in binding.nodes() {
			it.set_value(Value::Var(self.0));
			it.ignore();
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalBinaryOp {
	pub op: Symbol,
	pub group_right: bool,
}

impl<'a> Evaluator<'a> for EvalBinaryOp {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		let op = self.op;

		let make_op_node = |cur_node: &Node<'a>,
		                    prev_op_node: Option<Node<'a>>,
		                    op_value: &'a [Node<'a>]|
		 -> Result<Option<Node<'a>>> {
			cur_node.ignore();
			if op_value.len() == 0 {
				let span = cur_node.span();
				err!("at {span}: operand for binary {op} is empty")?;
			}

			let op_span = Span::range(op_value);
			let op_node = ctx.node(Value::Group { scoped: false }, op_span);
			op_node.flag_done();
			op_node.set_nodes(op_value);
			let node = match prev_op_node {
				None => op_node,
				Some(op_prev) => {
					let span = Span::merge(op_prev.span(), op_node.span());
					let node = ctx.node(Value::BinaryOp(OpKey(OpKind::Core, op)), span);
					node.flag_done();
					if self.group_right {
						node.append_nodes([op_node, op_prev]);
					} else {
						node.append_nodes([op_prev, op_node]);
					}
					node
				}
			};
			Ok(Some(node))
		};

		for (node, targets) in binding.by_parent() {
			let mut binary_op = None;
			let mut children = node.remove_nodes(..);
			if self.group_right {
				for it in targets.iter().rev() {
					let idx = it.index();
					let op_value = &children[idx + 1..];
					children = &children[..idx];
					binary_op = make_op_node(it, binary_op, op_value)?;
				}
				binary_op = make_op_node(targets.last().unwrap(), binary_op, children)?;
			} else {
				let mut cur = 0;
				for it in targets.iter() {
					let idx = it.index();
					let op_value = &children[cur..idx];
					cur = idx + 1;
					binary_op = make_op_node(it, binary_op, op_value)?;
				}
				binary_op = make_op_node(targets.last().unwrap(), binary_op, &children[cur..])?;
			}

			node.push_node(binary_op.unwrap());
		}

		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalIndent;

impl<'a> Evaluator<'a> for EvalIndent {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		for (node, targets) in binding.by_parent() {
			let children = node.remove_nodes(..);
			let mut stack = vec![Vec::<Node<'a>>::new()];
			let mut cur = 0;
			for it in targets {
				it.ignore();
				let idx = it.index();
				if idx > cur {
					let last = stack.last_mut().unwrap();
					last.extend(&children[cur..idx]);
				}
				cur = idx + 1;

				match it.value() {
					Value::Indent(true) => {
						stack.push(Vec::new());
					}

					Value::Indent(false) => {
						if stack.len() <= 1 {
							let span = it.span();
							err!("[BUG] unbalanced dedent at {span}")?;
						}

						let nodes = stack.pop().unwrap();
						let node = ctx.node(
							Value::Sequence {
								scoped: true,
								indented: true,
							},
							Span::range(&nodes),
						);
						node.append_nodes(nodes);
						stack.last_mut().unwrap().push(node);
					}

					_ => err!("[BUG] invalid target node for eval indent: {it}")?,
				}
			}

			stack.last_mut().unwrap().extend(&children[cur..]);

			if stack.len() != 1 {
				let span = targets.first().unwrap().span();
				err!("[BUG] missing dedent for {span}")?;
			}

			let new_nodes = stack.pop().unwrap();
			node.append_nodes(new_nodes);
		}
		Ok(())
	}
}
