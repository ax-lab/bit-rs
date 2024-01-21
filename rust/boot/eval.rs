use super::*;

pub trait Eval: 'static {
	fn precedence(&self) -> Precedence;

	fn execute(&self, nodes: Vec<Node>) -> Result<()>;
}
