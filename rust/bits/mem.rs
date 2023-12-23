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

pub struct Arena {
	page_size: usize,
	max_alloc: usize,
	page: AtomicPtr<MemPage>,
	sync: Mutex<()>,
}

impl Arena {
	pub fn get() -> &'static Arena {
		static ARENA: OnceLock<Arena> = OnceLock::new();
		let arena = ARENA.get_or_init(|| {
			const PAGE_SIZE: usize = 256 * MB;
			const MAX_ALLOC: usize = PAGE_SIZE / 8;
			Arena::new(PAGE_SIZE, MAX_ALLOC)
		});
		arena
	}

	fn new(page_size: usize, max_alloc: usize) -> Arena {
		assert!(page_size > 0);
		assert!(max_alloc > 0 && max_alloc <= page_size);
		let page = MemPage::new(page_size);
		Arena {
			page_size,
			max_alloc,
			page: AtomicPtr::new(page.as_ptr()),
			sync: Default::default(),
		}
	}

	pub fn total_size() -> usize {
		MemPage::stat().size.load(SyncOrder::Relaxed)
	}

	pub fn total_used() -> usize {
		MemPage::stat().used.load(SyncOrder::Relaxed)
	}

	pub fn str<T: AsRef<str>>(&self, str: T) -> &'static str {
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

	pub fn slice<T: IntoIterator<Item = U>, U>(&self, elems: T) -> &'static mut [U]
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
			std::slice::from_raw_parts_mut(data, count)
		}
	}

	pub fn store<T>(&self, value: T) -> &'static mut T {
		unsafe {
			let mut ptr = self.alloc(Layout::new::<T>()).cast::<T>();
			ptr.as_ptr().write(value);
			ptr.as_mut()
		}
	}

	pub fn alloc(&self, layout: Layout) -> NonNull<u8> {
		// don't use the arena for large allocations
		let size = layout.size();
		if size >= self.max_alloc {
			return MemPage::raw_alloc(layout, size);
		}

		// default optimistic path
		if let Some(ptr) = self.page().alloc(layout) {
			return ptr;
		}

		// if the above failed, allocate a new page discarding any remaining
		// space on the existing page
		let lock = self.sync.lock().unwrap();

		// there's a chance someone created a new page in between
		if let Some(ptr) = self.page().alloc(layout) {
			return ptr;
		}

		let page = MemPage::new(self.page_size);
		let data = {
			let data = unsafe { page.as_ref() }.alloc(layout);
			data.unwrap()
		};

		self.page.store(page.as_ptr(), SyncOrder::Release);
		drop(lock);
		data
	}

	fn page(&self) -> &'static MemPage {
		unsafe {
			let ptr = NonNull::new_unchecked(self.page.load(SyncOrder::Acquire));
			ptr.as_ref()
		}
	}
}

struct MemPage {
	size: usize,
	next: AtomicUsize,
	data: [u8; 1],
}

impl MemPage {
	/// Create a new memory page with the given size and static lifetime.
	///
	/// Once created, the [`MemPage`] is never deallocated, but the backing
	/// memory for the page can be disposed of.
	pub fn new(size: usize) -> NonNull<Self> {
		const NUL: u8 = 0xDC;

		let align = std::mem::align_of::<MemPage>();
		let size = std::mem::size_of::<MemPage>() + size - 1;
		let data = unsafe {
			let layout = Layout::from_size_align(size, align).unwrap();
			let data = Self::raw_alloc(layout, 0);
			data.as_ptr().write_bytes(NUL, size);

			let data = data.cast::<MemPage>();
			data.as_ptr().write(MemPage {
				size,
				next: Default::default(),
				data: [NUL],
			});
			data
		};
		data
	}

	pub fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
		let align = layout.align();
		let size = std::cmp::max(1, layout.size());
		let data = self.data.as_ptr();
		loop {
			let next = self.next.load(SyncOrder::Relaxed);
			let addr = unsafe { data.add(next) as usize };

			// align the allocation and check if it's valid
			let pos = next + (align - addr % align) % align;
			let end = pos + size;
			if end > self.size {
				return None;
			}

			// the allocation is valid, try to commit
			if self
				.next
				.compare_exchange_weak(next, end, SyncOrder::Relaxed, SyncOrder::Relaxed)
				.is_ok()
			{
				Self::stat().used.fetch_add(size, SyncOrder::Relaxed);
				let ptr = unsafe {
					let ptr = data.add(next) as *mut u8;
					NonNull::new_unchecked(ptr)
				};
				return Some(ptr);
			}
		}
	}

	pub fn raw_alloc(layout: Layout, used: usize) -> NonNull<u8> {
		let stat = Self::stat();
		let size = layout.size();
		stat.size.fetch_add(size, SyncOrder::Relaxed);
		stat.used.fetch_add(used, SyncOrder::Relaxed);

		let ptr = unsafe { std::alloc::alloc(layout) };
		if let Some(ptr) = NonNull::new(ptr) {
			ptr
		} else {
			panic!("memory allocation failed: {layout:?}")
		}
	}

	fn stat() -> &'static MemStat {
		static STAT: MemStat = MemStat {
			size: AtomicUsize::new(0),
			used: AtomicUsize::new(0),
		};
		&STAT
	}
}

struct MemStat {
	size: AtomicUsize,
	used: AtomicUsize,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn arena_store() {
		let arena = Arena::get();

		let value = arena.store(format!("abc123"));
		assert_eq!("abc123", value);

		let value = arena.str("123456");
		assert_eq!("123456", value);

		let value = arena.slice([1, 2, 3, 4, 5]);
		assert_eq!(&[1, 2, 3, 4, 5], value);
	}
}
