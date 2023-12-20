use super::*;

pub mod repr;
pub mod symbol;

pub use repr::*;
pub use symbol::*;

/// Type descriptor for any type.
///
/// Types can be concrete, abstract, parametric, etc. This can also represent
/// partially specified, unknown, and invalid types.
///
/// Types are immutable and have static lifetime. New types can be created
/// through composition or overriding a base type.
///
/// Two types are considered equal if they have the exact same definition.
///
/// A type can describe a concrete data layout. Type operations however are
/// defined through abstract symbols. Providing a concrete implementation for
/// a given symbol + type arguments is left to each environment.
///
/// The type system also supports arbitrary constraints. This can be used to
/// constrain generic type parameters or to specify and propagate constraints
/// for type values.
///
/// Named types are also supported through abstract symbols. The textual
/// representation of a symbol (if any) is left to the environment.
#[derive(Copy, Clone)]
pub struct Type {
	data: &'static TypeData,
}

static NONE: TypeData = TypeData {
	kind: TypeKind::None,
	repr: Some(DataRepr::Empty),
};

static UNIT: TypeData = TypeData {
	kind: TypeKind::Unit,
	repr: Some(DataRepr::Empty),
};

static NEVER: TypeData = TypeData {
	kind: TypeKind::Never,
	repr: None,
};

static ANY: TypeData = TypeData {
	kind: TypeKind::Any,
	repr: None,
};

static UNKNOWN: TypeData = TypeData {
	kind: TypeKind::Unknown,
	repr: None,
};

impl Type {
	/// Null-value for a type, representing the lack of a type (e.g. void type).
	pub fn none() -> Type {
		let data = &NONE;
		Type { data }
	}

	/// Concrete type containing only a single zero-sized value.
	pub fn unit() -> Type {
		let data = &UNIT;
		Type { data }
	}

	/// Concrete type containing no possible values. The never type indicates
	/// is used to indicate a logically impossible value.
	pub fn never() -> Type {
		let data = &NEVER;
		Type { data }
	}

	/// Concrete type able to hold any possible value.
	pub fn any() -> Type {
		let data = &ANY;
		Type { data }
	}

	/// Abstract unknown type.
	pub fn unknown() -> Type {
		let data = &UNKNOWN;
		Type { data }
	}

	/// Empty invalid type. An invalid type indicates a type that is not valid
	/// at runtime, but can be returned for error handling.
	///
	/// Any number of non-empty invalid types are possible. An invalid type can
	/// be derived from an invalid type or by invalidating a valid type.
	///
	/// Operations with invalid types should always result in an invalid type.
	pub fn invalid() -> Type {
		Type::none().to_invalid()
	}

	/// Return the invalid type based on the current type.
	///
	/// If this is a valid type, return the type with the same definition
	/// but which is not valid.
	///
	/// For an invalid type, return the type itself.
	///
	/// This will always return the same type when called on the same base type.
	pub fn to_invalid(&self) -> Type {
		static INVALID: TypeMap<Type> = TypeMap::new();
		if let TypeKind::Invalid(..) = self.data.kind {
			*self
		} else {
			INVALID.get(self, |typ| TypeData {
				kind: TypeKind::Invalid(typ),
				repr: None,
			})
		}
	}

	/// Is this type valid?
	pub fn is_valid(&self) -> bool {
		!self.is_invalid()
	}

	/// Is this type an invalid type?
	pub fn is_invalid(&self) -> bool {
		matches!(self.data.kind, TypeKind::Invalid(..))
	}

	/// Return a valid type either by unwrapping an invalid type or returning
	/// self if it is already valid.
	pub fn get_valid(&self) -> Type {
		if let TypeKind::Invalid(typ) = self.data.kind {
			typ
		} else {
			*self
		}
	}

	/// Is this the none type?
	pub fn is_none(&self) -> bool {
		*self == Type::none()
	}

	/// Is this the unknown type?
	pub fn is_unknown(&self) -> bool {
		*self == Type::unknown()
	}

	/// A proper type is not none, unknown, or invalid.
	pub fn is_proper(&self) -> bool {
		!(self.is_none() || self.is_invalid() || self.is_unknown())
	}

	/// Create a new unique type sharing the same type definition as the
	/// current type.
	///
	/// Some types such as none and unknown can only have one instance, so
	/// unique will return the same type.
	///
	/// The returned unique type is only equal to itself.
	pub fn to_unique(&self) -> Type {
		let data = self.data.clone();
		let data = data.store();
		Type { data }
	}

