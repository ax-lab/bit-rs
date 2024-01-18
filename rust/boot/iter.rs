pub trait ExactChain<I, T>
where
	I: IntoIterator<Item = T>,
	I::IntoIter: ExactSizeIterator,
{
	fn chain_exact<U: IntoIterator<Item = T>>(self, next: U) -> ExactChainedIterator<T, I::IntoIter, U::IntoIter>
	where
		U::IntoIter: ExactSizeIterator;
}

impl<I, T> ExactChain<I, T> for I
where
	I: IntoIterator<Item = T>,
	I::IntoIter: ExactSizeIterator,
{
	fn chain_exact<U: IntoIterator<Item = T>>(self, next: U) -> ExactChainedIterator<T, I::IntoIter, U::IntoIter>
	where
		U::IntoIter: ExactSizeIterator,
	{
		ExactChainedIterator {
			a: self.into_iter(),
			b: next.into_iter(),
			a_done: false,
			b_done: false,
		}
	}
}

pub struct ExactChainedIterator<T, I1, I2>
where
	I1: Iterator<Item = T> + ExactSizeIterator,
	I2: Iterator<Item = T> + ExactSizeIterator,
{
	a: I1,
	b: I2,
	a_done: bool,
	b_done: bool,
}

impl<T, I1, I2> Iterator for ExactChainedIterator<T, I1, I2>
where
	I1: Iterator<Item = T> + ExactSizeIterator,
	I2: Iterator<Item = T> + ExactSizeIterator,
{
	type Item = T;

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

impl<T, I1, I2> ExactSizeIterator for ExactChainedIterator<T, I1, I2>
where
	I1: Iterator<Item = T> + ExactSizeIterator,
	I2: Iterator<Item = T> + ExactSizeIterator,
{
}
