use super::*;

pub mod repr;
pub mod symbol;

pub use repr::*;
pub use symbol::*;

pub struct TypeContext<'a> {
	ctx: ContextRef<'a>,

	none: TypeData<'a>,
	unit: TypeData<'a>,
	str: TypeData<'a>,
	bool: TypeData<'a>,
	sint: TypeData<'a>,
	uint: TypeData<'a>,
	never: TypeData<'a>,
	any: TypeData<'a>,
	unknown: TypeData<'a>,

	invalid: TypeMap<'a, Type<'a>>,
	builtin: TypeMap<'a, Primitive>,

	sum_types: TypeMap<'a, (Type<'a>, Type<'a>)>,
}

unsafe impl<'a> Send for TypeData<'a> {}
unsafe impl<'a> Sync for TypeData<'a> {}
impl<'a> UnwindSafe for TypeData<'a> {}

impl<'a> IsContext<'a> for TypeContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		let none = TypeData {
			ctx,
			kind: TypeKind::None,
		};

		let unit = TypeData {
			ctx,
			kind: TypeKind::Unit,
		};

		let str = TypeData {
			ctx,
			kind: TypeKind::Builtin(Primitive::String),
		};

		let bool = TypeData {
			ctx,
			kind: TypeKind::Builtin(Primitive::Bool),
		};

		let sint = TypeData {
			ctx,
			kind: TypeKind::Builtin(Primitive::SInt(64)),
		};

		let uint = TypeData {
			ctx,
			kind: TypeKind::Builtin(Primitive::UInt(64)),
		};

		let never = TypeData {
			ctx,
			kind: TypeKind::Never,
		};

		let any = TypeData {
			ctx,
			kind: TypeKind::Any,
		};

		let unknown = TypeData {
			ctx,
			kind: TypeKind::Unknown,
		};

		Self {
			ctx,
			none,
			unit,
			str,
			bool,
			sint,
			uint,
			never,
			any,
			unknown,

			invalid: TypeMap::new(),
			builtin: TypeMap::new(),
			sum_types: TypeMap::new(),
		}
	}

	fn init(&mut self) {}
}

