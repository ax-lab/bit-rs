use super::*;

#[derive(Debug)]
pub struct SplitLines;

impl Eval for SplitLines {
	fn precedence(&self) -> Precedence {
		Precedence::LineSplit
	}

	fn execute(&self, nodes: &[Node]) -> Result<()> {
		for it in nodes {
			if let Some(Raw::List(tokens, flags)) = it.cast() {
				if flags.has(RawFlag::LineSplit) {
					continue;
				}

				let flags = flags.and(RawFlag::LineSplit);

				let mut nodes = Vec::new();
				let mut cur = 0;
				for (n, token) in tokens.list().iter().enumerate() {
					if let Token::Break(..) = token {
						if n > cur {
							let node = Node::new(Raw::List(tokens.range(cur..n), flags));
							nodes.push(node);
						}
						cur = n + 1;
					}
				}
				if cur > 0 {
					it.set_done(true);
					if cur < tokens.len() {
						let node = Node::new(Raw::List(tokens.range(cur..), flags));
						nodes.push(node);
					}

					it.replace(nodes);
				}
			}
		}
		Ok(())
	}
}
