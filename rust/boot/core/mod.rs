use super::*;

mod program;
mod raw;

pub use program::*;
pub use raw::*;

pub static SOURCES: Bindings = Bindings::new();

impl IsValue for Source {
	fn describe(&self, out: &mut Writer) -> Result<()> {
		write!(out, "source text `{self}`")?;
		Ok(())
	}

	fn bind(&self, node: Node) {
		SOURCES.add_node(node);
	}
}

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