impl<'a> TypeContext<'a> {
	/// Null-value for a type, representing the lack of a type (e.g. void type).
	pub fn none(&'a self) -> Type<'a> {
		let data = &self.none;
		Type { data }
	}

	/// Concrete type containing only a single zero-sized value.
	pub fn unit(&'a self) -> Type<'a> {
		let data = &self.unit;
		Type { data }
	}

	/// Concrete type containing no possible values. The never type indicates
	/// is used to indicate a logically impossible value.
	pub fn never(&'a self) -> Type<'a> {
		let data = &self.never;
		Type { data }
	}

	/// Concrete type able to hold any possible value.
	pub fn any(&'a self) -> Type<'a> {
		let data = &self.any;
		Type { data }
	}

	/// Abstract unknown type.
	pub fn unknown(&'a self) -> Type<'a> {
		let data = &self.unknown;
		Type { data }
	}

	/// Default string type.
	pub fn str(&'a self) -> Type<'a> {
		let data = &self.str;
		Type { data }
	}

	/// Builtin boolean type.
	pub fn bool(&'a self) -> Type<'a> {
		let data = &self.bool;
		Type { data }
	}

	/// Default signed integer.
	pub fn sint(&'a self) -> Type<'a> {
		let data = &self.sint;
		Type { data }
	}

	/// Default unsigned integer.
	pub fn uint(&'a self) -> Type<'a> {
		let data = &self.uint;
		Type { data }
	}

	/// Empty invalid type. An invalid type indicates a type that is not valid
	/// at runtime, but can be returned for error handling.
	///
	/// Any number of non-empty invalid types are possible. An invalid type can
	/// be derived from an invalid type or by invalidating a valid type.
	///
	/// Operations with invalid types should always result in an invalid type.
	pub fn invalid(&'a self) -> Type {
		self.none().to_invalid()
	}

	fn store(&'a self, data: TypeData<'a>) -> &'a TypeData<'a> {
		let arena = self.ctx.arena();
		match data.kind {
			TypeKind::None => &self.none,
			TypeKind::Unknown => &self.unknown,
			_ => arena.store(data),
		}
	}
}

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
pub struct Type<'a> {
	data: &'a TypeData<'a>,
}

impl<'a> Type<'a> {
	#[inline]
	pub fn context(&self) -> ContextRef<'a> {
		self.data.ctx
	}

	#[inline]
	pub fn store(&self) -> &'a Arena {
		self.context().arena()
	}

	/// Return the invalid type based on the current type.
	///
	/// If this is a valid type, return the type with the same definition
	/// but which is not valid.
	///
	/// For an invalid type, return the type itself.
	///
	/// This will always return the same type when called on the same base type.
	pub fn to_invalid(self) -> Type<'a> {
		if let TypeKind::Invalid(..) = self.data.kind {
			self
		} else {
			let types = self.types();
			types.invalid.get(&self, |typ| {
				let data = TypeData {
					ctx: self.data.ctx,
					kind: TypeKind::Invalid(typ),
				};
				types.store(data)
			})
		}
	}

	/// Is this type valid?
	pub fn is_valid(self) -> bool {
		!self.is_invalid()
	}

	/// Is this type an invalid type?
	pub fn is_invalid(self) -> bool {
		matches!(self.data.kind, TypeKind::Invalid(..))
	}

	/// Return a valid type either by unwrapping an invalid type or returning
	/// self if it is already valid.
	pub fn get_valid(self) -> Type<'a> {
		if let TypeKind::Invalid(typ) = self.data.kind {
			typ
		} else {
			self
		}
	}

	/// Is this the none type?
	pub fn is_none(self) -> bool {
		self == self.types().none()
	}

	/// Is this the unknown type?
	pub fn is_unknown(self) -> bool {
		self == self.types().unknown()
	}

	/// A proper type is not none, unknown, or invalid.
	pub fn is_proper(self) -> bool {
		!(self.is_none() || self.is_invalid() || self.is_unknown())
	}

	/// Create a new unique type sharing the same type definition as the
	/// current type.
	///
	/// Some types such as none and unknown can only have one instance, so
	/// unique will return the same type.
	///
	/// The returned unique type is only equal to itself.
	pub fn to_unique(self) -> Type<'a> {
		let data = self.data.clone();
		let data = self.types().store(data);
		Type { data }
	}

	/// Return the sum of this type with the given type.
	pub fn sum(self, other: Type<'a>) -> Type<'a> {
		let types = self.types();
		let (a, b) = if self < other { (self, other) } else { (other, self) };
		if a.is_invalid() || b.is_invalid() {
			let va = a.get_valid();
			let vb = b.get_valid();
			va.sum(vb).to_invalid()
		} else {
			types.sum_types.get(&(a, b), |(a, b): (Type<'a>, Type<'a>)| {
				if a.is_unknown() || a.is_none() {
					b.data
				} else if b.is_unknown() || b.is_none() {
					a.data
				} else if a.contains(b) {
					a.data
				} else if b.contains(a) {
					b.data
				} else {
					types.store(TypeData {
						ctx: types.ctx,
						kind: TypeKind::Sum(a, b),
					})
				}
			})
		}
	}

	/// Return the intersection of this type with the given type.
	pub fn intersect(&self, _other: Type<'a>) -> Type<'a> {
		todo!()
	}

	/// Return the type resulting from subtracting the given type from the
	/// current type.
	pub fn subtract(&self, _other: Type<'a>) -> Type<'a> {
		todo!()
	}

	/// Is the current type a superset of the given type?
	pub fn contains(self, other: Type<'a>) -> bool {
		if self == other {
			return true;
		}

		if let TypeKind::Never | TypeKind::None = other.data.kind {
			return true;
		}

		if other.is_invalid() {
			return self.contains(other.get_valid());
		}

		match self.data.kind {
			TypeKind::Unit => false,
			TypeKind::None => false,
			TypeKind::Never => false,
			TypeKind::Any => true,
			TypeKind::Builtin(_) => false,
			TypeKind::Unknown => true,
			TypeKind::Invalid(inner) => inner.is_none() || inner.contains(other),
			TypeKind::Sum(a, b) => a.contains(other) || b.contains(other),
		}
	}

	#[inline]
	fn as_ptr(self) -> *const TypeData<'a> {
		self.data.as_ptr()
	}

	#[inline]
	pub fn types(&self) -> &'a TypeContext<'a> {
		self.data.ctx.types()
	}
}

impl<'a> Eq for Type<'a> {}

impl<'a> PartialEq for Type<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl<'a> Hash for Type<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_ptr().hash(state)
	}
}

impl<'a> Debug for Type<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.data)
	}
}

impl<'a> Display for Type<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.data)
	}
}

