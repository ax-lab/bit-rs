use super::*;

#[derive(Debug)]
pub struct Print;

impl IsValue for Print {}

#[derive(Debug)]
pub struct ParsePrint;

impl Eval for ParsePrint {
	fn precedence(&self) -> Precedence {
		Precedence::Print
	}

	fn execute(&self, nodes: &[Node]) -> Result<()> {
		for it in nodes {
			it.set_done(true);
			if let Some(parent) = it.parent() {
				let index = it.index();
				let children = parent.remove_nodes(index..);
				let print = Node::new_at(Print, children.span());
				print.set_done(true);

				let children = children.range(1..);
				print.append_nodes(children);

				parent.insert_nodes(index, [print]);
			}
		}
		Ok(())
	}
}
