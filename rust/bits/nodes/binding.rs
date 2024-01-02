use std::mem::{discriminant, Discriminant};

use super::*;

const MAX_KEYS: usize = 2;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Match<'a> {
	Exact(Value<'a>),
	KindOf(Discriminant<Value<'a>>),
}

impl<'a> Match<'a> {
	pub fn exact(value: Value<'a>) -> Self {
		Self::Exact(value)
	}

	pub fn unit() -> Self {
		Self::kind_of(Value::Unit)
	}

	pub fn str() -> Self {
		Self::kind_of(Value::Str(""))
	}

	pub fn source() -> Self {
		Self::kind_of(Value::Source(Source::default()))
	}

	pub fn kind_of(value: Value<'a>) -> Self {
		Self::KindOf(discriminant(&value))
	}

	pub fn matches(self, node: Node<'a>) -> bool {
		match self {
			Match::Exact(v) => node.key() == v,
			Match::KindOf(v) => discriminant(&node.key()) == v,
		}
	}

	fn key(&self) -> Key<'a> {
		match self {
			&Match::Exact(v) => Key::Exact(v),
			&Match::KindOf(v) => Key::KindOf(v),
		}
	}
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
enum Key<'a> {
	#[default]
	None,
	Exact(Value<'a>),
	KindOf(Discriminant<Value<'a>>),
}

impl<'a> Key<'a> {
	pub fn for_value(value: Value<'a>) -> [Key<'a>; MAX_KEYS] {
		match value {
			Value::None => Default::default(),
			Value::Unit => Self::one(Self::kind(value)),
			Value::Bool(_) => Self::one(Self::kind(value)),
			Value::Str(_) => Self::two(Self::Exact(value), Self::kind(value)),
			Value::SInt(_) => Self::one(Self::kind(value)),
			Value::UInt(_) => Self::one(Self::kind(value)),
			Value::Source(_) => Self::one(Self::kind(value)),
		}
	}

	fn kind(v: Value<'a>) -> Self {
		Self::KindOf(discriminant(&v))
	}

	fn one<const N: usize>(key: Self) -> [Self; N] {
		let mut out: [Self; N] = [Default::default(); N];
		out[0] = key;
		out
	}

	fn two<const N: usize>(a: Self, b: Self) -> [Self; N] {
		let mut out: [Self; N] = [Default::default(); N];
		out[0] = a;
		out[1] = b;
		out
	}
}

pub struct BoundNodes<'a> {
	span: Span<'a>,
	value: &'a BoundValue<'a>,
	nodes: Vec<Node<'a>>,
}

impl<'a> BoundNodes<'a> {
	pub fn nodes(&self) -> &[Node<'a>] {
		self.nodes.as_slice()
	}

	pub fn eval(&self) -> &'a dyn Evaluator<'a> {
		self.value.val
	}

	pub fn order(&self) -> Value<'a> {
		self.value.ord
	}

	pub fn span(&self) -> Span<'a> {
		self.span
	}
}

pub struct Bindings<'a> {
	ctx: ContextRef<'a>,
	by_source: RwLock<HashMap<Source<'a>, &'a BySource<'a>>>,
	segment_heap: RwLock<SegmentHeap<'a>>,
}

impl<'a> Bindings<'a> {
	pub fn new(ctx: ContextRef<'a>) -> Self {
		Self {
			ctx,
			by_source: Default::default(),
			segment_heap: Default::default(),
		}
	}

	pub fn add(&self, key: Value<'a>, node: Node<'a>) {
		if key == Value::None {
			return;
		}

		let mut heap = self.segment_heap.write().unwrap();
		let src = node.span().src();
		node.keep_alive();
		if src != Source::default() {
			self.by_source(src).add_node(key, node, &mut heap);
		}
		self.by_source(Source::default()).add_node(key, node, &mut heap);
	}

	pub fn match_any(&self, pattern: Match<'a>) -> Binding<'a> {
		self.by_source(Source::default()).get_binding(pattern, 0, usize::MAX)
	}

	pub fn match_at(&self, at: Span<'a>, pattern: Match<'a>) -> Binding<'a> {
		self.by_source(at.src()).get_binding(pattern, at.pos(), at.end())
	}

	pub fn get_next(&self) -> Option<BoundNodes<'a>> {
		let mut heap = self.segment_heap.write().unwrap();
		while let Some(item) = heap.shift() {
			let source = item.parent.table.source;
			let value = item.val();
			let mut items = vec![item];
			let mut done_segments = item.parent.table.done_segments.borrow_mut();
			done_segments.push(item);
			while let Some(&next) = heap.queue.get(0) {
				if item.same_binding_as(next) {
					let next = heap.shift().unwrap();
					items.push(next);
					done_segments.push(next);
				} else {
					break;
				}
			}

			let mut nodes = Vec::new();
			for &it in items.iter() {
				it.get_nodes(&mut nodes);
			}

			if nodes.len() > 0 {
				let sta = value.sta;
				let end = std::cmp::min(value.end, source.len());
				let span = source.range(sta..end);
				return Some(BoundNodes { span, value, nodes });
			}
		}

