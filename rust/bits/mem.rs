use std::{alloc::Layout, ptr::NonNull};

use super::*;

pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;
pub const GB: usize = 1024 * MB;
pub const TB: usize = 1024 * GB;

pub fn print_bytes<T: Write, U: AsRef<str>>(out: &mut T, label: U, bytes: usize) -> Result<()> {
	let label = label.as_ref();
	write!(out, "{label}").raise()?;
	if bytes < KB {
		write!(out, "{bytes} bytes")
	} else if bytes < MB {
		write!(out, "{} KB", bytes / KB)
	} else if bytes < GB {
		write!(out, "{:.1} MB", (bytes as f64) / (MB as f64))
	} else {
		write!(out, "{:.2} GB", (bytes as f64) / (GB as f64))
	}
	.raise()
}

pub struct Store {
	size: usize,
	used: AtomicUsize,
	page: AtomicPtr<Page>,
	free: AtomicPtr<Free>,
	sync: Mutex<()>,
}

#[derive(Copy, Clone)]
struct Free {
	func: fn(*mut u8, usize, usize),
	addr: *mut u8,
	size: usize,
	data: usize,
	next: *mut Free,
}

impl Store {
	const DEFAULT_PAGE: usize = 256 * MB;
	const MIN_PAGE: usize = 1024;
	const MAX_ALLOC_RATIO: usize = 8;

	pub fn new() -> Self {
		Self::with_page(Self::DEFAULT_PAGE)
	}

	pub fn with_page(size: usize) -> Self {
		assert!(size > 0);
		let size = std::cmp::max(Self::MIN_PAGE, size);
		let store = Self {
			size,
			used: Default::default(),
			page: Default::default(),
			free: Default::default(),
			sync: Default::default(),
		};
		store.alloc_page();
		store
	}

