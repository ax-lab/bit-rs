use super::*;

pub const I8: Kind = Kind::Int(IntKind::I8);
pub const U8: Kind = Kind::Int(IntKind::U8);
pub const I16: Kind = Kind::Int(IntKind::I16);
pub const I32: Kind = Kind::Int(IntKind::I32);
pub const I64: Kind = Kind::Int(IntKind::I64);
pub const U16: Kind = Kind::Int(IntKind::U16);
pub const U32: Kind = Kind::Int(IntKind::U32);
pub const U64: Kind = Kind::Int(IntKind::U64);

#[derive(Copy, Clone, Debug)]
pub enum Int {
	I8(i8),
	I16(i16),
	I32(i32),
	I64(i64),
	U8(u8),
	U16(u16),
	U32(u32),
	U64(u64),
	Lit(i128),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum IntKind {
	I8,
	I16,
	I32,
	I64,
	U8,
	U16,
	U32,
	U64,
	Lit,
}

impl IntKind {
	pub fn signed(&self) -> bool {
		match self {
			IntKind::I8 | IntKind::I16 | IntKind::I32 | IntKind::I64 => true,
			IntKind::U8 | IntKind::U16 | IntKind::U32 | IntKind::U64 => false,
			IntKind::Lit => true,
		}
	}

	pub fn bits(&self) -> u8 {
		match self {
			IntKind::I8 | IntKind::U8 => 8,
			IntKind::I16 | IntKind::U16 => 16,
			IntKind::I32 | IntKind::U32 => 32,
			IntKind::I64 | IntKind::U64 => 64,
			IntKind::Lit => 0,
		}
	}

	pub fn is_valid(&self, other: &IntKind) -> bool {
		if self.signed() == other.signed() {
			other.bits() >= self.bits()
		} else {
			other.bits() > self.bits()
		}
	}
}

impl Int {
	pub fn kind(&self) -> IntKind {
		match self {
			Int::I8(_) => IntKind::I8,
			Int::I16(_) => IntKind::I16,
			Int::I32(_) => IntKind::I32,
			Int::I64(_) => IntKind::I64,
			Int::U8(_) => IntKind::U8,
			Int::U16(_) => IntKind::U16,
			Int::U32(_) => IntKind::U32,
			Int::U64(_) => IntKind::U64,
			Int::Lit(_) => IntKind::Lit,
		}
	}

	pub fn as_usize(&self) -> usize {
		self.as_u64().expect("value is not a valid usize") as usize
	}

	pub fn as_i64(&self) -> Option<i64> {
		let val = match *self {
			Int::I64(v) => v,
			Int::I8(v) => v as i64,
			Int::I16(v) => v as i64,
			Int::I32(v) => v as i64,
			Int::U8(v) => v as i64,
			Int::U16(v) => v as i64,
			Int::U32(v) => v as i64,
			Int::U64(v) => {
				if v > i64::MAX as u64 {
					return None;
				}
				v as i64
			}
			Int::Lit(v) => {
				if v > i64::MAX as i128 {
					return None;
				}
				v as i64
			}
		};
		Some(val)
	}

	pub fn as_u64(&self) -> Option<u64> {
		let val = match *self {
			Int::U64(v) => v,
			Int::U8(v) => v as u64,
			Int::U16(v) => v as u64,
			Int::U32(v) => v as u64,
			Int::I8(v) => {
				if v < 0 {
					return None;
				}
				v as u64
			}
			Int::I16(v) => {
				if v < 0 {
					return None;
				}
				v as u64
			}
			Int::I32(v) => {
				if v < 0 {
					return None;
				}
				v as u64
			}
			Int::I64(v) => {
				if v < 0 {
					return None;
				}
				v as u64
			}
			Int::Lit(v) => {
				if v < 0 || v > u64::MAX as i128 {
					return None;
				}
				v as u64
			}
		};
		Some(val)
	}

