use std::mem::{discriminant, Discriminant};

use super::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Match<'a> {
	Exact(Value<'a>),
	KindOf(Discriminant<Value<'a>>),
	Token(Discriminant<Token>),
}

impl<'a> Match<'a> {
	pub fn exact(value: Value<'a>) -> Self {
		Self::Exact(value)
	}

	pub fn symbol<T: Into<Symbol>>(symbol: T) -> Self {
		Self::Exact(Value::Token(Token::Symbol(symbol.into())))
	}

	pub fn word<T: Into<Symbol>>(word: T) -> Self {
		Self::Exact(Value::Token(Token::Word(word.into())))
	}

	pub fn indent() -> Self {
		Self::kind_of(Value::Indent(true))
	}

	pub fn token_kind(token: Token) -> Self {
		Self::Token(discriminant(&token))
	}

	pub fn token(token: Token) -> Self {
		Self::Exact(Value::Token(token))
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
		let key = node.key();
		match self {
			Match::Exact(v) => key == v,
			Match::KindOf(v) => discriminant(&key) == v,
			Match::Token(v) => {
				if let Value::Token(token) = key {
					v == discriminant(&token)
				} else {
					false
				}
			}
		}
	}

	fn key(&self) -> Key<'a> {
		match self {
			&Match::Exact(v) => Key::Exact(v),
			&Match::KindOf(v) => Key::KindOf(v),
			&Match::Token(v) => Key::Token(v),
		}
	}
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
enum Key<'a> {
	#[default]
	None,
	Exact(Value<'a>),
	KindOf(Discriminant<Value<'a>>),
	Token(Discriminant<Token>),
}

const MAX_KEYS: usize = 3;

impl<'a> Key<'a> {
	pub fn for_value(v: Value<'a>) -> [Key<'a>; MAX_KEYS] {
		match v {
			Value::None => Default::default(),
			Value::Unit => Self::as_kind(v),
			Value::Bool(_) => Self::as_kind(v),
			Value::SInt(_) => Self::as_kind(v),
			Value::UInt(_) => Self::as_kind(v),
			Value::Source(_) => Self::as_kind(v),
			Value::Module(_) => Self::as_kind(v),
			Value::Group { .. } => Self::as_kind(v),
			Value::Sequence { .. } => Self::as_kind(v),
			Value::Print => Self::as_kind(v),
			Value::Let(_) => Self::as_kind(v),
			Value::Var(_) => Self::as_kind(v),
			Value::Indent(_) => Self::as_kind(v),
			Value::If { .. } => Self::as_kind(v),
			Value::ElseIf { .. } => Self::as_kind(v),
			Value::Else { .. } => Self::as_kind(v),
			Value::For { .. } => Self::as_kind(v),

			Value::Str(_) => Self::as_value(v),
			Value::BinaryOp(_) => Self::as_value(v),

			Value::Token(token) => Self::three(Self::Exact(v), Self::Token(discriminant(&token)), Self::kind_of(v)),
		}
	}

	fn as_kind<const N: usize>(v: Value<'a>) -> [Self; N] {
		Self::one(Self::kind_of(v))
	}

	fn as_value<const N: usize>(v: Value<'a>) -> [Self; N] {
		Self::two(Self::Exact(v), Self::kind_of(v))
	}

	fn kind_of(v: Value<'a>) -> Self {
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

	fn three<const N: usize>(a: Self, b: Self, c: Self) -> [Self; N] {
		let mut out: [Self; N] = [Default::default(); N];
		out[0] = a;
		out[1] = b;
		out[2] = c;
		out
	}
}

pub struct BoundNodes<'a> {
	sta: usize,
	end: usize,
	src: Source<'a>,
	value: &'a BoundValue<'a>,
	nodes: Vec<Node<'a>>,
	sorted_by_parent: bool,
}

impl<'a> BoundNodes<'a> {
	pub fn nodes(&self) -> &[Node<'a>] {
		self.nodes.as_slice()
	}

	pub fn eval(&self) -> &'a dyn Evaluator<'a> {
		self.value.val
	}

	pub fn order(&self) -> Precedence {
		self.value.ord
	}

	pub fn src(&self) -> Source<'a> {
		self.src
	}

	pub fn pos(&self) -> usize {
		self.sta
	}

	pub fn end(&self) -> usize {
		self.end
	}

	pub fn len(&self) -> usize {
		self.end - self.sta
	}

	pub fn by_parent<'b>(&'b mut self) -> ParentNodes<'a, 'b> {
		if !self.sorted_by_parent {
			self.nodes.sort_by_key(|node| (node.parent(), node.index(), *node));
			self.sorted_by_parent = true;
		}
		let parent = ParentNodes {
			nodes: &self.nodes,
			index: 0,
		};
		parent
	}
}