	pub fn stats() -> &'static MemStat {
		Page::stats()
	}

	pub fn on_drop(&self, addr: *mut u8, size: usize, data: usize, func: fn(*mut u8, usize, usize)) {
		let free = Free {
			func,
			addr,
			size,
			data,
			next: std::ptr::null_mut(),
		};
		let free = self.store(free);
		loop {
			let next = self.free.load(MEM_ORDER);
			free.next = next;
			if self
				.free
				.compare_exchange_weak(next, free as *mut Free, MEM_ORDER, MEM_ORDER)
				.is_ok()
			{
				break;
			}
		}
	}

	pub fn store<T>(&self, value: T) -> &mut T {
		let layout = Layout::new::<T>();
		unsafe {
			let mut data = self.alloc(layout).cast::<T>();
			data.as_ptr().write(value);

			if std::mem::needs_drop::<T>() {
				self.on_drop(data.as_ptr() as *mut u8, 0, 0, |addr, _, _| {
					std::ptr::drop_in_place(addr as *mut T);
				});
			}

			data.as_mut()
		}
	}

	pub fn str<T: AsRef<str>>(&self, str: T) -> &str {
		let str = str.as_ref();
		let len = str.len();
		if len == 0 {
			return "";
		}

		let layout = Layout::from_size_align(len, 1).unwrap();
		unsafe {
			let data = self.alloc(layout).as_ptr();
			std::ptr::copy_nonoverlapping(str.as_ptr(), data, len);

			let data = std::slice::from_raw_parts(data, len);
			std::str::from_utf8_unchecked(data)
		}
	}

	pub fn slice<T: IntoIterator<Item = U>, U>(&self, elems: T) -> &mut [U]
	where
		T::IntoIter: ExactSizeIterator,
	{
		let elems = elems.into_iter();
		let count = elems.len();
		if count == 0 {
			return &mut [];
		}

		let size = count * std::mem::size_of::<U>();
		let align = std::mem::align_of::<U>();
		let layout = Layout::from_size_align(size, align).unwrap();
		unsafe {
			let data = self.alloc(layout).cast::<U>().as_ptr();
			for (n, it) in elems.enumerate() {
				data.add(n).write(it);
			}

			if std::mem::needs_drop::<U>() {
				self.on_drop(data as *mut u8, count, 0, |data, count, _| {
					let data = std::slice::from_raw_parts_mut(data as *mut U, count);
					for it in data.iter_mut().rev() {
						std::ptr::drop_in_place(it);
					}
				})
			}

			std::slice::from_raw_parts_mut(data, count)
		}
	}

	pub fn chunk<T: IntoIterator<Item = U>, U: Copy>(&self, elems: T) -> &ChunkOf<U>
	where
		T::IntoIter: ExactSizeIterator,
	{
		let elems = elems.into_iter();
		let count = elems.len();

		let elem_size = std::mem::size_of::<U>();
		let size = std::mem::size_of::<ChunkOf<U>>() + count * elem_size - elem_size;
		let align = std::mem::align_of::<ChunkOf<U>>();
		let layout = Layout::from_size_align(size, align).unwrap();
		unsafe {
			let mut chunk = self.alloc(layout).cast::<ChunkOf<U>>();
			let chunk = chunk.as_mut();
			chunk.len = count;

			let data = chunk.dat.as_mut_ptr();
			for (n, it) in elems.enumerate() {
				data.add(n).write(it);
			}
			chunk
		}
	}

	pub fn chunk_from_slice<T: Copy>(&self, elems: &[T]) -> &ChunkOf<T> {
		self.chunk(elems.iter().copied())
	}

	pub fn alloc(&self, layout: Layout) -> NonNull<u8> {
		let stats = Self::stats();
		let size = layout.size();

		// don't use the arena for large allocations
		let max_alloc = self.size / Self::MAX_ALLOC_RATIO;
		debug_assert!(max_alloc > std::mem::size_of::<Free>());
		if size > max_alloc {
			let data = Page::raw_alloc(layout);
			stats.add(size, size);
			self.on_drop(data.as_ptr(), size, layout.align(), |ptr, size, align| {
				Self::stats().sub(size, size);
				let layout = Layout::from_size_align(size, align).unwrap();
				Page::raw_free(ptr, layout);
			});
			return data;
		}

		self.used.fetch_add(size, MEM_ORDER);

		loop {
			// default optimistic path
			if let Some(data) = self.page().alloc(layout) {
				return data;
			}

			// if the above failed, allocate a new page discarding any remaining
			// space on the existing page
			let lock = self.sync.lock().unwrap();

			// there's a chance someone created a new page in between
			if let Some(data) = self.page().alloc(layout) {
				return data;
			}

			self.alloc_page();
			drop(lock);
		}
	}

	fn alloc_page(&self) {
		let page = Page::new(self.size).as_ptr();
		self.page.store(page, SyncOrder::Release);
		self.on_drop(page as *mut u8, 0, 0, |page, _, _| {
			let page = page as *mut Page;
			unsafe {
				std::ptr::drop_in_place(page);
			}
		});
	}

	fn page(&self) -> &Page {
		unsafe {
			let ptr = NonNull::new_unchecked(self.page.load(SyncOrder::Acquire));
			ptr.as_ref()
		}
	}
}

impl Drop for Store {
	fn drop(&mut self) {
		let used = self.used.load(MEM_ORDER);
		Self::stats().sub(0, used);

		let mut free = self.free.load(MEM_ORDER);
		if self
			.free
			.compare_exchange(free, std::ptr::null_mut(), MEM_ORDER, MEM_ORDER)
			.is_ok()
		{
			unsafe {
				while let Some(it) = free.as_ref() {
					free = it.next;
					(it.func)(it.addr, it.size, it.data);
				}
			}
		}
	}
}

struct Page {
	size: usize,
	used: AtomicUsize,
	data: [u8; 1],
}

impl Page {
	const NUL: u8 = 0xDC;

	fn layout(size: usize) -> Layout {
		let size = std::mem::size_of::<Page>() + size - 1;
		Layout::from_size_align(size, std::mem::align_of::<Page>()).unwrap()
	}