		None
	}

	pub fn get_pending(&self) -> Vec<Node<'a>> {
		let mut output = Vec::new();
		let sources = self.by_source.read().unwrap();
		for it in sources.values() {
			let by_key = it.by_key.read().unwrap();
			for tb in by_key.values() {
				let pending = tb.nodes.borrow();
				let pending = pending.iter().copied();
				let pending = pending.filter(|node| {
					if !node.is_done() {
						node.flag_done();
						true
					} else {
						false
					}
				});
				output.extend(pending);
			}
		}
		output.sort();
		output
	}

	fn by_source(&self, src: Source<'a>) -> &'a BySource<'a> {
		if let Some(entry) = self.by_source.read().unwrap().get(&src) {
			return entry;
		}

		let mut entries = self.by_source.write().unwrap();
		let entry = entries
			.entry(src)
			.or_insert_with(|| self.ctx.store(BySource::new(self.ctx, src)));
		*entry
	}
}

struct BySource<'a> {
	ctx: ContextRef<'a>,
	source: Source<'a>,
	by_key: RwLock<HashMap<Key<'a>, &'a BindTable<'a>>>,
}

impl<'a> BySource<'a> {
	pub fn new(ctx: ContextRef<'a>, source: Source<'a>) -> Self {
		Self {
			ctx,
			source,
			by_key: Default::default(),
		}
	}

	pub fn add_node(&self, key: Value<'a>, node: Node<'a>, heap: &mut SegmentHeap<'a>) {
		for key in Key::for_value(key) {
			if key != Key::None {
				let entries = self.by_key(key);
				entries.add_node(node, heap);
			}
		}
	}

	pub fn get_binding(&self, pattern: Match<'a>, sta: usize, end: usize) -> Binding<'a> {
		let key = pattern.key();
		self.by_key(key).get_pattern(pattern, sta, end)
	}

	fn by_key(&self, key: Key<'a>) -> &'a BindTable<'a> {
		if let Some(entry) = self.by_key.read().unwrap().get(&key) {
			return entry;
		}

		let mut entries = self.by_key.write().unwrap();
		let entry = entries
			.entry(key)
			.or_insert_with(|| self.ctx.store(BindTable::new(self.ctx, self.source)));
		*entry
	}
}

struct BindTable<'a> {
	ctx: ContextRef<'a>,
	source: Source<'a>,
	sorted: Cell<bool>,
	nodes: RefCell<Vec<Node<'a>>>,
	patterns: RefCell<HashMap<Match<'a>, &'a PatternBindings<'a>>>,
	done_segments: RefCell<Vec<&'a BoundSegment<'a>>>,
}

impl<'a> BindTable<'a> {
	pub fn new(ctx: ContextRef<'a>, source: Source<'a>) -> Self {
		Self {
			ctx,
			source,
			sorted: true.into(),
			nodes: Default::default(),
			patterns: Default::default(),
			done_segments: Default::default(),
		}
	}

	pub fn add_node(&self, node: Node<'a>, heap: &mut SegmentHeap<'a>) {
		let mut nodes = self.nodes.borrow_mut();
		let sorted = self.sorted.get() && node.pos() >= nodes.last().map(|x| x.pos()).unwrap_or_default();
		nodes.push(node);
		self.sorted.set(sorted);

		// requeue any processed segments since there is a new node
		let mut done_segments = self.done_segments.borrow_mut();
		let node_pos = node.pos();
		let mut cur = 0;
		for i in 0..done_segments.len() {
			let seg = done_segments[i];
			if seg.sta() <= node_pos && node_pos < seg.end() {
				heap.enqueue_or_fix(seg);
			} else {
				done_segments[cur] = seg;
				cur += 1;
			}
		}
		done_segments.truncate(cur);
	}

	pub fn sorted_nodes(&self) -> Ref<[Node<'a>]> {
		if !self.sorted.get() {
			let mut nodes = self.nodes.borrow_mut();
			nodes.sort_by_key(|x| x.pos());
			self.sorted.set(true);
		}

		let out = self.nodes.borrow();
		Ref::map(out, |x| x.as_slice())
	}

