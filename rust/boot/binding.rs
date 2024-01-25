use super::*;

pub struct Bindings {
	bindings: Init<BindTable>,
}

#[derive(Default)]
struct BindTable {
	by_source: Table<Source, BindingMap>,
	globals: RwLock<Vec<&'static dyn Eval>>,
}

impl Bindings {
	pub const fn new() -> Self {
		Self {
			bindings: Init::default(),
		}
	}

	pub fn add_node(&self, node: Node) {
		let map = self.get_by_source(node.source());
		map.add_node(node);
		map.queue_reindex();
	}

	pub fn set_init<T: Eval>(&self, eval: T) {
		let bindings = self.bindings.get();
		let eval = Arena::get().store(eval);
		let mut globals = bindings.globals.write().unwrap();
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
		let bindings = self.bindings.get();
		bindings.by_source.get_or_init_ref(&src, |arena, src| {
			let map = arena.store(BindingMap::default());
			let globals = bindings.globals.read().unwrap();
			let span = src.span();
			for &eval in globals.iter() {
				map.add_bind(Bind {
					eval,
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
		let nodes = {
			let (ref mut nodes, ref mut sorted) = *parent.nodes.lock().unwrap();
			if !*sorted {
				nodes.sort_by_key(|x| x.span());
				*sorted = true;
			}

			let sta = self.span.sta();
			let end = self.span.end();

			let sta_index = nodes.partition_point(|x| x.offset() < sta);
			let end_index = nodes[sta_index..].partition_point(|x| x.offset() < end) + sta_index;

			self.parent.add_done(self);

			nodes
				.drain(sta_index..end_index)
				.filter(|x| !x.done())
				.collect::<Vec<Node>>()
		};

		self.eval.execute(nodes)?;

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

pub(crate) struct BindingMap {
	nodes: Mutex<(Vec<Node>, bool)>,
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
		let changed_end = self.changed_sta.load(Order::Relaxed);

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
		let mut nodes = self.nodes.lock().unwrap();
		let (ref mut nodes, ref mut sorted) = *nodes;
		nodes.push(node);
		*sorted = false;

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
			pending: Default::default(),
			complete: Default::default(),
			pending_reindex: false.into(),
			changed_sta: usize::MAX.into(),
			changed_end: 0.into(),
		}
	}
}