	/// Create a new memory page with the given size and static lifetime.
	///
	/// Once created, the [`MemPage`] is never deallocated, but the backing
	/// memory for the page can be disposed of.
	pub fn new(size: usize) -> NonNull<Self> {
		let data = unsafe {
			let layout = Self::layout(size);
			let data = Self::raw_alloc(layout);
			Self::stats().add(size, 0);

			data.as_ptr().write_bytes(Self::NUL, size);

			let data = data.cast::<Page>();
			data.as_ptr().write(Page {
				size,
				used: 0.into(),
				data: [Self::NUL],
			});
			data
		};
		data
	}

	fn free(&mut self) {
		Self::stats().sub(self.size, 0);
		let ptr = self as *mut Self as *mut u8;
		let layout = Self::layout(self.size);
		unsafe {
			ptr.write_bytes(Self::NUL, layout.size());
			std::alloc::dealloc(ptr, layout);
		}
	}

	pub fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
		let align = layout.align();
		let size = std::cmp::max(1, layout.size());
		let data = self.data.as_ptr();
		loop {
			let next = self.used.load(SyncOrder::Relaxed);
			let addr = unsafe { data.add(next) as usize };

			// align the allocation and check if it's valid
			let pos = next + (align - addr % align) % align;
			let end = pos + size;
			if end > self.size {
				return None;
			}

			// the allocation is valid, try to commit
			if self
				.used
				.compare_exchange_weak(next, end, SyncOrder::Relaxed, SyncOrder::Relaxed)
				.is_ok()
			{
				Self::stats().add(0, end - next);
				let ptr = unsafe {
					let ptr = data.add(pos) as *mut u8;
					NonNull::new_unchecked(ptr)
				};
				return Some(ptr);
			}
		}
	}

	pub fn raw_alloc(layout: Layout) -> NonNull<u8> {
		let ptr = unsafe { std::alloc::alloc(layout) };
		if let Some(ptr) = NonNull::new(ptr) {
			ptr
		} else {
			panic!("memory allocation failed: {layout:?}")
		}
	}

	pub fn raw_free(ptr: *mut u8, layout: Layout) {
		unsafe {
			std::alloc::dealloc(ptr, layout);
		}
	}

	fn stats() -> &'static MemStat {
		static STAT: MemStat = MemStat {
			size: AtomicUsize::new(0),
			used: AtomicUsize::new(0),
			max_size: AtomicUsize::new(0),
			max_used: AtomicUsize::new(0),
		};
		&STAT
	}
}

impl Drop for Page {
	fn drop(&mut self) {
		self.free();
	}
}

const MEM_ORDER: SyncOrder = SyncOrder::Relaxed;
const STAT_ORDER: SyncOrder = SyncOrder::Relaxed;

pub struct MemStat {
	size: AtomicUsize,
	used: AtomicUsize,
	max_size: AtomicUsize,
	max_used: AtomicUsize,
}

impl MemStat {
	fn add(&self, size: usize, used: usize) {
		let prev_size = self.size.fetch_add(size, STAT_ORDER);
		let prev_used = self.used.fetch_add(used, STAT_ORDER);
		self.max_size.fetch_max(prev_size + size, STAT_ORDER);
		self.max_used.fetch_max(prev_used + used, STAT_ORDER);
	}

	fn sub(&self, size: usize, used: usize) {
		self.size.fetch_sub(size, STAT_ORDER);
		self.used.fetch_sub(used, STAT_ORDER);
	}

	pub fn size(&self) -> usize {
		self.size.load(STAT_ORDER)
	}

	pub fn used(&self) -> usize {
		self.used.load(STAT_ORDER)
	}

	pub fn max_size(&self) -> usize {
		self.max_size.load(STAT_ORDER)
	}

	pub fn max_used(&self) -> usize {
		self.max_used.load(STAT_ORDER)
	}
}

pub struct ChunkOf<T> {
	len: usize,
	dat: [T; 1],
}

