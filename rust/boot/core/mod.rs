use super::*;

mod program;

pub use program::*;

impl IsValue for Source {
	fn describe(&self, out: &mut Writer) -> Result<()> {
		write!(out, "source text `{self}`")?;
		Ok(())
	}
}
