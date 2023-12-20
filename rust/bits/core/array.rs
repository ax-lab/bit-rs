use super::*;

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Array {
	kind: KindId,
	list: Arc<[XValueCell]>,
}

impl XValueCell {
	pub fn array<T: IntoIterator<Item = U>, U: Into<XValue>>(kind: &XKind, elems: T) -> Self {
		let array = Array::of(kind, elems);
		XValue::Array(array).into()
	}

	pub fn as_array(&self) -> &Array {
		match &self.val {
			XValue::Array(array) => &array,
			_ => panic!("value is not an array: {self:?}"),
		}
	}
}

impl Array {
	pub fn of<T: IntoIterator<Item = U>, U: Into<XValueCell>>(kind: &XKind, elems: T) -> Self {
		let list = elems.into_iter().map(|x| x.into()).collect::<Vec<XValueCell>>();
		for it in list.iter() {
			if !it.is_kind_of(kind) {
				panic!("value is not a valid `{kind:?}`: {it:?}")
			}
		}

		Self {
			kind: XKind::array_of(kind).id(),
			list: list.into(),
		}
	}

	pub fn kind(&self) -> XKind {
		self.kind.into()
	}

	pub fn len(&self) -> usize {
		self.list.len()
	}

	pub fn as_slice(&self) -> &[XValueCell] {
		&self.list
	}
}

impl Display for Array {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "[")?;
		for (n, it) in self.list.iter().enumerate() {
			if n > 0 {
				write!(f, ", ")?;
			}
			write!(f, "{it}")?;
		}
		write!(f, "]")
	}
}

impl XKind {
	pub fn array_of(&self) -> Self {
		XKind::Array(self.as_ref())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn simple_array() {
		let cell = XValueCell::array(&I64, [1, 2, 3]);
		assert_eq!(XKind::array_of(&I64), cell.kind());
		assert_eq!(cell.as_array().as_slice(), [1, 2, 3].map(|x| x.into()));
	}
}
