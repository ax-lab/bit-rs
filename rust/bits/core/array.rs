use super::*;

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Array {
	kind: Kind,
	list: Arc<[Data]>,
}

impl Data {
	pub fn array<T: IntoIterator<Item = U>, U: Into<Value>>(kind: &Kind, elems: T) -> Self {
		let array = Array::of(kind, elems);
		Value::Array(array).into()
	}

	pub fn as_array(&self) -> &Array {
		match &self.val {
			Value::Array(array) => &array,
			_ => panic!("value is not an array: {self:?}"),
		}
	}
}

impl Array {
	pub fn of<T: IntoIterator<Item = U>, U: Into<Data>>(kind: &Kind, elems: T) -> Self {
		let list = elems.into_iter().map(|x| x.into()).collect::<Vec<Data>>();
		for it in list.iter() {
			if !it.is_kind_of(kind) {
				panic!("value is not a valid `{kind:?}`: {it:?}")
			}
		}

		Self {
			kind: Kind::array_of(kind),
			list: list.into(),
		}
	}

	pub fn kind(&self) -> Kind {
		self.kind
	}

	pub fn len(&self) -> usize {
		self.list.len()
	}

	pub fn as_slice(&self) -> &[Data] {
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

impl Kind {
	pub fn array_of(&self) -> Self {
		Kind::Array(self.as_ref())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn simple_array() {
		let cell = Data::array(&I64, [1, 2, 3]);
		assert_eq!(Kind::array_of(&I64), cell.kind());
		assert_eq!(cell.as_array().as_slice(), [1, 2, 3].map(|x| x.into()));
	}
}
