use super::*;

mod group;
mod lines;
mod program;
mod raw;

pub use group::*;
pub use lines::*;
pub use program::*;
pub use raw::*;

#[derive(Debug)]
pub struct Global<T: Eval>(T);

impl<T: Eval> Global<T> {
	pub fn new(eval: T) -> Self {
		Self(eval)
	}
}

impl<T: Eval> GlobalInit for Global<T> {
	fn init_eval(&'static self, src: Source) -> &'static dyn Eval {
		let _ = src;
		&self.0
	}
}

#[derive(Debug)]
pub struct RemoveNode(pub Precedence);

impl Eval for RemoveNode {
	fn precedence(&self) -> Precedence {
		self.0
	}

	fn execute(&self, nodes: &[Node]) -> Result<()> {
		for it in nodes {
			it.set_done(true);
			it.remove();
		}
		Ok(())
	}
}
