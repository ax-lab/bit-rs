use super::*;

#[derive(Debug)]
pub struct Program;

impl IsValue for Program {
	fn process(&self, msg: Message) -> Result<()> {
		match msg {
			Message::AreYouOkay(ans) => *ans = true,
			_ => {}
		}
		Ok(())
	}
}