impl<T> ChunkOf<T> {
	pub unsafe fn from_ptr<'a>(ptr: *const u8) -> &'a Self {
		unsafe { &*(ptr as *const Self) }
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn as_ptr(&self) -> *const u8 {
		self as *const Self as *const u8
	}

	pub fn as_slice(&self) -> &[T] {
		let ptr = self.dat.as_ptr();
		unsafe { std::slice::from_raw_parts(ptr, self.len) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn basic_store() {
		let store = Store::with_page(4096);

		let value = store.store(format!("abc123"));
		assert_eq!("abc123", value);

		let value = store.str("123");
		assert_eq!("123", value);

		let value = store.str("abcdef");
		assert_eq!("abcdef", value);

		drop(store);
	}

	#[test]
	fn store_values() {
		let store = Store::with_page(1024);
		let mut values = Vec::new();
		for i in 1..2048usize {
			let item = store.store(i);
			values.push(item);
		}

		for (n, i) in values.iter().enumerate() {
			assert_eq!(**i, n + 1);
		}
	}

	#[test]
	fn store_interleaved() {
		let arena = Store::with_page(512);
		let mut v0 = Vec::new();
		let mut v1 = Vec::new();
		let mut v2 = Vec::new();
		let mut v3 = Vec::new();
		for i in 1..1024usize {
			v0.push(arena.store(i));
			v1.push(arena.store((i % 255) as u8));
			v2.push(arena.store(i as u16));
			v3.push(arena.store(()));
		}

		for (n, i) in v0.iter().enumerate() {
			let expected = n + 1;
			assert_eq!(**i, expected);
			assert_eq!(*v1[n], (expected % 255) as u8);
			assert_eq!(*v2[n], expected as u16);
		}

		let mut last = v3[0] as *const ();
		for ptr in v3.into_iter().skip(1) {
			let ptr = ptr as *const ();
			assert!(ptr != last);
			last = ptr;
		}
	}

	#[test]
	fn store_drops() {
		let counter: Arc<RwLock<usize>> = Default::default();

		let arena = Store::with_page(256);
		let count = 10000;

		for _ in 0..count {
			arena.store(DropCounter::new(counter.clone()));
		}

		assert_eq!(*counter.read().unwrap(), count);
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

	#[test]
	fn store_big_alloc() {
		let counter: Arc<RwLock<usize>> = Default::default();

		let arena = Store::with_page(1);
		let count = 10000;

		for _ in 0..count {
			arena.store(DropCounter::new(counter.clone()));
		}

		assert_eq!(*counter.read().unwrap(), count);
		drop(arena);
		assert_eq!(*counter.read().unwrap(), 0);
	}

	#[test]
	fn store_slice() {
		let counter: Arc<RwLock<usize>> = Default::default();

		let arena = Store::with_page(17);
		let count = 10000;
		let get_counter = || *counter.read().unwrap();

		let mut source_items = Vec::new();
		for i in 0..count {
			source_items.push((i * 10, DropCounter::new(counter.clone())));
		}

		assert_eq!(get_counter(), count);

		let items = arena.slice(source_items);
		assert_eq!(items.len(), count);
		assert_eq!(get_counter(), count);

		for (i, (it, _)) in items.iter().enumerate() {
			assert_eq!(it, &(i * 10));
		}

		drop(arena);
		assert_eq!(get_counter(), 0);
	}

	#[test]
	pub fn store_chunk() {
		let arena = Store::new();
		let value = arena.chunk([1, 2, 3, 4, 5]);
		assert_eq!(5, value.len());
		assert_eq!(&[1, 2, 3, 4, 5], value.as_slice());
	}

	#[derive(Debug)]
	struct DropCounter(Arc<RwLock<usize>>);

	impl DropCounter {
		pub fn new(value: Arc<RwLock<usize>>) -> Self {
			{
				let mut value = value.write().unwrap();
				*value += 1;
			}
			Self(value)
		}
	}

	impl Drop for DropCounter {
		fn drop(&mut self) {
			let mut value = self.0.write().unwrap();
			*value -= 1;
		}
	}
}