pub struct ParentNodes<'a, 'b> {
	nodes: &'b [Node<'a>],
	index: usize,
}

impl<'a, 'b> Iterator for ParentNodes<'a, 'b> {
	type Item = (Node<'a>, &'b [Node<'a>]);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			break if let Some(&node) = self.nodes.get(self.index) {
				let nodes = &self.nodes[self.index..];
				let count = nodes.partition_point(|x| x.parent() == node.parent());
				let nodes = &nodes[..count];
				debug_assert!(nodes.len() >= 1);

				self.index += nodes.len();
				if let Some(parent) = node.parent() {
					Some((parent, nodes))
				} else {
					continue;
				}
			} else {
				None
			};
		}
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

	pub fn root_nodes(&self, include_silent: bool) -> Vec<Node<'a>> {
		let by_source = &self.by_source.read().unwrap();
		let mut root_nodes = HashSet::new();
		for (_, by_src) in by_source.iter() {
			let by_key = by_src.by_key.read().unwrap();
			for it in by_key.values() {
				let nodes = it.nodes.borrow();
				for node in nodes.iter() {
					let mut cur = *node;
					while let Some(par) = cur.parent() {
						cur = par;
					}
					if include_silent || !cur.is_silent() {
						root_nodes.insert(cur);
					}
				}
			}
		}

		let mut root_nodes = root_nodes.into_iter().collect::<Vec<_>>();
		root_nodes.sort_by_key(|node| (node.span(), node.value()));
		root_nodes
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

	pub fn match_at(&self, src: Source<'a>, range: std::ops::Range<usize>, pattern: Match<'a>) -> Binding<'a> {
		assert!(range.end >= range.start);
		self.by_source(src).get_binding(pattern, range.start, range.end)
	}

	pub fn get_next(&self) -> Option<BoundNodes<'a>> {
		let mut heap = self.segment_heap.write().unwrap();
		while let Some(item) = heap.shift() {
			let src = item.parent.table.source;
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
				let end = std::cmp::min(value.end, src.len());
				return Some(BoundNodes {
					sta,
					end,
					src,
					value,
					nodes,
					sorted_by_parent: false,
				});
			} else if DEBUG_EVAL && DEBUG_EVAL_EMPTY {
				let val = item.val();
				let sta = val.sta;
				let end = val.end;
				let src = item.parent.table.source;
				println!(">>> EMPTY BINDING: {:?} ({src} from {sta} to {end})", val.val);
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
			ord: Precedence::Last,
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
	ord: Precedence,
}

impl<'a> Binding<'a> {
	pub fn bind<T: Evaluator<'a> + 'a>(&self, eval: T) {
		if self.sta >= self.end {
			return;
		}
		let bindings = self.table.by_pattern(self.pattern);
		let value = self.table.ctx.store(BoundValue {
			sta: self.sta,
			end: self.end,
			val: self.table.ctx.store(eval),
			ord: self.ord,
		});
		bindings.bind(self.table.ctx, value);
	}

	pub fn with_precedence(mut self, ord: Precedence) -> Self {
		self.ord = ord;
		self
	}
}

struct BoundValue<'a> {
	sta: usize,
	end: usize,
	val: &'a dyn Evaluator<'a>,
	ord: Precedence,
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
