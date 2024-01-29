use super::*;

#[derive(Debug)]
pub struct Group;

impl IsValue for Group {
	fn is_collection(&self) -> bool {
		true
	}

	fn output_code(&self, ctx: CodeContext, node: Node) -> Result<Code> {
		let children = node.children();
		match children.len() {
			0 => {
				let code = Code {
					expr: Expr::None,
					span: node.span(),
				};
				Ok(code)
			}
			1 => children[0].compile(ctx),
			_ => raise!(@node => "invalid group with multiple children:\n{node}"),
		}
	}
}
