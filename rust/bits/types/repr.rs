use super::*;

/// Primitive data types.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Primitive {
	/// Zero sized type.
	Empty,
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
		if let TypeKind::Builtin(repr) = self.data.kind {
			repr == typ
		} else {
			false
		}
	}
}

impl<'a> TypeContext<'a> {
	pub fn builtin(&'a self, typ: Primitive) -> Type<'a> {
		match typ {
			Primitive::Bool => self.bool(),
			Primitive::String => self.str(),
			Primitive::SInt(64) => self.sint(),
			Primitive::UInt(64) => self.uint(),
			_ => self.builtin.get(&typ, |typ| self.store(self.from_primitive(typ))),
		}
	}

	fn from_primitive(&self, typ: Primitive) -> TypeData<'a> {
		TypeData {
			ctx: self.ctx,
			kind: TypeKind::Builtin(typ),
		}
	}
}

impl Debug for Primitive {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Empty => write!(f, "()"),
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
