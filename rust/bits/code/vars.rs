use super::*;

pub struct VariablesContext<'a> {
	ctx: ContextRef<'a>,
}

impl<'a> IsContext<'a> for VariablesContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		Self { ctx }
	}
}

impl<'a> VariablesContext<'a> {
	pub fn declare(&self, name: Symbol, source: Node<'a>) -> Var<'a> {
		let data = self.ctx.store(VarData { name, source });
		Var { data }
	}
}

#[derive(Copy, Clone)]
pub struct Var<'a> {
	data: &'a VarData<'a>,
}

impl<'a> Var<'a> {
	pub fn name(&self) -> Symbol {
		self.data.name
	}

	pub fn source(&self) -> Node<'a> {
		self.data.source
	}
}

struct VarData<'a> {
	name: Symbol,
	source: Node<'a>,
}

impl<'a> Eq for Var<'a> {}

impl<'a> Ord for Var<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.name()
			.cmp(&other.name())
			.then_with(|| self.source().cmp(&other.source()))
	}
}

impl<'a> PartialOrd for Var<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> PartialEq for Var<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Hash for Var<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(self.data as *const VarData).hash(state)
	}
}

impl<'a> Display for Var<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let span = self.data.source.span();
		write!(f, "`")?;
		self.data.name.write_name(f, false)?;
		write!(f, "`")?;
		if !span.is_empty() {
			write!(f, " from {span}")?;
		}
		Ok(())
	}
}

impl<'a> Debug for Var<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let span = self.data.source.span();
		write!(f, "Var(")?;
		self.data.name.write_name(f, false)?;
		if !span.is_empty() {
			write!(f, " from {span}")?;
		}
		write!(f, ")")?;
		Ok(())
	}
}
