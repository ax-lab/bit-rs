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
	pub op: OpKey,
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
					let node = ctx.node(Value::BinaryOp(op), span);
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

#[derive(Debug)]
pub struct EvalIndentedBlock;

impl<'a> Evaluator<'a> for EvalIndentedBlock {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		for (node, targets) in binding.by_parent() {
			// only parse the block operator at the end of a group
			for it in targets {
				it.keep_alive();
			}

			let sep = targets.last().unwrap();
			if sep.next().is_some() {
				continue;
			}

			sep.ignore();

			let span = sep.span();
			if let Some(next) = sep.find_next() {
				let is_seq = if let Value::Sequence { indented, .. } = next.value() {
					indented
				} else {
					false
				};
				if !is_seq {
					let span_next = next.span();
					err!("at {span_next}: expected indented block for {span}, but found {next}")?;
				}

				let children = node.remove_nodes(..);
				let children = &children[..children.len() - 1];
				let head = ctx.node(Value::Group { scoped: true }, Span::range(children));
				head.set_nodes(children);
				head.flag_done();

				next.remove();
				next.flag_done();
				node.append_nodes([head, next]);
			} else {
				err!("at {span}: expected indented block after delimiter")?;
			}
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalBlock<'a>(
	pub &'static str,
	pub fn(ctx: ContextRef<'a>, root: Node<'a>, expr: &'a [Node<'a>], block: Node<'a>) -> Result<()>,
);

impl<'a> Evaluator<'a> for EvalBlock<'a> {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		let kind = self.0;
		let init = self.1;

		for (parent, matches) in binding.by_parent() {
			for it in matches {
				it.keep_alive();
			}

			let head = matches.first().unwrap();
			if head.index() != 0 || !parent.value().is_block() {
				continue;
			}

			head.ignore();

			let expr = parent.remove_nodes(..);
			let expr = &expr[1..];
			parent.ignore();

			let block = loop {
				let span = if let Some(next) = parent.next() {
					if let Value::Sequence { indented, .. } = next.value() {
						if indented {
							break next;
						}
					}
					next.span()
				} else {
					head.span()
				};
				err!("at {span}: {kind} must followed by an indented block")?;
			};

			let root = parent.parent().unwrap();
			let index = parent.index();
			root.remove_nodes(index..index + 2);

			let root = if root.len() == 0 && root.value().is_block() {
				root
			} else {
				let span = Span::merge(head.span(), block.span());
				let root = ctx.node(Value::Group { scoped: true }, span);
				root
			};

			init(ctx, root, expr, block)?;
		}
		Ok(())
	}
}

pub fn eval_if<'a>(ctx: ContextRef<'a>, root: Node<'a>, expr: &'a [Node<'a>], block: Node<'a>) -> Result<()> {
	let if_cond = ctx.node(Value::Group { scoped: true }, Span::range(expr));
	if_cond.set_nodes(expr);

	root.set_value(Value::If);
	root.append_nodes([if_cond, block]);
	Ok(())
}

pub fn eval_else<'a>(ctx: ContextRef<'a>, root: Node<'a>, expr: &'a [Node<'a>], block: Node<'a>) -> Result<()> {
	let (kind, expr) = if let Some(Value::Token(Token::Word(sym))) = expr.get(0).map(|x| x.value()) {
		if sym == Symbol::str("if") {
			expr[0].ignore();
			(Value::ElseIf, &expr[1..])
		} else {
			let span = Span::range(expr);
			err!("at {span}: else statement does not allow an expression")?
		}
	} else {
		(Value::Else, expr)
	};

	root.set_value(kind);
	root.flag_done();

	if expr.len() > 0 {
		let cond = ctx.node(Value::Group { scoped: true }, Span::range(expr));
		cond.set_nodes(expr);
		root.append_nodes([cond, block]);
	} else {
		root.append_nodes([block]);
	}

	Ok(())
}