impl<'a> Ord for Type<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.data.cmp(&other.data)
	}
}

impl<'a> PartialOrd for Type<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Clone, Eq, PartialEq)]
struct TypeData<'a> {
	ctx: ContextRef<'a>,
	kind: TypeKind<'a>,
}

impl<'a> Ord for TypeData<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.kind.cmp(&other.kind)
	}
}

impl<'a> PartialOrd for TypeData<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
enum TypeKind<'a> {
	None,
	Unit,
	Never,
	Any,
	Unknown,
	Invalid(Type<'a>),
	Builtin(Primitive),
	Sum(Type<'a>, Type<'a>),
}

impl<'a> TypeData<'a> {
	fn as_ptr(&self) -> *const Self {
		self
	}

	fn display_id(&self) -> usize {
		(self.as_ptr() as usize / std::mem::size_of::<usize>()) % 0x1000000
	}
}

impl<'a> Debug for TypeData<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut ptr = false;
		let types = self.ctx.types();
		match self.kind {
			TypeKind::None => {
				write!(f, "")?;
				ptr = self.as_ptr() != types.none.as_ptr();
			}
			TypeKind::Unit => {
				write!(f, "()")?;
				ptr = self.as_ptr() != types.unit.as_ptr();
			}
			TypeKind::Never => {
				write!(f, "!")?;
				ptr = self.as_ptr() != types.never.as_ptr();
			}
			TypeKind::Any => {
				write!(f, "any")?;
				ptr = self.as_ptr() != types.any.as_ptr();
			}
			TypeKind::Unknown => {
				write!(f, "???")?;
				ptr = self.as_ptr() != types.unknown.as_ptr();
			}
			TypeKind::Invalid(typ) => {
				if typ != types.none() {
					write!(f, "!!!({typ:?}")?;
				} else {
					write!(f, "!!!")?;
				}
			}
			TypeKind::Builtin(typ) => {
				write!(f, "{typ:?}")?;
				ptr = self.as_ptr() != types.builtin(typ).as_ptr();
			}
			TypeKind::Sum(a, b) => {
				write!(f, "{a:?} | {b:?}")?;
			}
		}
		if ptr {
			write!(f, "#{:06x}", self.display_id())?;
		}
		Ok(())
	}
}

impl<'a> Display for TypeData<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

struct TypeMap<'a, T: Eq + Hash + Clone> {
	map: RwLock<HashMap<T, &'a TypeData<'a>>>,
}

impl<'a, T: Eq + Hash + Clone> TypeMap<'a, T> {
	pub fn new() -> Self {
		Self {
			map: Default::default(),
		}
	}

	pub fn get<F: Fn(T) -> &'a TypeData<'a>>(&self, key: &T, init: F) -> Type<'a> {
		if let Some(data) = self.map.read().unwrap().get(key).copied() {
			return Type { data };
		}

		let mut map = self.map.write().unwrap();
		let entry = map.entry(key.clone()).or_insert_with(|| init(key.clone()));
		Type { data: *entry }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_types() {
		let ctx = Context::new();
		let types = ctx.get().types();
		assert_eq!(types.none(), types.none());
		assert_eq!(types.unit(), types.unit());
		assert_eq!(types.never(), types.never());
		assert_eq!(types.any(), types.any());
		assert_eq!(types.unknown(), types.unknown());
		assert_eq!(types.invalid(), types.invalid());

		assert!(types.invalid().is_invalid());
		assert!(types.unit().to_invalid().is_invalid());

		assert_ne!(types.unit().to_unique(), types.unit());
		assert_eq!(types.none().to_unique(), types.none());
		assert_eq!(types.unknown().to_unique(), types.unknown());

		assert_eq!(types.str(), types.builtin(Primitive::String));
		assert_eq!(types.bool(), types.builtin(Primitive::Bool));
		assert_eq!(types.sint(), types.builtin(Primitive::SInt(64)));
		assert_eq!(types.uint(), types.builtin(Primitive::UInt(64)));
	}
}
