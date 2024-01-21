use super::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Node {
	data: NonNull<NodeData>,
}

struct NodeData {
	span: Span,
	done: AtomicBool,
	value: ValueCell,
}

impl Node {
	pub fn new(value: Value, span: Span) -> Self {
		let data = Arena::get().alloc(NodeData {
			span,
			done: false.into(),
			value: ValueCell::new(value),
		});
		let node = Self { data };
		value.get().bind(node);
		node
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
	pub fn span(&self) -> Span {
		let data = self.data();
		data.span
	}

	#[inline(always)]
	pub fn value(&self) -> Value {
		let data = self.data();
		data.value.get()
	}

	#[inline(always)]
	pub fn set_value(&self, value: Value) {
		let data = self.data();
		data.value.set(value);
		value.get().bind(*self);
	}

	#[inline(always)]
	fn data(&self) -> &'static NodeData {
		unsafe { self.data.as_ref() }
	}
}
