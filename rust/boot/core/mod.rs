use super::*;

mod program;

pub use program::*;

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
pub struct PrintSource;

impl Eval for PrintSource {
	fn precedence(&self) -> Precedence {
		Precedence::Source
	}

	fn execute(&self, nodes: Vec<Node>) -> Result<()> {
		for it in nodes {
			if let Some(src) = it.cast::<Source>() {
				it.set_done(true);
				println!("\n>>> {src} <<<\n");
				println!("{}■", src.text());
			}
		}
		Ok(())
	}
}
