use super::*;

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
	data: &'static TypeKind,
}

static NONE: TypeKind = TypeKind::None;
static UNIT: TypeKind = TypeKind::Unit;
static NEVER: TypeKind = TypeKind::Never;
static ANY: TypeKind = TypeKind::Any;
static UNKNOWN: TypeKind = TypeKind::Unknown;

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
		static INVALID: TypeDataMap = TypeDataMap::new();
		if let TypeKind::Invalid(..) = self.data {
			*self
		} else {
			INVALID.get(self, |typ| TypeKind::Invalid(typ))
		}
	}

	/// Is this type valid?
	pub fn is_valid(&self) -> bool {
		!self.is_invalid()
	}

	/// Is this type an invalid type?
	pub fn is_invalid(&self) -> bool {
		matches!(self.data, TypeKind::Invalid(..))
	}

	/// Return a valid type either by unwrapping an invalid type or returning
	/// self if it is already valid.
	pub fn get_valid(&self) -> Type {
		if let &TypeKind::Invalid(typ) = self.data {
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
	fn as_ptr(&self) -> *const TypeKind {
		self.data
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
enum TypeKind {
	None,
	Unit,
	Never,
	Any,
	Unknown,
	Invalid(Type),
}

impl TypeKind {
	fn store(self) -> &'static Self {
		match self {
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

impl Debug for TypeKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut ptr = false;
		match self {
			Self::None => {
				write!(f, "None")?;
				ptr = self.as_ptr() != NONE.as_ptr();
			}
			Self::Unit => {
				write!(f, "Unit")?;
				ptr = self.as_ptr() != UNIT.as_ptr();
			}
			Self::Never => {
				write!(f, "Never")?;
				ptr = self.as_ptr() != NEVER.as_ptr();
			}
			Self::Any => {
				write!(f, "Any")?;
				ptr = self.as_ptr() != ANY.as_ptr();
			}
			Self::Unknown => {
				write!(f, "Unknown")?;
				ptr = self.as_ptr() != UNKNOWN.as_ptr();
			}
			Self::Invalid(typ) => {
				write!(f, "Invalid({typ:?}")?;
			}
		}
		if ptr {
			write!(f, "#{:06x}", self.display_id())?;
		}
		Ok(())
	}
}

impl Display for TypeKind {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

struct TypeDataMap {
	map: OnceLock<RwLock<HashMap<Type, &'static TypeKind>>>,
}

impl TypeDataMap {
	pub const fn new() -> Self {
		Self { map: OnceLock::new() }
	}

	pub fn get<F: Fn(Type) -> TypeKind>(&self, key: &Type, init: F) -> Type {
		let map = self.map.get_or_init(|| Default::default());
		if let Some(data) = map.read().unwrap().get(key).copied() {
			return Type { data };
		}

		let mut map = map.write().unwrap();
		let entry = map.entry(*key).or_insert_with(|| {
			let data = init(*key);
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
