use super::*;

pub static RAW: Bindings = Bindings::new();

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum RawFlag {
	None = 0,
	LineSplit = 1 << 0,
}

impl RawFlag {
	#[inline(always)]
	pub fn and(self, flag: RawFlag) -> RawFlag {
		unsafe { std::mem::transmute(self as u32 | flag as u32) }
	}

	#[inline(always)]
	pub fn has(self, flag: RawFlag) -> bool {
		(self as u32 & flag as u32) > 0
	}
}

pub enum Raw {
	List(TokenList, RawFlag),
	Empty(Span),
}

impl Raw {
	pub fn list(&self) -> &'static [Token] {
		match self {
			Raw::List(inner, _) => inner.list(),
			Raw::Empty(_) => &[],
		}
	}
}

impl IsValue for Raw {
	fn bind(&self, node: Node) {
		RAW.add(node)
	}
}

impl HasSpan for Raw {
	fn span(&self) -> Span {
		match self {
			Raw::List(inner, _) => inner.span(),
			Raw::Empty(span) => *span,
		}
	}
}

impl Writable for Raw {
	fn write(&self, f: &mut Writer) -> Result<()> {
		match self {
			Raw::List(inner, _) => {
				write!(f, "Raw(")?;
				{
					let out = &mut f.indented();
					for (n, it) in inner.list().iter().enumerate() {
						write!(out, "\n[{n}] {it}")?;
						out.write_location(" at ", it.span())?;
					}
				}
				write!(f, "\n)")?;
			}
			Raw::Empty(span) => write!(f, "Raw(empty at {span})")?,
		}
		Ok(())
	}
}

formatted!(Raw);

#[derive(Debug)]
pub struct ExpandRaw;

impl Eval for ExpandRaw {
	fn precedence(&self) -> Precedence {
		Precedence::ExpandRaw
	}

	fn execute(&self, nodes: &[Node]) -> Result<()> {
		for it in nodes {
			if let Some(raw) = it.cast::<Raw>() {
				it.set_done(true);
				match raw {
					Raw::List(tokens, ..) => {
						let group = Node::new_at(Group, tokens.span());
						let children = tokens.list().iter().map(|x| Node::new(*x));
						group.append_nodes(children);
						it.replace([group]);
					}
					Raw::Empty(..) => {
						it.remove();
					}
				}
			}
		}
		Ok(())
	}
}
