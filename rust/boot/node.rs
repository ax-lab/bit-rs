use super::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Node {
	data: NonNull<NodeData>,
}

struct NodeData {
	span: Span,
	done: AtomicBool,
	value: ValueCell,
	parent: NodeCell,
	index: AtomicUsize,
	children: List<Node>,
	next_pending: AtomicPtr<NodeData>,
}

static PENDING_NODES: AtomicPtr<NodeData> = AtomicPtr::new(std::ptr::null_mut());

impl Node {
	pub fn new<T: Into<Value> + HasSpan>(value: T) -> Self {
		let span = value.span();
		Self::new_at(value, span)
	}

	pub fn new_at<T: Into<Value>>(value: T, span: Span) -> Self {
		let value = value.into();
		let data = Arena::get().alloc(NodeData {
			span,
			done: false.into(),
			value: ValueCell::new(value),
			parent: Default::default(),
			index: Default::default(),
			children: Default::default(),
			next_pending: Default::default(),
		});

		let node = Self { data };

		loop {
			let next = PENDING_NODES.load(Order::Relaxed);
			node.data().next_pending.store(next, Order::Relaxed);
			if PENDING_NODES
				.compare_exchange_weak(next, data.as_ptr(), Order::Relaxed, Order::Relaxed)
				.is_ok()
			{
				break;
			}
		}

		value.get().bind(node);
		node
	}

	pub fn check_pending() -> Result<()> {
		let mut pending = PENDING_NODES
			.fetch_update(Order::Release, Order::Acquire, |_| Some(std::ptr::null_mut()))
			.unwrap();
		while let Some(data) = NonNull::new(pending) {
			let node = Node { data };
			if !node.done() {
				return raise!(@node => "node has not been solved:\n{node}");
			}

			pending = node.data().next_pending.load(Order::Relaxed);
		}
		Ok(())
	}

	pub fn send(&self, msg: Message) -> Result<bool> {
		self.value().get().process(msg)
	}

	#[inline(always)]
	pub fn done(&self) -> bool {
		let data = self.data();
		data.done.load(Order::Relaxed)
	}

	#[inline(always)]
	pub fn set_done(&self, done: bool) {
		let data = self.data();
		if !done {
			if data
				.done
				.compare_exchange(true, false, Order::Relaxed, Order::Relaxed)
				.is_ok()
			{
				let value = data.value.get();
				value.get().bind(*self);
			}
		} else {
			data.done.store(done, Order::Relaxed);
		}
	}

	#[inline(always)]
	pub fn offset(&self) -> usize {
		let data = self.data();
		data.span.sta()
	}

	#[inline(always)]
	pub fn source(&self) -> Source {
		let data = self.data();
		data.span.source()
	}

	#[inline(always)]
	pub fn value(&self) -> Value {
		let data = self.data();
		data.value.get()
	}

