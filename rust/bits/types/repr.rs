use super::*;

/// Describes the concrete underlying value for a [`Type`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DataRepr<'a> {
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
	Ptr(Type<'a>),
	/// Reference to an specific type. A reference is basically a pointer that
	/// can never be null.
	Ref(Type<'a>),
	/// A value holding a type reference.
	Type,
	/// A value holding a reference for a specific base type and its sub-types.
	TypeOf(Type<'a>),
	/// Plain function value.
	Func(Type<'a>),
	/// Record composite.
	Record(&'a [Type<'a>]),
	/// Untagged union type.
	Union(&'a [Type<'a>]),
	/// Fixed array.
	Array(usize, Type<'a>),
	/// Slice type.
	Slice(Type<'a>),
}

/// Primitive data types.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Primitive {
	/// Boolean
	Bool,
	/// Generic string
	String,
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

impl<'a> Type<'a> {
	pub fn is_builtin(&self, typ: Primitive) -> bool {
		if let Some(&DataRepr::Builtin(repr)) = self.repr() {
			repr == typ
		} else {
			false
		}
	}
}

impl<'a> TypeContext<'a> {
	pub fn builtin(&'a self, typ: Primitive) -> Type<'a> {
		self.builtin.get(&typ, |typ| self.store(self.from_primitive(typ)))
	}

	fn from_primitive(&self, typ: Primitive) -> TypeData<'a> {
		TypeData {
			ctx: self.ctx,
			kind: TypeKind::Builtin(typ),
			repr: Some(DataRepr::Builtin(typ)),
			debug_value: |v, f| Self::debug_builtin(v, f),
		}
	}

	fn debug_builtin(val: Value, f: &mut Formatter) -> std::fmt::Result {
		if let Some(DataRepr::Builtin(typ)) = val.get_type().repr() {
			let dt = val.data();
			match typ {
				Primitive::Bool => write!(f, "{}", dt.bool()),
				Primitive::String => write!(f, "{:?}", unsafe { dt.str() }),
				Primitive::SInt(8) => write!(f, "{}", dt.i8()),
				Primitive::SInt(16) => write!(f, "{}", dt.i16()),
				Primitive::SInt(32) => write!(f, "{}", dt.i32()),
				Primitive::UInt(8) => write!(f, "{}", dt.u8()),
				Primitive::UInt(16) => write!(f, "{}", dt.u16()),
				Primitive::UInt(32) => write!(f, "{}", dt.u32()),
				Primitive::SInt(0..=64) => write!(f, "{}", dt.i64()),
				Primitive::UInt(0..=64) => write!(f, "{}", dt.u64()),
				Primitive::SInt(n) => todo!("SInt({n}) not implemented"),
				Primitive::UInt(n) => todo!("UInt({n}) not implemented"),
				Primitive::UIntSize => write!(f, "{}", dt.usize()),
				Primitive::SIntSize => write!(f, "{}", dt.isize()),
				Primitive::Char => write!(f, "{}", dt.char()),
				Primitive::Float32 => write!(f, "{:?}", dt.f32()),
				Primitive::Float64 => write!(f, "{:?}", dt.f64()),
				Primitive::Pointer => write!(f, "{:?}", dt.ptr()),
				Primitive::UIntPtr => write!(f, "{:?}", dt.usize()),
				Primitive::SIntPtr => write!(f, "{:?}", dt.isize()),
				Primitive::PtrDiff => write!(f, "{:?}", dt.isize()),
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
			Self::String => write!(f, "string"),
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
		let ctx = Context::new();
		let types = ctx.get().types();
		let u32 = types.builtin(Primitive::UInt(32));
		assert_eq!(u32, types.builtin(Primitive::UInt(32)));
		assert_ne!(u32, types.builtin(Primitive::UInt(64)));
	}
}
