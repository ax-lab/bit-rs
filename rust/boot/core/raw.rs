use super::*;

pub static RAW: Bindings = Bindings::new();

pub enum Raw {
	List(TokenList),
	Empty(Span),
}

impl Raw {
	pub fn list(&self) -> &'static [Token] {
		match self {
			Raw::List(inner) => inner.list(),
			Raw::Empty(_) => &[],
		}
	}
}

impl IsValue for Raw {
	fn bind(&self, node: Node) {
		RAW.add_node(node)
	}
}

impl HasSpan for Raw {
	fn span(&self) -> Span {
		match self {
			Raw::List(inner) => inner.span(),
			Raw::Empty(span) => *span,
		}
	}
}

impl Writable for Raw {
	fn write(&self, f: &mut Writer) -> Result<()> {
		match self {
			Raw::List(inner) => {
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