	#[inline(always)]
	pub fn cast<T: IsValue>(&self) -> Option<&'static T> {
		self.value().cast()
	}

	#[inline(always)]
	pub fn set_value(&self, value: Value) {
		let data = self.data();
		data.done.store(false, Order::Relaxed);
		data.value.set(value);
		value.get().bind(*self);
	}

	#[inline(always)]
	pub fn children(&self) -> &'static [Node] {
		let data = self.data();
		data.children.items()
	}

	#[inline(always)]
	pub fn len(&self) -> usize {
		self.children().len()
	}

	#[inline(always)]
	pub fn node(&self, index: usize) -> Option<Node> {
		self.children().get(index).copied()
	}

	#[inline(always)]
	pub fn first(&self) -> Option<Node> {
		self.node(0)
	}

	#[inline(always)]
	pub fn last(&self) -> Option<Node> {
		self.children().last().copied()
	}

	#[inline(always)]
	pub fn parent(&self) -> Option<Node> {
		let data = self.data();
		data.parent.get()
	}

	#[inline(always)]
	pub fn index(&self) -> usize {
		let data = self.data();
		data.index.load(Order::Relaxed)
	}

	#[inline(always)]
	pub fn next(&self) -> Option<Node> {
		let next = self.index() + 1;
		self.parent().and_then(|x| x.node(next))
	}

	#[inline(always)]
	pub fn prev(&self) -> Option<Node> {
		let index = self.index();
		if index == 0 {
			return None;
		}

		let prev = index - 1;
		self.parent().and_then(|x| x.node(prev))
	}

	pub fn insert_nodes<T: IntoIterator<Item = Node>>(&self, at: usize, nodes: T)
	where
		T::IntoIter: ExactSizeIterator,
	{
		let data = self.data();

		let nodes = nodes.into_iter();
		let len = nodes.len();
		if len == 0 {
			return;
		}

		let children = data.children.items();
		data.children.insert_and_set(children, at, nodes);

		let children = data.children.items();
		for (n, it) in children.iter().enumerate().skip(at) {
			it.set_parent(Some(*self), n);
		}
	}

	pub fn push_node(self, node: Node) {
		self.append_nodes(std::iter::once(node));
	}

	pub fn append_nodes<T: IntoIterator<Item = Node>>(&self, nodes: T)
	where
		T::IntoIter: ExactSizeIterator,
	{
		self.insert_nodes(self.len(), nodes)
	}

	pub fn remove(self) -> bool {
		if let Some(parent) = self.parent() {
			let data = self.data();
			let idx = self.index();
			parent.remove_nodes(idx..idx + 1);
			data.index.store(0, Order::Relaxed);
			true
		} else {
			false
		}
	}

	pub fn remove_nodes<T: RangeBounds<usize>>(self, range: T) -> &'static [Node] {
		let data = self.data();
		let children = data.children.items();
		let sta = match range.start_bound() {
			std::ops::Bound::Included(&n) => n,
			std::ops::Bound::Excluded(&n) => n + 1,
			std::ops::Bound::Unbounded => 0,
		};

		let removed = data.children.remove_and_set(children, range);
		if removed.len() == 0 {
			return removed;
		}

		for it in removed {
			// NOTE: don't touch the index since it may be required by an operator
			let it = it.data();
			it.parent.set(None);
		}

		let nodes = data.children.items();
		for n in sta..nodes.len() {
			let it = nodes[n].data();
			it.index.store(n, Order::Relaxed);
		}

		return removed;
	}

	#[inline(always)]
	fn data(&self) -> &'static NodeData {
		unsafe { self.data.as_ref() }
	}

	#[inline(always)]
	fn set_parent(&self, parent: Option<Node>, index: usize) {
		let data = self.data();
		data.index.store(index, Order::Relaxed);
		data.parent.set(parent);
	}
}

#[derive(Default)]
pub(crate) struct NodeCell {
	data: AtomicPtr<NodeData>,
}

impl NodeCell {
	#[inline(always)]
	pub fn get(&self) -> Option<Node> {
		let data = self.data.load(Order::Relaxed);
		if data.is_null() {
			None
		} else {
			let data = unsafe { NonNull::new_unchecked(data) };
			Some(Node { data })
		}
	}

	#[inline(always)]
	pub fn set(&self, node: Option<Node>) {
		let ptr = if let Some(node) = node {
			node.data.as_ptr()
		} else {
			std::ptr::null_mut()
		};
		self.data.store(ptr, Order::Relaxed);
	}
}

impl Node {
	pub fn write_pos(&self, f: &mut Writer, label: &str) -> Result<()> {
		let span = self.span();
		if !span.is_empty() {
			write!(f, "{label}{span}")?;
			if let Some(text) = span.display_text(0) {
				write!(f, "    # {text}")?;
			}
		}
		Ok(())
	}

	pub fn write_with_pos(&self, f: &mut Writer) -> Result<()> {
		self.write(f)?;
		self.write_pos(f, "\n… at ")?;
		Ok(())
	}
}

impl Writable for Node {
	fn write(&self, f: &mut Writer) -> Result<()> {
		let value = self.value();
		if value.process(Message::Output(*self, f))? {
			return Ok(());
		}

		value.write(f)?;
		let nodes = self.children();
		if nodes.len() > 0 {
			let mut f = f.indented();
			write!(f, " {{")?;
			for (n, it) in nodes.iter().enumerate() {
				write!(f, "\n")?;
				write!(f, "[{n}] ")?;
				it.write_with_pos(&mut f)?;
			}
			f.dedent();
			write!(f, "\n}}")?;
		}

		Ok(())
	}
}

formatted!(Node);

impl HasSpan for Node {
	#[inline(always)]
	fn span(&self) -> Span {
		let data = self.data();
		data.span
	}
}
