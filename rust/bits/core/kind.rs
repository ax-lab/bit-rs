use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct KindId {
	ptr: *const Kind,
}

impl Default for KindId {
	fn default() -> Self {
		KindId::none()
	}
}

impl KindId {
	pub fn as_kind(&self) -> &'static Kind {
		unsafe { &*self.ptr }
	}

	pub fn none() -> Self {
		static NONE: OnceLock<KindId> = OnceLock::new();
		let out = NONE.get_or_init(|| Kind::None.id());
		*out
	}

	pub fn unknown() -> Self {
		static UNKNOWN: OnceLock<KindId> = OnceLock::new();
		let out = UNKNOWN.get_or_init(|| Kind::Unknown.id());
		*out
	}

	pub fn is_none(&self) -> bool {
		self.as_kind() != &Kind::None
	}

	pub fn is_some(&self) -> bool {
		!self.is_none()
	}

	pub fn is_valid(&self) -> bool {
		self.is_some() && self.is_known()
	}

	pub fn is_unknown(&self) -> bool {
		self.as_kind() == &Kind::Unknown
	}

	pub fn is_known(&self) -> bool {
		!self.is_unknown()
	}
}

impl Debug for KindId {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let kind = self.as_kind();
		write!(f, "<{kind:?} #{:?}>", self.ptr)
	}
}

impl Ord for KindId {
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_kind().cmp(other.as_kind())
	}
}

impl PartialOrd for KindId {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl From<KindId> for Kind {
	fn from(value: KindId) -> Self {
		value.as_kind().clone()
	}
}

unsafe impl Send for KindId {}
unsafe impl Sync for KindId {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Kind {
	None,
	Unknown,
	Any,
	Unit,
	Bool,
	Int(IntKind),
	Float(FloatKind),
	Str,
	Array(&'static Kind),
}

impl Data {
	pub fn is_kind_of(&self, kind: &Kind) -> bool {
		self.kind().is_valid(kind)
	}
}

impl<T: Into<Value>> From<T> for Data {
	fn from(value: T) -> Self {
		Self { val: value.into() }
	}
}

impl Kind {
	pub fn id(&self) -> KindId {
		KindId { ptr: self.as_ref() }
	}

	pub fn is_none(&self) -> bool {
		self == &Kind::None
	}

	pub fn is_valid(&self, other: &Kind) -> bool {
		match other {
			Kind::None => false,
			Kind::Any => self != &Kind::None,
			Kind::Int(other) => {
				if let Kind::Int(kind) = self {
					kind.is_valid(other)
				} else {
					false
				}
			}
			Kind::Float(other) => {
				if let Kind::Float(kind) = self {
					kind.is_valid(other)
				} else {
					false
				}
			}
			_ => self == other,
		}
	}
}

impl Default for Kind {
	fn default() -> Self {
		Kind::None
	}
}

impl Kind {
	pub fn as_ref(&self) -> &'static Self {
		static MAP: OnceLock<RwLock<HashMap<Kind, KindPtr>>> = OnceLock::new();
		let map = MAP.get_or_init(|| Default::default());
		{
			let map = map.read().unwrap();
			if let Some(ptr) = map.get(self) {
				return ptr.as_ref();
			}
		}

		let mut map = map.write().unwrap();
		let entry = map.entry(self.clone()).or_insert_with(|| KindPtr::new(self.clone()));
		return entry.as_ref();

		#[derive(Copy, Clone, Eq, PartialEq)]
		struct KindPtr(*const Kind);

		impl KindPtr {
			pub fn new(kind: Kind) -> Self {
				let ptr = Box::leak(Box::new(kind));
				Self(ptr)
			}

			pub fn as_ref(self) -> &'static Kind {
				unsafe { &*self.0 }
			}
		}

		unsafe impl Send for KindPtr {}
		unsafe impl Sync for KindPtr {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn kind_as_ref() {
		let a1 = Kind::Float(FloatKind::F32).as_ref();
		let a2 = Kind::Float(FloatKind::F32).as_ref();

		let b1 = Kind::Float(FloatKind::F64).as_ref();
		let b2 = Kind::Float(FloatKind::F64).as_ref();

		assert_eq!(a1, &Kind::Float(FloatKind::F32));
		assert_eq!(a2, &Kind::Float(FloatKind::F32));
		assert_eq!(a1 as *const Kind, a2 as *const Kind);

		assert_eq!(b1, &Kind::Float(FloatKind::F64));
		assert_eq!(b2, &Kind::Float(FloatKind::F64));
		assert_eq!(b1 as *const Kind, b2 as *const Kind);
	}
}
