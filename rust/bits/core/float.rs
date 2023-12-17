use super::*;

#[derive(Copy, Clone, Debug)]
pub enum Float {
	F32(f32),
	F64(f64),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum FloatKind {
	F32,
	F64,
}

impl FloatKind {
	pub fn bits(&self) -> u8 {
		match self {
			FloatKind::F32 => 32,
			FloatKind::F64 => 64,
		}
	}

	pub fn is_valid(&self, other: &FloatKind) -> bool {
		self.bits() <= other.bits()
	}
}

impl Float {
	pub fn kind(&self) -> FloatKind {
		match self {
			Float::F32(_) => FloatKind::F32,
			Float::F64(_) => FloatKind::F64,
		}
	}

	#[inline]
	pub fn as_f64(&self) -> f64 {
		match self {
			&Float::F32(v) => v as f64,
			&Float::F64(v) => v,
		}
	}
}

impl Eq for Float {}

impl PartialEq for Float {
	fn eq(&self, other: &Self) -> bool {
		let (lhs, rhs) = (self.as_f64(), other.as_f64());
		if lhs == rhs {
			true
		} else if lhs.is_nan() {
			rhs.is_nan()
		} else {
			false
		}
	}
}

impl Hash for Float {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		let val = self.as_f64();
		if val == 0.0 {
			0.hash(state)
		} else if val.is_nan() {
			f64_bits(f64::NAN).hash(state)
		} else {
			f64_bits(val).hash(state)
		}
	}
}

impl Ord for Float {
	fn cmp(&self, other: &Self) -> Ordering {
		let lhs = self.as_f64();
		let rhs = other.as_f64();
		if lhs < rhs {
			Ordering::Less
		} else if rhs < lhs {
			Ordering::Greater
		} else {
			Ordering::Equal
		}
	}
}

impl PartialOrd for Float {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Display for Float {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		match self {
			&Float::F32(v) => {
				let v = if v == 0.0 { 0.0 } else { v };
				write!(f, "{v}")
			}
			&Float::F64(v) => {
				let v = if v == 0.0 { 0.0 } else { v };
				write!(f, "{v}")
			}
		}
	}
}

#[inline]
fn f64_bits(val: f64) -> u64 {
	unsafe { std::mem::transmute(val) }
}

macro_rules! from_float {
	($t:ty, $id:ident) => {
		impl From<$t> for Float {
			fn from(value: $t) -> Float {
				Float::$id(value)
			}
		}

		impl From<$t> for Value {
			fn from(value: $t) -> Value {
				Value::Float(value.into())
			}
		}
	};
}

from_float!(f32, F32);
from_float!(f64, F64);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compare() {
		let a = [Float::F64(1.25), Float::F32(1.25)];
		let b = [Float::F64(1.00), Float::F32(1.00)];
		let c = [Float::F64(-1.00), Float::F32(-1.00)];

		for a1 in a.iter() {
			let mut vals = HashMap::new();
			vals.insert(*a1, "A");

			for a2 in a.iter() {
				assert_eq!(a1, a2);
				assert_eq!(Ordering::Equal, a1.cmp(a2));
				assert_eq!("A", vals[a2]);
			}

			for x in b.iter().chain(c.iter()) {
				assert_ne!(a1, x);
				assert_eq!(Ordering::Greater, a1.cmp(x));
				assert_eq!(Ordering::Less, x.cmp(a1));
			}
		}

		for b1 in b.iter() {
			let mut vals = HashMap::new();
			vals.insert(*b1, "B");

			for b2 in b.iter() {
				assert_eq!(b1, b2);
				assert_eq!(Ordering::Equal, b1.cmp(b2));
				assert_eq!("B", vals[b2]);
			}

			for x in c.iter() {
				assert_ne!(b1, x);
				assert_eq!(Ordering::Greater, b1.cmp(x));
				assert_eq!(Ordering::Less, x.cmp(b1));
			}
		}

		for c1 in c.iter() {
			let mut vals = HashMap::new();
			vals.insert(*c1, "C");

			for c2 in c.iter() {
				assert_eq!(c1, c2);
				assert_eq!(Ordering::Equal, c1.cmp(c2));
				assert_eq!("C", vals[c2]);
			}

			for x in a.iter().chain(b.iter()) {
				assert_ne!(c1, x);
				assert_eq!(Ordering::Less, c1.cmp(x));
				assert_eq!(Ordering::Greater, x.cmp(c1));
			}
		}
	}

	#[test]
	fn zero_and_minus_zero() {
		const POS64: f64 = 0.0;
		const NEG64: f64 = -0.0;

		const POS32: f32 = 0.0;
		const NEG32: f32 = -0.0;

		assert!(f64_bits(POS64) != f64_bits(NEG64));
		assert!(f64_bits(POS32 as f64) != f64_bits(NEG32 as f64));

		let p64 = Float::F64(POS64);
		let n64 = Float::F64(NEG64);

		let p32 = Float::F32(POS32);
		let n32 = Float::F32(NEG32);

		assert_eq!(p64, n64);
		assert_eq!(p32, n32);

		assert_eq!(p64.to_string(), n64.to_string());
		assert_eq!(p32.to_string(), n32.to_string());

		assert_eq!("F64(-0.0)", format!("{n64:?}"));
		assert_eq!("F32(-0.0)", format!("{n32:?}"));

		for key in [p64, p32, n64, n32] {
			assert_eq!(Ordering::Equal, key.cmp(&n64));
			assert_eq!(Ordering::Equal, key.cmp(&n32));
			assert_eq!(Ordering::Equal, key.cmp(&p64));
			assert_eq!(Ordering::Equal, key.cmp(&p32));

			assert_eq!(Ordering::Equal, n64.cmp(&key));
			assert_eq!(Ordering::Equal, n32.cmp(&key));
			assert_eq!(Ordering::Equal, p64.cmp(&key));
			assert_eq!(Ordering::Equal, p32.cmp(&key));

			let mut vals = HashMap::new();
			vals.insert(key, "A");

			assert_eq!("A", vals[&p64]);
			assert_eq!("A", vals[&n64]);
			assert_eq!("A", vals[&p32]);
			assert_eq!("A", vals[&n32]);
		}
	}

	#[test]
	fn nan() {
		let a1 = Float::F32(f32::NAN);
		let b1 = Float::F64(f64::NAN);

		let a2 = Float::F32(-(0.0 / 0.0));
		let b2 = Float::F64(-(0.0 / 0.0));

		assert!(f64_bits(a1.as_f64()) != f64_bits(a2.as_f64()));

		for key in [a1, a2, b1, b2] {
			assert_eq!(Ordering::Equal, key.cmp(&a1));
			assert_eq!(Ordering::Equal, key.cmp(&a2));
			assert_eq!(Ordering::Equal, key.cmp(&b1));
			assert_eq!(Ordering::Equal, key.cmp(&b2));

			assert_eq!(Ordering::Equal, a1.cmp(&key));
			assert_eq!(Ordering::Equal, a2.cmp(&key));
			assert_eq!(Ordering::Equal, b1.cmp(&key));
			assert_eq!(Ordering::Equal, b2.cmp(&key));

			let mut vals = HashMap::new();
			vals.insert(key, "A");

			assert_eq!("A", vals[&a1]);
			assert_eq!("A", vals[&a2]);
			assert_eq!("A", vals[&b1]);
			assert_eq!("A", vals[&b2]);
		}
	}
}
