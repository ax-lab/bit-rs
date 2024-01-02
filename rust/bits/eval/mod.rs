use super::*;

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
