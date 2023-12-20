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
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

	pub fn is_builtin(&self, typ: Primitive) -> bool {
		if let Some(&DataRepr::Builtin(repr)) = self.repr() {
			repr == typ
		} else {
			false
		}
	}

	fn from_primitive(typ: Primitive) -> TypeData {
		TypeData {
			kind: TypeKind::Builtin(typ),
			repr: Some(DataRepr::Builtin(typ)),
			debug_value: |v, f| Self::fmt_builtin_value(true, v, f),
			display_value: Some(|v, f| Self::fmt_builtin_value(false, v, f)),
		}
	}

	fn fmt_builtin_value(debug: bool, v: Value, f: &mut Formatter) -> std::fmt::Result {
		let _ = debug;
		if let Some(DataRepr::Builtin(typ)) = v.get_type().repr() {
			let v = v.data();
			match typ {
				Primitive::Bool => write!(f, "{}", v.bool()),
				Primitive::SInt(8) => write!(f, "{}", v.i8()),
				Primitive::SInt(16) => write!(f, "{}", v.i16()),
				Primitive::SInt(32) => write!(f, "{}", v.i32()),
				Primitive::UInt(8) => write!(f, "{}", v.u8()),
				Primitive::UInt(16) => write!(f, "{}", v.u16()),
				Primitive::UInt(32) => write!(f, "{}", v.u32()),
				Primitive::SInt(0..=64) => write!(f, "{}", v.i64()),
				Primitive::UInt(0..=64) => write!(f, "{}", v.u64()),
				Primitive::SInt(n) => todo!("SInt({n}) not implemented"),
				Primitive::UInt(n) => todo!("UInt({n}) not implemented"),
				Primitive::UIntSize => write!(f, "{}", v.usize()),
				Primitive::SIntSize => write!(f, "{}", v.isize()),
				Primitive::Char => write!(f, "{}", v.char()),
				Primitive::Float32 => write!(f, "{:?}", v.f32()),
				Primitive::Float64 => write!(f, "{:?}", v.f64()),
				Primitive::Pointer => write!(f, "{:?}", v.ptr()),
				Primitive::UIntPtr => write!(f, "{:?}", v.usize()),
				Primitive::SIntPtr => write!(f, "{:?}", v.isize()),
				Primitive::PtrDiff => write!(f, "{:?}", v.isize()),
			}
		} else {
			unreachable!("invalid value")
		}
	}
}

impl Debug for Primitive {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Bool => write!(f, "bool"),
			Self::SInt(n) => write!(f, "i{n}"),
			Self::UInt(n) => write!(f, "u{n}"),
			Self::SIntSize => write!(f, "isize"),
			Self::UIntSize => write!(f, "usize"),
			Self::Char => write!(f, "char"),
			Self::Float32 => write!(f, "f32"),
			Self::Float64 => write!(f, "f64"),
			Self::Pointer => write!(f, "ptr"),
			Self::UIntPtr => write!(f, "uintptr"),
			Self::SIntPtr => write!(f, "intptr"),
			Self::PtrDiff => write!(f, "ptr_diff"),
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
