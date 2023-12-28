use super::*;

#[derive(Copy, Clone)]
pub struct Node<'a> {
	data: &'a NodeData<'a>,
}

pub struct NodeData<'a> {
	ctx: ContextRef<'a>,
	value: Value<'a>,
}

impl<'a> Node<'a> {
	#[inline]
	pub fn context(&self) -> ContextRef<'a> {
		self.data.ctx
	}

	#[inline]
	pub fn store(&self) -> &'a Store {
		self.context().arena()
	}

	#[inline]
	pub fn value(&self) -> &'a Value<'a> {
		&self.data.value
	}

	pub fn get_type(&self) -> Type<'a> {
		todo!()
	}

	fn as_ptr(&self) -> *const NodeData<'a> {
		self.data
	}
}

impl<'a> ContextRef<'a> {
	pub fn node(&self, value: Value<'a>) -> Node<'a> {
		let data = self.store(NodeData { ctx: *self, value });
		Node { data }
	}
}

impl<'a> Display for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.value())
	}
}

impl<'a> Debug for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.value())
	}
}

impl<'a> Eq for Node<'a> {}

impl<'a> PartialEq for Node<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl<'a> Ord for Node<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.value().cmp(other.value())
	}
}

impl<'a> PartialOrd for Node<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> Hash for Node<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_ptr().hash(state)
	}
}
