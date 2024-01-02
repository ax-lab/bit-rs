use super::*;

pub mod binding;
pub use binding::*;

pub mod heap;
pub mod iter;

pub use heap::*;
pub use iter::*;

pub struct NodeContext<'a> {
	ctx: ContextRef<'a>,
	bindings: Bindings<'a>,
}

impl<'a> IsContext<'a> for NodeContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		Self {
			ctx,
			bindings: Bindings::new(ctx),
		}
	}
}

impl<'a> ContextRef<'a> {
	pub fn node(&self, value: Value<'a>, span: Span<'a>) -> Node<'a> {
		self.nodes().new_node(value, span)
	}

	pub fn bindings(&self) -> &'a Bindings<'a> {
		&self.nodes().bindings
	}
}

impl<'a> NodeContext<'a> {
	pub fn new_node(&self, value: Value<'a>, span: Span<'a>) -> Node<'a> {
		let data = self.ctx.store(NodeData {
			ctx: self.ctx,
			span,
			value: value.into(),
			nodes: Default::default(),
			parent: Default::default(),
			index: Default::default(),
			is_done: false.into(),
		});
		let node = Node { data };
		self.reindex_node(node);
		node
	}

	pub fn reindex_node(&self, node: Node<'a>) {
		self.bindings.add(node.key(), node);
	}
}

#[derive(Copy, Clone)]
pub struct Node<'a> {
	data: &'a NodeData<'a>,
}

pub struct NodeData<'a> {
	ctx: ContextRef<'a>,
	span: Span<'a>,
	value: Cell<Value<'a>>,
	nodes: Cell<&'a [Node<'a>]>,
	parent: Cell<Option<Node<'a>>>,
	index: Cell<usize>,
	is_done: Cell<bool>,
}

impl<'a> Node<'a> {
	pub fn context(self) -> ContextRef<'a> {
		self.data.ctx
	}

	pub fn arena(self) -> &'a Arena {
		self.context().arena()
	}

	#[inline]
	pub fn value(self) -> Value<'a> {
		self.data.value.get()
	}

	pub fn set_value(self, value: Value<'a>) {
		self.data.value.set(value);
		self.context().nodes().reindex_node(self)
	}

	#[inline]
	pub fn pos(self) -> usize {
		self.data.span.pos()
	}

	#[inline]
	pub fn span(self) -> Span<'a> {
		self.data.span
	}

	#[inline]
	pub fn nodes(self) -> &'a [Node<'a>] {
		self.data.nodes.get()
	}

	#[inline]
	pub fn len(self) -> usize {
		self.nodes().len()
	}

	pub fn node(self, index: usize) -> Option<Node<'a>> {
		self.nodes().get(index).copied()
	}

	pub fn set_nodes(self, nodes: &'a [Node<'a>]) {
		for it in self.nodes() {
			it.data.parent.set(None);
			it.data.index.set(0);
		}

		self.data.nodes.set(nodes);
		for (n, it) in nodes.iter().enumerate() {
			assert!(it.parent().is_none());
			it.data.parent.set(Some(self));
			it.data.index.set(n);
		}
	}

	pub fn parent(self) -> Option<Node<'a>> {
		self.data.parent.get()
	}

	pub fn index(self) -> usize {
		self.data.index.get()
	}

	pub fn next(self) -> Option<Node<'a>> {
		let next = self.index() + 1;
		self.parent().and_then(|x| x.node(next))
	}

	pub fn prev(self) -> Option<Node<'a>> {
		let index = self.index();
		if index == 0 {
			return None;
		}

		let prev = index - 1;
		self.parent().and_then(|x| x.node(prev))
	}

	pub fn insert_nodes<T: IntoIterator<Item = Node<'a>>>(self, at: usize, nodes: T)
	where
		T::IntoIter: ExactSizeIterator,
	{
		let nodes = nodes.into_iter();
		let len = nodes.len();
		if len == 0 {
			return;
		}

		let arena = self.data.ctx.arena();
		let head = self.nodes()[..at].iter().copied();
		let tail = self.nodes()[at..].iter().copied();
		let nodes = head.chain_exact(nodes).chain_exact(tail);
		let nodes: &'a [Node<'a>] = arena.slice(nodes);
		for (n, it) in nodes.iter().enumerate().skip(at).take(len) {
			assert!(it.parent().is_none());
			it.data.parent.set(Some(self));
			it.data.index.set(n);
		}
		self.data.nodes.set(nodes);
	}

	pub fn append_nodes<T: IntoIterator<Item = Node<'a>>>(self, nodes: T)
	where
		T::IntoIter: ExactSizeIterator,
	{
		self.insert_nodes(self.len(), nodes)
	}

	pub fn remove_nodes<T: RangeBounds<usize>>(self, range: T) -> &'a [Node<'a>] {
		let arena = self.data.ctx.arena();
		let nodes = self.nodes();
		let sta = match range.start_bound() {
			std::ops::Bound::Included(&n) => n,
			std::ops::Bound::Excluded(&n) => n + 1,
			std::ops::Bound::Unbounded => 0,
		};
		let end = match range.end_bound() {
			std::ops::Bound::Included(&n) => n + 1,
			std::ops::Bound::Excluded(&n) => n,
			std::ops::Bound::Unbounded => nodes.len(),
		};

		assert!(sta <= end && end <= nodes.len());
		let removed = &nodes[sta..end];
		if removed.len() == 0 {
			return removed;
		}

		for it in removed {
			it.data.parent.set(None);
			it.data.index.set(0);
		}

		let nodes = if sta == 0 {
			&nodes[end..]
		} else if end >= self.len() {
			&nodes[..sta]
		} else {
			let head = nodes[0..sta].iter().copied();
			let tail = nodes[end..].iter().copied();
			let nodes = head.chain_exact(tail);
			arena.slice(nodes)
		};

		for n in sta..nodes.len() {
			let it = nodes[n].data;
			it.index.set(n);
		}

		self.data.nodes.set(nodes);
		return removed;
	}

	pub fn get_type(self) -> Type<'a> {
		todo!()
	}

	pub fn keep_alive(self) {
		self.data.is_done.set(false);
	}

	pub fn flag_done(self) {
		self.data.is_done.set(true);
	}

	pub fn is_done(self) -> bool {
		self.data.is_done.get()
	}

	fn as_ptr(self) -> *const NodeData<'a> {
		self.data
	}

	#[allow(unused)]
	fn check_node(self, recursive: bool) {
		if let Some(parent) = self.parent() {
			assert_eq!(Some(self), parent.node(self.index()));
		}

		for (n, it) in self.nodes().iter().copied().enumerate() {
			assert_eq!(Some(self), it.parent());
			assert_eq!(n, it.index());
		}

		if recursive {
			for it in self.nodes() {
				it.check_node(true);
			}
		}
	}
}

impl<'a> Display for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.value())
	}
}

impl<'a> Debug for Node<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?} -- {:?}", self.value(), self.span())
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
		self.value()
			.cmp(&other.value())
			.then_with(|| self.span().cmp(&other.span()))
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
