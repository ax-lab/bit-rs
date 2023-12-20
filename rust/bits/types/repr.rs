use super::*;

/// Describes the concrete underlying value for a [`Type`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DataRepr {
	/// Zero sized.
	Empty,
	/// A builtin primitive value.
	Builtin(Primitive),
	/// A value storing a symbol.
	Symbol(Symbol),
	/// Unspecified integer value. Includes signed and unsigned.
	Integer,
	/// Unspecified unsigned integer type.
	Unsigned,
	/// Unspecified numeric type. Includes integers, decimals, and floats.
	Number,
	/// Default string representation.
	String,
	/// Pointer to an specific type.
	Ptr(Type),
	/// Reference to an specific type. A reference is basically a pointer that
	/// can never be null.
	Ref(Type),
	/// A value holding a type reference.
	Type,
	/// A value holding a reference for a specific base type and its sub-types.
	TypeOf(Type),
	/// Plain function value.
	Func(Type),
	/// Record composite.
	Record(&'static [Type]),
	/// Untagged union type.
	Union(&'static [Type]),
	/// Fixed array.
	Array(usize, Type),
	/// Slice type.
	Slice(Type),
}

/// Primitive data types.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Primitive {
	/// Boolean
	Bool,
	/// Fixed size signed int. The zero size is the best native integer.
	SInt(u8),
	/// Fixed size unsigned int. The zero size is the best native integer.
	UInt(u8),
	/// Unsigned int capable of holding any representable memory size.
	///
	/// Note that this can be less than the minimum size to hold a pointer
	/// due to segmented architectures and pointer provenance.
	UIntSize,
	/// Signed int equivalent of an [`Primitive::UIntSize`].
	SIntSize,
	/// Unicode character.
	Char,
	/// Single precision floating point (32 bits IEEE 754).
	Float32,
	/// Double precision floating point (64 bits IEEE 754).
	Float64,
	/// Arbitrary pointer to anything.
	Pointer,
	/// An unsigned integer that can be safely converted to and from a pointer.
	///
	/// This has enough bits to hold any pointer address and additional pointer
	/// provenance.
	UIntPtr,
	/// Signed integer equivalent of a [`Primitive::UIntPtr`]
	SIntPtr,
	/// Signed integer type with the result of subtracting two pointers.
	PtrDiff,
}

impl Type {
	pub fn builtin(typ: Primitive) -> Self {
		static MAP: TypeMap<Primitive> = TypeMap::new();
		MAP.get(&typ, |typ| Self::from_primitive(typ))
	}

	fn from_primitive(typ: Primitive) -> TypeData {
		TypeData {
			kind: TypeKind::Builtin(typ),
			repr: Some(DataRepr::Builtin(typ)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn builtin_types() {
		let u32 = Type::builtin(Primitive::UInt(32));
		assert_eq!(u32, Type::builtin(Primitive::UInt(32)));
		assert_ne!(u32, Type::builtin(Primitive::UInt(64)));
	}
}
