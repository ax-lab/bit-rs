use super::*;

#[derive(Default)]
pub struct Queue {
	pending_reindex: Mutex<VecDeque<&'static BindingMap>>,
	queue: Mutex<BindQueue>,
}

impl Queue {
	pub fn get() -> &'static Self {
		static QUEUE: Init<Queue> = Init::default();
		QUEUE.get()
	}

	pub fn process_next(&self) -> Result<bool> {
		let mut pending = self.pending_reindex.lock().unwrap();
		for it in pending.drain(..) {
			it.reindex();
		}
		drop(pending);

		let mut queue = self.queue.lock().unwrap();
		let len = queue.len();
		if len > 0 {
			let bind = queue.list[0];
			queue.swap(0, len - 1);
			queue.list.truncate(len - 1);
			queue.shift_down(0);
			bind.execute()?;
			Ok(true)
		} else {
			Ok(false)
		}
	}

	pub(crate) fn queue_reindex(&self, map: &'static BindingMap) {
		let mut pending = self.pending_reindex.lock().unwrap();
		pending.push_back(map);
	}

	pub(crate) fn queue_bind(&self, bind: Bind) {
		let mut queue = self.queue.lock().unwrap();
		let index = queue.list.len();
		queue.list.push(bind);
		queue.shift_up(index);
	}
}

#[derive(Default)]
struct BindQueue {
	list: Vec<Bind>,
}

impl BindQueue {
	#[inline(always)]
	pub fn len(&self) -> usize {
		self.list.len()
	}

	#[inline(always)]
	pub fn swap(&mut self, a: usize, b: usize) {
		self.list.swap(a, b);
	}
}

impl IsHeap for BindQueue {
	fn heap_len(&self) -> usize {
		self.len()
	}

	fn heap_swap(&mut self, a: usize, b: usize) {
		self.swap(a, b);
	}

	fn heap_less(&self, a: usize, b: usize) -> bool {
		let a = &self.list[a];
		let b = &self.list[b];
		a.cmp(&b).is_lt()
	}
}