	pub fn get_pattern(&'a self, pattern: Match<'a>, sta: usize, end: usize) -> Binding<'a> {
		Binding {
			pattern,
			table: self,
			sta,
			end,
			ord: Value::SInt(0),
		}
	}

	fn by_pattern(&'a self, pattern: Match<'a>) -> &'a PatternBindings<'a> {
		if let Some(entry) = self.patterns.borrow().get(&pattern) {
			return entry;
		}

		let mut entries = self.patterns.borrow_mut();
		let entry = entries.entry(pattern).or_insert_with(|| {
			self.ctx.store(PatternBindings {
				table: self,
				pattern,
				segments: Default::default(),
			})
		});
		*entry
	}
}

pub struct Binding<'a> {
	pattern: Match<'a>,
	table: &'a BindTable<'a>,
	sta: usize,
	end: usize,
	ord: Value<'a>,
}

impl<'a> Binding<'a> {
	pub fn bind<T: Evaluator<'a> + 'a>(&self, eval: T) {
		let bindings = self.table.by_pattern(self.pattern);
		let value = self.table.ctx.store(BoundValue {
			sta: self.sta,
			end: self.end,
			val: self.table.ctx.store(eval),
			ord: self.ord,
		});
		bindings.bind(self.table.ctx, value);
	}

	pub fn with_precedence(mut self, ord: Value<'a>) -> Self {
		self.ord = ord;
		self
	}
}

struct BoundValue<'a> {
	sta: usize,
	end: usize,
	val: &'a dyn Evaluator<'a>,
	ord: Value<'a>,
}

struct PatternBindings<'a> {
	table: &'a BindTable<'a>,
	pattern: Match<'a>,
	segments: RefCell<Vec<&'a BoundSegment<'a>>>,
}

struct BoundSegment<'a> {
	parent: &'a PatternBindings<'a>,
	sta: Cell<usize>,
	end: Cell<usize>,
	val: Cell<&'a BoundValue<'a>>,
	queue_pos: Cell<usize>,
}

impl<'a> BoundSegment<'a> {
	pub fn same_binding_as(&self, other: &Self) -> bool {
		self.parent as *const _ == other.parent as *const _ && self.val() as *const _ == other.val() as *const _
	}

	pub fn get_nodes(&self, output: &mut Vec<Node<'a>>) {
		let tb = self.parent.table;
		let nodes = tb.sorted_nodes();
		let sta = self.sta.get();
		let end = self.end.get();
		let sta_index = nodes.partition_point(|node| node.pos() < sta);
		let end_index = sta_index + nodes[sta_index..].partition_point(|node| node.pos() < end);
		let nodes = nodes[sta_index..end_index].iter().copied();
		let nodes = nodes.filter(|node: &Node<'a>| {
			if !node.is_done() && self.parent.pattern.matches(*node) {
				node.flag_done();
				true
			} else {
				false
			}
		});
		output.extend(nodes);
	}
}

impl<'a> Eq for BoundSegment<'a> {}

impl<'a> PartialEq for BoundSegment<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.same_binding_as(other) && self.sta == other.sta && self.end == other.end
	}
}

impl<'a> Ord for BoundSegment<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		let a = self.val();
		let b = other.val();
		a.ord
			.cmp(&b.ord)
			.then_with(|| a.sta.cmp(&b.sta))
			.then_with(|| b.end.cmp(&a.end))
	}
}

impl<'a> PartialOrd for BoundSegment<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> BoundSegment<'a> {
	pub fn val(&self) -> &'a BoundValue<'a> {
		self.val.get()
	}

	pub fn set_val(&self, val: &'a BoundValue<'a>) {
		self.val.set(val);
	}

	pub fn sta(&self) -> usize {
		self.sta.get()
	}

	pub fn end(&self) -> usize {
		self.end.get()
	}

	pub fn queue_pos(&self) -> usize {
		self.queue_pos.get()
	}

	pub fn set_range(&self, sta: usize, end: usize) {
		self.sta.set(sta);
		self.end.set(end);
	}
}

impl<'a> BoundValue<'a> {
	pub fn contains_range(&self, sta: usize, end: usize) -> bool {
		sta >= self.sta && sta < self.end && end <= self.end
	}
}

impl<'a> PatternBindings<'a> {
	fn bind(&'a self, ctx: ContextRef<'a>, val: &'a BoundValue<'a>) {
		let sta = val.sta;
		let end = val.end;
		let mut heap = ctx.nodes().bindings.segment_heap.write().unwrap();
		let heap = &mut heap;

		struct Segment<'a> {
			sta: usize,
			end: usize,
			val: &'a BoundValue<'a>,
			queue: bool,
		}

		let create_segment = |heap: &mut SegmentHeap<'a>, seg: Segment<'a>| {
			let segment = ctx.store(BoundSegment {
				parent: self,
				sta: seg.sta.into(),
				end: seg.end.into(),
				queue_pos: NOT_QUEUED.into(),
				val: seg.val.into(),
			});
			if seg.queue {
				heap.enqueue_or_fix(segment);
			}
			segment
		};