pub fn eval_for<'a>(ctx: ContextRef<'a>, root: Node<'a>, expr: &'a [Node<'a>], block: Node<'a>) -> Result<()> {
	let for_expr = ctx.node(Value::Group { scoped: true }, Span::range(expr));
	for_expr.set_nodes(expr);

	root.set_value(Value::For);
	root.append_nodes([for_expr, block]);
	Ok(())
}

#[derive(Debug)]
pub struct EvalIf;

impl<'a> Evaluator<'a> for EvalIf {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = ctx;
		for if_node in binding.nodes() {
			let parent = if let Some(parent) = if_node.parent() {
				parent
			} else {
				continue;
			};

			let index = if_node.index();
			let mut chain = 0;
			while let Some(Value::ElseIf | Value::Else) = parent.node(index + chain + 1).map(|x| x.value()) {
				chain += 1;
			}

			let chain = parent.remove_nodes(index + 1..index + 1 + chain);
			if chain.len() > 0 {
				for it in chain.iter().take(chain.len() - 1) {
					if it.value() == Value::Else {
						let span = it.span();
						err!("at {span}: else must be the last statement in an if chain")?;
					}
				}

				let last = chain.last().unwrap();
				let (mut else_node, chain) = if let Value::Else = last.value() {
					debug_assert!(last.len() == 1);
					let node = last.node(0).unwrap();
					last.ignore();
					node.remove();
					(Some(node), &chain[..chain.len() - 1])
				} else {
					(None, chain)
				};

				for &else_if in chain.iter().rev() {
					else_if.set_value(Value::If);
					if let Some(else_node) = else_node {
						else_if.push_node(else_node);
					}
					else_node = Some(else_if);
				}

				if_node.push_node(else_node.unwrap());
			}
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalElse;

impl<'a> Evaluator<'a> for EvalElse {
	fn eval_nodes(&self, ctx: ContextRef<'a>, mut binding: BoundNodes<'a>) -> Result<()> {
		for (node, targets) in binding.by_parent() {
			for it in targets {
				it.keep_alive();
			}

			let head = targets.first().unwrap();
			if head.index() != 0 || !node.value().is_block() {
				continue;
			}

			head.ignore();

			let span_else = head.span();
			let if_node = if let Some(if_node) = head.find_prev_non_block() {
				if let Value::If = if_node.value() {
					if if_node.len() != 2 {
						err!("at {span_else}: invalid `else` statement (if block arity)")?;
					}
					if_node
				} else {
					let span = if_node.span();
					err!("at {span}: expected `if` before `else` at {span_else}")?
				}
			} else {
				err!("at {span_else}: invalid `else` statement (missing if)")?
			};

			let expr = node.remove_nodes(..);
			let expr = &expr[1..];
			node.ignore();

			let block = loop {
				let span = if let Some(next) = node.next() {
					if let Value::Sequence { indented, .. } = next.value() {
						if indented {
							break next;
						}
					}
					next.span()
				} else {
					head.span()
				};
				err!("at {span}: else statement must followed by an indented block")?;
			};

			let root = node.parent().unwrap();
			let index = node.index();
			root.remove_nodes(index..index + 2);

			if root.len() == 0 && root.value().is_block() {
				root.remove();
				root.ignore();
			}

			let block = if expr.len() > 0 {
				let expr_node = ctx.node(Value::Group { scoped: true }, Span::range(expr));
				expr_node.set_nodes(expr);

				let new_block = ctx.node(
					Value::Group { scoped: true },
					Span::merge(expr_node.span(), block.span()),
				);
				block.remove();
				new_block.append_nodes([expr_node, block]);
				new_block
			} else {
				block
			};

			block.remove();
			if_node.push_node(block);
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalBool(pub bool);

impl<'a> Evaluator<'a> for EvalBool {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = ctx;
		for it in binding.nodes() {
			it.set_value(Value::Bool(self.0));
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct EvalFor;

impl<'a> Evaluator<'a> for EvalFor {
	fn eval_nodes(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = (ctx, binding);
		todo!()
	}
}