	/// Underlying data representation for types that have it.
	pub fn data(&self) -> Option<&'static DataRepr> {
		self.data.repr.as_ref()
	}

	/// Return the sum of this type with the given type.
	pub fn sum(&self, _other: Type) -> Type {
		todo!()
	}

	/// Return the intersection of this type with the given type.
	pub fn intersect(&self, _other: Type) -> Type {
		todo!()
	}

	/// Return the type resulting from subtracting the given type from the
	/// current type.
	pub fn subtract(&self, _other: Type) -> Type {
		todo!()
	}

	/// Is the current type a superset of the given type?
	pub fn contains(&self, _other: Type) -> bool {
		todo!()
	}

	#[inline]
	fn as_ptr(&self) -> *const TypeData {
		self.data.as_ptr()
	}
}

impl Eq for Type {}

impl PartialEq for Type {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl Hash for Type {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_ptr().hash(state)
	}
}

impl Debug for Type {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.data)
	}
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.data)
	}
}

impl Ord for Type {
	fn cmp(&self, other: &Self) -> Ordering {
		self.data.cmp(&other.data)
	}
}

impl PartialOrd for Type {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct TypeData {
	kind: TypeKind,
	repr: Option<DataRepr>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
enum TypeKind {
	None,
	Unit,
	Never,
	Any,
	Unknown,
	Invalid(Type),
	Builtin(Primitive),
}

impl TypeData {
	fn store(self) -> &'static Self {
		match self.kind {
			TypeKind::None => &NONE,
			TypeKind::Unknown => &UNKNOWN,
			_ => Box::leak(Box::new(self)),
		}
	}

	fn as_ptr(&self) -> *const Self {
		self
	}

	fn display_id(&self) -> usize {
		(self.as_ptr() as usize / std::mem::size_of::<usize>()) % 0x1000000
	}
}

impl Debug for TypeData {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut ptr = false;
		match self.kind {
			TypeKind::None => {
				write!(f, "None")?;
				ptr = self.as_ptr() != NONE.as_ptr();
			}
			TypeKind::Unit => {
				write!(f, "Unit")?;
				ptr = self.as_ptr() != UNIT.as_ptr();
			}
			TypeKind::Never => {
				write!(f, "Never")?;
				ptr = self.as_ptr() != NEVER.as_ptr();
			}
			TypeKind::Any => {
				write!(f, "Any")?;
				ptr = self.as_ptr() != ANY.as_ptr();
			}
			TypeKind::Unknown => {
				write!(f, "Unknown")?;
				ptr = self.as_ptr() != UNKNOWN.as_ptr();
			}
			TypeKind::Invalid(typ) => {
				write!(f, "Invalid({typ:?}")?;
			}
			TypeKind::Builtin(typ) => {
				write!(f, "{typ:?}")?;
				ptr = self.as_ptr() != Type::builtin(typ).as_ptr();
			}
		}
		if ptr {
			write!(f, "#{:06x}", self.display_id())?;
		}
		Ok(())
	}
}

impl Display for TypeData {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

struct TypeMap<T: Eq + Hash + Clone> {
	map: OnceLock<RwLock<HashMap<T, &'static TypeData>>>,
}

impl<T: Eq + Hash + Clone> TypeMap<T> {
	pub const fn new() -> Self {
		Self { map: OnceLock::new() }
	}

	pub fn get<F: Fn(T) -> TypeData>(&self, key: &T, init: F) -> Type {
		let map = self.map.get_or_init(|| Default::default());
		if let Some(data) = map.read().unwrap().get(key).copied() {
			return Type { data };
		}

		let mut map = map.write().unwrap();
		let entry = map.entry(key.clone()).or_insert_with(|| {
			let data = init(key.clone());
			data.store()
		});
		Type { data: *entry }
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_types() {
		assert_eq!(Type::none(), Type::none());
		assert_eq!(Type::unit(), Type::unit());
		assert_eq!(Type::never(), Type::never());
		assert_eq!(Type::any(), Type::any());
		assert_eq!(Type::unknown(), Type::unknown());
		assert_eq!(Type::invalid(), Type::invalid());

		assert!(Type::invalid().is_invalid());
		assert!(Type::unit().to_invalid().is_invalid());

		assert_ne!(Type::unit().to_unique(), Type::unit());
		assert_eq!(Type::none().to_unique(), Type::none());
		assert_eq!(Type::unknown().to_unique(), Type::unknown());
	}
}
