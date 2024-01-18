use super::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct List<T: Clone + 'static> {
	data: *const ListData<T>,
}

unsafe impl<T: Send + Clone> Send for List<T> {}
unsafe impl<T: Sync + Clone> Sync for List<T> {}

impl<T: Clone + 'static> List<T> {
	pub fn empty() -> Self {
		let data = Arena::get().store(ListData {
			inner: Default::default(),
		});
		Self { data }
	}

	pub fn new<I: IntoIterator<Item = T>>(items: I) -> Self
	where
		I::IntoIter: ExactSizeIterator,
	{
		let list = Self::empty();
		list.replace(items);
		list
	}

	#[inline(always)]
	pub fn len(&self) -> usize {
		self.items().len()
	}

	#[inline(always)]
	pub fn items(&self) -> &'static [T] {
		let data = self.data();
		let list = *data.inner.read().unwrap();
		list
	}

	#[inline(always)]
	pub fn set(&self, list: &'static [T]) {
		let data = self.data();
		*data.inner.write().unwrap() = list;
	}

	#[inline(always)]
	pub fn replace<I: IntoIterator<Item = T>>(&self, items: I)
	where
		I::IntoIter: ExactSizeIterator,
	{
		let list = Arena::get().slice(items);
		self.set(list);
	}

	#[inline(always)]
	pub fn push(&self, item: T) {
		self.append(std::iter::once(item));
	}

	#[inline(always)]
	pub fn append<I: IntoIterator<Item = T>>(&self, items: I)
	where
		I::IntoIter: ExactSizeIterator,
	{
		let list = self.items();
		self.insert_and_set(list, list.len(), items);
	}

	#[inline(always)]
	pub fn insert_and_set<I: IntoIterator<Item = T>>(&self, list: &'static [T], at: usize, items: I)
	where
		I::IntoIter: ExactSizeIterator,
	{
		self.replace_and_set(list, at..at, items);
	}

	#[inline(always)]
	pub fn remove_and_set<R: RangeBounds<usize>>(self, list: &'static [T], range: R) -> &'static [T] {
		self.replace_and_set(list, range, std::iter::empty())
	}

	pub fn replace_and_set<R: RangeBounds<usize>, I: IntoIterator<Item = T>>(
		&self,
		list: &'static [T],
		range: R,
		items: I,
	) -> &'static [T]
	where
		I::IntoIter: ExactSizeIterator,
	{
		let sta = match range.start_bound() {
			std::ops::Bound::Included(&n) => n,
			std::ops::Bound::Excluded(&n) => n + 1,
			std::ops::Bound::Unbounded => 0,
		};
		let end = match range.end_bound() {
			std::ops::Bound::Included(&n) => n + 1,
			std::ops::Bound::Excluded(&n) => n,
			std::ops::Bound::Unbounded => list.len(),
		};

		assert!(sta <= end && end <= list.len());

		let head = list[..sta].iter().cloned();
		let tail = list[end..].iter().cloned();
		let items = items.into_iter();

		let replaced = &list[sta..end];
		if replaced.len() == 0 && items.len() == 0 {
			return replaced;
		}

		let list = if items.len() == 0 {
			if sta == 0 {
				&list[end..]
			} else if end >= self.len() {
				&list[..sta]
			} else {
				let list = head.chain_exact(tail);
				Arena::get().slice(list)
			}
		} else {
			let list = head.chain_exact(items).chain_exact(tail);
			Arena::get().slice(list)
		};

		self.set(list);
		replaced
	}

	#[inline(always)]
	fn data(&self) -> &'static ListData<T> {
		unsafe { &*self.data }
	}
}

impl<I: IntoIterator<Item = T>, T: Clone> From<I> for List<T>
where
	I::IntoIter: ExactSizeIterator,
{
	fn from(value: I) -> Self {
		List::new(value)
	}
}

impl<T: Clone> Default for List<T> {
	fn default() -> Self {
		Self::empty()
	}
}

struct ListData<T: 'static> {
	inner: RwLock<&'static [T]>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_list() {
		let ls: List<i32> = List::empty();
		assert_eq!(0, ls.len());

		let expected: &[i32] = &[];
		assert_eq!(expected, ls.items());
	}

	#[test]
	fn simple_list() {
		let ls: List<i32> = [1, 2, 3].into();
		assert_eq!(3, ls.len());
		assert_eq!(&[1, 2, 3], ls.items());
	}

	#[test]
	fn list_push() {
		let ls: List<i32> = List::empty();
		ls.push(1);
		ls.push(2);
		ls.push(3);
		assert_eq!(&[1, 2, 3], ls.items());

		let ls: List<i32> = [1, 2, 3].into();
		ls.push(4);
		ls.push(5);
		assert_eq!(&[1, 2, 3, 4, 5], ls.items());
	}

	#[test]
	fn list_append() {
		let ls: List<i32> = List::empty();
		ls.append([1, 2, 3]);
		assert_eq!(&[1, 2, 3], ls.items());

		let ls: List<i32> = [1, 2, 3].into();
		ls.append([4, 5]);
		assert_eq!(&[1, 2, 3, 4, 5], ls.items());
	}

	#[test]
	fn list_insert() {
		let ls: List<i32> = List::empty();

		ls.insert_and_set(ls.items(), 0, [3, 6, 9]);
		assert_eq!(&[3, 6, 9], ls.items());

		ls.insert_and_set(ls.items(), 0, [1, 2]);
		assert_eq!(&[1, 2, 3, 6, 9], ls.items());

		ls.insert_and_set(ls.items(), 3, [4, 5]);
		assert_eq!(&[1, 2, 3, 4, 5, 6, 9], ls.items());

		ls.insert_and_set(ls.items(), 6, [7, 8]);
		assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9], ls.items());

		ls.insert_and_set(ls.items(), 9, [0]);
		assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0], ls.items());
	}

	#[test]
	fn list_remove() {
		let ls: List<i32> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into();

		let out = ls.remove_and_set(ls.items(), 0..0);
		assert_eq!(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9], ls.items());
		assert!(out.len() == 0);

		let out = ls.remove_and_set(ls.items(), 4..=6);
		assert_eq!(&[0, 1, 2, 3, 7, 8, 9], ls.items());
		assert_eq!(&[4, 5, 6], out);

		let out = ls.remove_and_set(ls.items(), 4..);
		assert_eq!(&[0, 1, 2, 3], ls.items());
		assert_eq!(&[7, 8, 9], out);

		let out = ls.remove_and_set(ls.items(), ..2);
		assert_eq!(&[2, 3], ls.items());
		assert_eq!(&[0, 1], out);

		let out = ls.remove_and_set(ls.items(), ..);
		assert!(ls.items().len() == 0);
		assert_eq!(&[2, 3], out);

		let out = ls.remove_and_set(ls.items(), ..);
		assert!(ls.items().len() == 0);
		assert!(out.len() == 0);
	}
}
