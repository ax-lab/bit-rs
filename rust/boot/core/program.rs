use super::*;

#[derive(Debug)]
pub struct Program;

impl IsValue for Program {
	fn process(&self, msg: Message) -> Result<bool> {
		match msg {
			Message::Output(node, out) => {
				if node.len() == 0 {
					write!(out, "Program is empty")?;
				} else {
					write!(out, "Program:\n")?;

					let out = &mut out.indented();
					for it in node.nodes() {
						write!(out, "\n")?;
						it.write_with_pos(out)?;
					}
				}

				true
			}

			_ => return Ok(false),
		};
		Ok(true)
	}
}
