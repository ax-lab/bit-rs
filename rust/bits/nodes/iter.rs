use super::*;

pub trait ExactChain<'a>: IntoIterator<Item = Node<'a>> {
	type Iter: Iterator<Item = Node<'a>> + ExactSizeIterator;

	fn chain_exact<T: IntoIterator<Item = Node<'a>>>(
		self,
		next: T,
	) -> ExactChainedIterator<'a, Self::Iter, T::IntoIter>
	where
		T::IntoIter: ExactSizeIterator;
}

impl<'a, I> ExactChain<'a> for I
where
	I: IntoIterator<Item = Node<'a>>,
	I::IntoIter: ExactSizeIterator,
{
	type Iter = Self::IntoIter;

	fn chain_exact<T: IntoIterator<Item = Node<'a>>>(self, next: T) -> ExactChainedIterator<'a, Self::Iter, T::IntoIter>
	where
		T::IntoIter: ExactSizeIterator,
	{
		ExactChainedIterator {
			a: self.into_iter(),
			b: next.into_iter(),
			a_done: false,
			b_done: false,
		}
	}
}

pub struct ExactChainedIterator<'a, T, U>
where
	T: Iterator<Item = Node<'a>> + ExactSizeIterator,
	U: Iterator<Item = Node<'a>> + ExactSizeIterator,
{
	a: T,
	b: U,
	a_done: bool,
	b_done: bool,
}

impl<'a, T, U> Iterator for ExactChainedIterator<'a, T, U>
where
	T: Iterator<Item = Node<'a>> + ExactSizeIterator,
	U: Iterator<Item = Node<'a>> + ExactSizeIterator,
{
	type Item = Node<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if !self.a_done {
			if let Some(a) = self.a.next() {
				return Some(a);
			} else {
				self.a_done = true;
			}
		}

		if !self.b_done {
			if let Some(b) = self.b.next() {
				return Some(b);
			} else {
				self.b_done = true;
			}
		}

		None
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let size = self.a.len() + self.b.len();
		(size, Some(size))
	}
}

impl<'a, T, U> ExactSizeIterator for ExactChainedIterator<'a, T, U>
where
	T: Iterator<Item = Node<'a>> + ExactSizeIterator,
	U: Iterator<Item = Node<'a>> + ExactSizeIterator,
{
}