	pub fn sign_abs(&self) -> (i32, u128) {
		match *self {
			Int::I8(v) => {
				let s = v.signum() as i32;
				(s, v.abs() as u128)
			}
			Int::I16(v) => {
				let s = v.signum() as i32;
				(s, v.abs() as u128)
			}
			Int::I32(v) => {
				let s = v.signum() as i32;
				(s, v.abs() as u128)
			}
			Int::I64(v) => {
				let s = v.signum() as i32;
				(s, v.abs() as u128)
			}
			Int::U8(v) => {
				let s = if v > 0 { 1 } else { 0 };
				(s, v as u128)
			}
			Int::U16(v) => {
				let s = if v > 0 { 1 } else { 0 };
				(s, v as u128)
			}
			Int::U32(v) => {
				let s = if v > 0 { 1 } else { 0 };
				(s, v as u128)
			}
			Int::U64(v) => {
				let s = if v > 0 { 1 } else { 0 };
				(s, v as u128)
			}
			Int::Lit(v) => {
				let s = if v > 0 { 1 } else { 0 };
				(s, v.abs() as u128)
			}
		}
	}
}

impl Display for Int {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			Int::I8(v) => write!(f, "{v}"),
			Int::I16(v) => write!(f, "{v}"),
			Int::I32(v) => write!(f, "{v}"),
			Int::I64(v) => write!(f, "{v}"),
			Int::U8(v) => write!(f, "{v}"),
			Int::U16(v) => write!(f, "{v}"),
			Int::U32(v) => write!(f, "{v}"),
			Int::U64(v) => write!(f, "{v}"),
			Int::Lit(v) => write!(f, "{v}"),
		}
	}
}

impl Eq for Int {}

impl PartialEq for Int {
	fn eq(&self, other: &Self) -> bool {
		self.sign_abs() == other.sign_abs()
	}
}

impl Ord for Int {
	fn cmp(&self, other: &Self) -> Ordering {
		self.sign_abs().cmp(&other.sign_abs())
	}
}

impl PartialOrd for Int {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Hash for Int {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.sign_abs().hash(state)
	}
}

macro_rules! from_int {
	($t:ty, $id:ident) => {
		impl From<$t> for Int {
			fn from(value: $t) -> Int {
				Int::$id(value)
			}
		}

		impl From<$t> for Value {
			fn from(value: $t) -> Value {
				Value::Int(value.into())
			}
		}
	};
}

from_int!(i8, I8);
from_int!(u8, U8);
from_int!(i16, I16);
from_int!(i32, I32);
from_int!(i64, I64);
from_int!(u16, U16);
from_int!(u32, U32);
from_int!(u64, U64);

impl From<usize> for Int {
	fn from(value: usize) -> Self {
		Int::U64(value as u64)
	}
}

impl From<usize> for Value {
	fn from(value: usize) -> Self {
		Value::Int(value.into())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn equality() {
		let a = [
			Int::I8(69),
			Int::I16(69),
			Int::I32(69),
			Int::I64(69),
			Int::U8(69),
			Int::U16(69),
			Int::U32(69),
			Int::U64(69),
		];

		let b = [
			Int::I8(42),
			Int::I16(42),
			Int::I32(42),
			Int::I64(42),
			Int::U8(42),
			Int::U16(42),
			Int::U32(42),
			Int::U64(42),
		];

		for a1 in a.iter() {
			let mut vals = HashMap::new();
			vals.insert(*a1, "A");

			for a2 in a.iter() {
				assert_eq!(a1, a2);
				assert_eq!(Ordering::Equal, a1.cmp(a2));
				assert_eq!("A", vals[a2]);
			}

			for x in b.iter() {
				assert_ne!(a1, x);
				assert_eq!(Ordering::Greater, a1.cmp(x));
				assert_eq!(Ordering::Less, x.cmp(a1));
				assert!(vals.get(x).is_none());
			}
		}
	}
}