		let mut segments = self.segments.borrow_mut();
		let insert_pos = segments.partition_point(|&seg| seg.end() <= sta);

		if insert_pos >= segments.len() {
			segments.push(create_segment(
				heap,
				Segment {
					sta,
					end,
					val,
					queue: true,
				},
			));
		} else {
			let mut sta = sta;
			let mut cur_idx = insert_pos;

			while cur_idx < segments.len() && sta < end {
				let cur_seg = segments[cur_idx];
				let cur_sta = cur_seg.sta();
				let cur_end = cur_seg.end();

				let gap_before = cur_sta > sta;
				if gap_before {
					let seg_end = std::cmp::min(end, cur_sta);
					segments.insert(
						cur_idx,
						create_segment(
							heap,
							Segment {
								sta,
								end: seg_end,
								val,
								queue: true,
							},
						),
					);
					cur_idx += 1;
					sta = seg_end;
					continue;
				}

				// a binding is only overwritten by a more specific or equal binding
				let bind_is_more_specific = cur_seg.val().contains_range(val.sta, val.end);
				if bind_is_more_specific {
					let keep_prefix = sta > cur_sta;
					let keep_suffix = end < cur_end;
					let is_queued = cur_seg.queue_pos() != NOT_QUEUED;
					if keep_prefix {
						segments.insert(
							cur_idx,
							create_segment(
								heap,
								Segment {
									val: cur_seg.val(),
									sta: cur_sta,
									end: sta,
									queue: is_queued,
								},
							),
						);
						cur_idx += 1;
					}

					if keep_suffix {
						cur_idx += 1;
						segments.insert(
							cur_idx,
							create_segment(
								heap,
								Segment {
									val: cur_seg.val(),
									sta: end,
									end: cur_end,
									queue: is_queued,
								},
							),
						);
					}

					let new_end = std::cmp::min(end, cur_end);
					cur_seg.set_range(sta, new_end);
					cur_seg.set_val(val);
					heap.enqueue_or_fix(cur_seg);
				}

				sta = cur_end;
				cur_idx += 1;
			}

			// suffix
			if sta < end {
				segments.insert(
					cur_idx,
					create_segment(
						heap,
						Segment {
							val,
							sta,
							end,
							queue: true,
						},
					),
				);
			}
		}
	}
}

#[derive(Default)]
struct SegmentHeap<'a> {
	queue: Vec<&'a BoundSegment<'a>>,
}

impl<'a> IsHeap for SegmentHeap<'a> {
	fn heap_len(&self) -> usize {
		self.check_table();
		self.queue.len()
	}

	fn heap_less(&self, a: usize, b: usize) -> bool {
		self.check_table();
		let a = self.queue[a];
		let b = self.queue[b];
		a.cmp(&b).is_le()
	}

	fn heap_swap(&mut self, a: usize, b: usize) {
		self.check_table();
		self.queue.swap(a, b);
		let sa = self.queue[a];
		let sb = self.queue[b];
		sa.queue_pos.set(a);
		sb.queue_pos.set(b);
		self.check_table();
	}
}

impl<'a> SegmentHeap<'a> {
	pub fn shift(&mut self) -> Option<&'a BoundSegment<'a>> {
		let len = self.heap_len();
		if len == 0 {
			return None;
		}

		self.heap_swap(0, len - 1);
		let next = self.queue.pop().unwrap();
		next.queue_pos.set(NOT_QUEUED);
		self.shift_down(0);

		Some(next)
	}

	pub fn enqueue_or_fix(&mut self, segment: &'a BoundSegment<'a>) {
		let cur_pos = segment.queue_pos();
		if cur_pos != NOT_QUEUED {
			self.fix(cur_pos);
			return;
		}

		let queue_pos = self.heap_len();
		self.queue.push(segment);
		segment.queue_pos.set(queue_pos);
		self.shift_up(queue_pos);
	}

	#[allow(unused)]
	fn check_table(&self) {
		for i in 0..self.queue.len() {
			assert!(self.queue[i].queue_pos() == i);
		}
	}

	#[allow(unused)]
	fn check_heap(&self) {
		self.check_pos(0);
	}

	#[allow(unused)]
	fn check_pos(&self, n: usize) {
		let lhs = Self::lhs(n);
		let rhs = Self::rhs(n);
		if lhs < self.queue.len() {
			assert!(self.heap_less(n, lhs));
			self.check_pos(lhs);
		}
		if rhs < self.queue.len() {
			assert!(self.heap_less(n, rhs));
			self.check_pos(rhs);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hash() {
		let mut map = HashMap::new();
		map.insert(discriminant(&Value::Str("a")), 123);
		assert_eq!(Some(&123), map.get(&discriminant(&Value::Str("z"))));
	}
}
