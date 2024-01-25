use super::*;

pub trait Eval: 'static + Debug {
	fn precedence(&self) -> Precedence;

	fn execute(&self, nodes: Vec<Node>) -> Result<()>;
}
