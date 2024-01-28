use super::*;

pub trait Eval: 'static + Debug {
	fn precedence(&self) -> Precedence;

	fn execute(&self, nodes: &[Node]) -> Result<()>;
}

pub trait GlobalInit: 'static + Debug {
	fn init_eval(&'static self, src: Source) -> &'static dyn Eval;
}
