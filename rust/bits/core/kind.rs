use super::*;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct KindId {
	ptr: *const XKind,
}

impl Default for KindId {
	fn default() -> Self {
		KindId::none()
	}
}

impl KindId {
	pub fn as_kind(&self) -> &'static XKind {
		unsafe { &*self.ptr }
	}

	pub fn none() -> Self {
		static NONE: OnceLock<KindId> = OnceLock::new();
		let out = NONE.get_or_init(|| XKind::None.id());
		*out
	}

	pub fn unknown() -> Self {
		static UNKNOWN: OnceLock<KindId> = OnceLock::new();
		let out = UNKNOWN.get_or_init(|| XKind::Unknown.id());
		*out
	}

	pub fn is_none(&self) -> bool {
		self.as_kind() != &XKind::None
	}

	pub fn is_some(&self) -> bool {
		!self.is_none()
	}

	pub fn is_valid(&self) -> bool {
		self.is_some() && self.is_known()
	}

	pub fn is_unknown(&self) -> bool {
		self.as_kind() == &XKind::Unknown
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

impl From<KindId> for XKind {
	fn from(value: KindId) -> Self {
		value.as_kind().clone()
	}
}

unsafe impl Send for KindId {}
unsafe impl Sync for KindId {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum XKind {
	None,
	Unknown,
	Any,
	Unit,
	Bool,
	Int(IntKind),
	Float(FloatKind),
	Str,
	Array(&'static XKind),
}

impl XValueCell {
	pub fn is_kind_of(&self, kind: &XKind) -> bool {
		self.kind().is_valid(kind)
	}
}

impl<T: Into<XValue>> From<T> for XValueCell {
	fn from(value: T) -> Self {
		Self { val: value.into() }
	}
}

impl XKind {
	pub fn id(&self) -> KindId {
		KindId { ptr: self.as_ref() }
	}

	pub fn is_none(&self) -> bool {
		self == &XKind::None
	}

	pub fn is_valid(&self, other: &XKind) -> bool {
		match other {
			XKind::None => false,
			XKind::Any => self != &XKind::None,
			XKind::Int(other) => {
				if let XKind::Int(kind) = self {
					kind.is_valid(other)
				} else {
					false
				}
			}
			XKind::Float(other) => {
				if let XKind::Float(kind) = self {
					kind.is_valid(other)
				} else {
					false
				}
			}
			_ => self == other,
		}
	}
}

impl Default for XKind {
	fn default() -> Self {
		XKind::None
	}
}

impl XKind {
	pub fn as_ref(&self) -> &'static Self {
		static MAP: OnceLock<RwLock<HashMap<XKind, KindPtr>>> = OnceLock::new();
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
		struct KindPtr(*const XKind);

		impl KindPtr {
			pub fn new(kind: XKind) -> Self {
				let ptr = Box::leak(Box::new(kind));
				Self(ptr)
			}

			pub fn as_ref(self) -> &'static XKind {
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
		let a1 = XKind::Float(FloatKind::F32).as_ref();
		let a2 = XKind::Float(FloatKind::F32).as_ref();

		let b1 = XKind::Float(FloatKind::F64).as_ref();
		let b2 = XKind::Float(FloatKind::F64).as_ref();

		assert_eq!(a1, &XKind::Float(FloatKind::F32));
		assert_eq!(a2, &XKind::Float(FloatKind::F32));
		assert_eq!(a1 as *const XKind, a2 as *const XKind);

		assert_eq!(b1, &XKind::Float(FloatKind::F64));
		assert_eq!(b2, &XKind::Float(FloatKind::F64));
		assert_eq!(b1 as *const XKind, b2 as *const XKind);
	}
}
