use super::*;

pub struct Bindings {
	bindings: Init<BindTable>,
}

pub struct SymbolBindings {
	bindings: Init<Table<Symbol, BindTable>>,
}

impl SymbolBindings {
	pub const fn new() -> Self {
		Self {
			bindings: Init::default(),
		}
	}

	#[inline(always)]
	pub fn get<T: Into<Symbol>>(&self, symbol: T) -> &'static BindTable {
		let symbol = symbol.into();
		let bindings = self.bindings.get();
		bindings.get(&symbol)
	}

	#[inline(always)]
	pub fn add(&self, symbol: &Symbol, node: Node) {
		self.get(*symbol).add_node(node);
	}
}

#[derive(Default)]
pub struct BindTable {
	by_source: Table<Source, BindingMap>,
	globals: RwLock<Vec<&'static dyn GlobalInit>>,
}

impl Bindings {
	pub const fn new() -> Self {
		Self {
			bindings: Init::default(),
		}
	}

	#[inline(always)]
	pub fn get(&self) -> &'static BindTable {
		self.bindings.get()
	}

	#[inline(always)]
	pub fn add(&self, node: Node) {
		self.get().add_node(node)
	}
}

impl BindTable {
	pub fn add_node(&self, node: Node) {
		let map = self.get_by_source(node.source());
		map.add_node(node);
		map.queue_reindex();
	}

	pub fn add_eval<T: Eval>(&self, eval: T) {
		self.add_global_init(Global::new(eval));
	}

	pub fn add_global_init<T: GlobalInit>(&self, eval: T) {
		let eval = Arena::get().store(eval);
		let mut globals = self.globals.write().unwrap();
		globals.push(eval);
	}

	pub fn set_span<T: Eval>(&self, span: Span, eval: T) {
		let map = self.get_by_source(span.source());
		let eval = Arena::get().store(eval);
		let bind = Bind {
			eval,
			span,
			parent: map,
		};
		map.add_bind(bind);
		map.queue_reindex();
	}

	fn get_by_source(&self, src: Source) -> &'static BindingMap {
		self.by_source.get_or_init_ref(&src, |arena, src| {
			let map = arena.store(BindingMap::default());
			let globals = self.globals.read().unwrap();
			let span = src.span();
			for &global in globals.iter() {
				map.add_bind(Bind {
					eval: global.init_eval(*src),
					span,
					parent: map,
				});
			}
			map
		})
	}
}

#[derive(Copy, Clone)]
pub(crate) struct Bind {
	eval: &'static dyn Eval,
	span: Span,
	parent: &'static BindingMap,
}

impl Bind {
	pub fn execute(self) -> Result<()> {
		let parent = self.parent;
		let new_nodes = {
			let mut nodes = parent.new_nodes.lock().unwrap();
			std::mem::take(&mut *nodes)
		};

		let mut nodes = parent.nodes.lock().unwrap();
		if new_nodes.len() > 0 {
			nodes.extend(new_nodes);
			nodes.sort_by_key(|x| x.span());
		}

		let sta = self.span.sta();
		let end = self.span.end();

		let sta_index = nodes.partition_point(|x| x.offset() < sta);
		let end_index = nodes[sta_index..].partition_point(|x| x.offset() < end) + sta_index;

		self.parent.add_done(self);

		self.eval.execute(&nodes[sta_index..end_index])?;

		let mut cur = sta_index;
		for index in sta_index..end_index {
			let node = nodes[index];
			if !node.done() {
				nodes[cur] = node;
				cur += 1;
			}
		}

		nodes.truncate(cur);

		Ok(())
	}
}

impl Eq for Bind {}

impl PartialEq for Bind {
	fn eq(&self, other: &Self) -> bool {
		self.eval as *const _ == other.eval as *const _
			&& self.span == other.span
			&& self.parent as *const _ == other.parent as *const _
	}
}

impl Ord for Bind {
	fn cmp(&self, other: &Self) -> Ordering {
		let pa = self.eval.precedence();
		let pb = other.eval.precedence();
		pa.cmp(&pb)
			.then_with(|| self.span.source().cmp(&other.span.source()))
			.then_with(|| {
				let la = self.span.len();
				let lb = other.span.len();
				la.cmp(&lb)
			})
			.then_with(|| self.span.sta().cmp(&other.span.sta()))
			.then_with(|| self.span.end().cmp(&other.span.end()))
	}
}

impl PartialOrd for Bind {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Bind {
	pub fn overlaps(&self, sta: usize, end: usize) -> bool {
		self.span.sta() < end && sta < self.span.end()
	}
}

impl Debug for Bind {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let span = self.span;
		let eval = self.eval;
		write!(f, "Bind({eval:?}) @ {span}")
	}
}

pub(crate) struct BindingMap {
	nodes: Mutex<Vec<Node>>,
	new_nodes: Mutex<Vec<Node>>,
	pending: Mutex<Vec<Bind>>,
	complete: Mutex<Vec<Bind>>,
	pending_reindex: AtomicBool,
	changed_sta: AtomicUsize,
	changed_end: AtomicUsize,
}

impl BindingMap {
	pub fn reindex(&self) {
		let queue = Queue::get();
		let mut pending = self.pending.lock().unwrap();

		let changed_sta = self.changed_sta.load(Order::Relaxed);
		let changed_end = self.changed_end.load(Order::Relaxed);

		self.changed_sta.store(usize::MAX, Order::Relaxed);
		self.changed_end.store(0, Order::Relaxed);
		self.pending_reindex.store(false, Order::Relaxed);

		for it in pending.drain(..) {
			queue.queue_bind(it);
		}

		if changed_end < changed_sta {
			return;
		}

		let mut complete = self.complete.lock().unwrap();
		let mut cur = 0;
		for pos in 0..complete.len() {
			let it = complete[pos];
			if it.overlaps(changed_sta, changed_end) {
				queue.queue_bind(it);
			} else {
				complete[cur] = it;
				cur += 1;
			}
		}
		complete.truncate(cur);
	}

	pub fn add_done(&self, bind: Bind) {
		let mut complete = self.complete.lock().unwrap();
		complete.push(bind);
	}

	fn add_node(&self, node: Node) {
		let mut nodes = self.new_nodes.lock().unwrap();
		nodes.push(node);

		let span = node.span();
		self.changed_sta.fetch_min(span.sta(), Order::Relaxed);
		self.changed_end.fetch_max(span.end(), Order::Relaxed);
	}

	fn add_bind(&self, bind: Bind) {
		let mut pending = self.pending.lock().unwrap();
		pending.push(bind);
	}

	fn queue_reindex(&'static self) {
		if self
			.pending_reindex
			.compare_exchange(false, true, Order::Relaxed, Order::Relaxed)
			.is_ok()
		{
			Queue::get().queue_reindex(self);
		}
	}
}

impl Default for BindingMap {
	fn default() -> Self {
		Self {
			nodes: Default::default(),
			new_nodes: Default::default(),
			pending: Default::default(),
			complete: Default::default(),
			pending_reindex: false.into(),
			changed_sta: usize::MAX.into(),
			changed_end: 0.into(),
		}
	}
}
