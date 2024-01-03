use super::*;

pub mod lexer;
pub use lexer::*;

pub trait Evaluator<'a>: Debug {
	fn parse(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()>;
}

#[derive(Debug)]
pub struct NoOp;

impl<'a> Evaluator<'a> for NoOp {
	fn parse(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = (ctx, binding);
		Ok(())
	}
}

#[derive(Debug)]
pub struct DebugPrint<'a>(pub &'a str);

impl<'a> Evaluator<'a> for DebugPrint<'a> {
	fn parse(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
		let _ = ctx;
		println!(
			"\n>>> Process {} -- {} / order = {} <<<",
			self.0,
			binding.span(),
			binding.order()
		);
		println!("{:#?}", binding.nodes());
		Ok(())
	}
}

#[derive(Debug)]
pub struct TokenizeSource;

impl<'a> Evaluator<'a> for TokenizeSource {
	fn parse(&self, ctx: ContextRef<'a>, binding: BoundNodes<'a>) -> Result<()> {
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
