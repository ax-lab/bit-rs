use std::alloc::*;

use super::*;

/// Represents an arbitrary globally unique identifier stored as an interned
/// array of bytes.
///
/// A symbol is only equal to itself. Since symbols are interned, comparison
/// and hashing uses plain pointer comparison.
///
/// Symbols are ordered lexicographically.
///
/// The meaning of the symbol bytes (e.g. string, UUID, integer) is left to
/// be specified by the environment.
#[derive(Copy, Clone)]
pub struct Symbol {
	data: &'static SymbolData,
}

impl Symbol {
	pub fn empty() -> Symbol {
		static DATA: SymbolData = SymbolData {
			len: 0,
			cnt: SymbolCounter::new(),
			buf: [0],
		};
		Symbol { data: &DATA }
	}

	pub fn unique(str: &str) -> Self {
		Self::str(str).to_unique()
	}

	pub fn str(str: &str) -> Self {
		Self::get(str.as_bytes())
	}

	pub fn get(bytes: &[u8]) -> Self {
		static MAP: OnceLock<RwLock<HashMap<&'static [u8], &'static SymbolData>>> = OnceLock::new();
		if bytes.len() == 0 {
			Self::empty()
		} else {
			let map = MAP.get_or_init(|| Default::default());
			if let Some(data) = map.read().unwrap().get(bytes).copied() {
				return Self { data };
			}

			let mut map = map.write().unwrap();
			if let Some(data) = map.get(bytes).copied() {
				return Self { data };
			}

			let data = SymbolData::store(bytes, SymbolCounter::new());
			map.insert(data.as_bytes(), data);
			Self { data }
		}
	}

	pub fn to_unique(&self) -> Symbol {
		let next = self.data.cnt.next();
		let data = SymbolData::store(self.as_bytes(), next);
		Self { data }
	}

	pub fn len(&self) -> usize {
		self.data.len
	}

	pub fn as_bytes(&self) -> &'static [u8] {
		self.data.as_bytes()
	}

	pub fn as_str(&self) -> Result<&'static str> {
		std::str::from_utf8(self.as_bytes()).raise()
	}

	pub fn as_ptr(&self) -> *const u8 {
		self.data.buf.as_ptr()
	}

	pub fn write_name(&self, f: &mut Formatter, allow_brackets: bool) -> std::fmt::Result {
		if let Ok(str) = self.as_str() {
			if allow_brackets {
				write!(f, "{str:?}")
			} else {
				write!(f, "{str}")
			}
		} else {
			if allow_brackets {
				write!(f, "(")?;
			}
			write!(f, "{:?}", self.as_bytes())?;
			if allow_brackets {
				write!(f, ")")?;
			}
			Ok(())
		}
	}
}

impl Default for Symbol {
	fn default() -> Self {
		Self::empty()
	}
}

impl<T: AsRef<str>> From<T> for Symbol {
	fn from(value: T) -> Self {
		Symbol::str(value.as_ref())
	}
}

impl Eq for Symbol {}

impl PartialEq for Symbol {
	fn eq(&self, other: &Self) -> bool {
		self.data.as_ptr() == other.data.as_ptr()
	}
}

impl Hash for Symbol {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.data.as_ptr().hash(state);
	}
}

impl Ord for Symbol {
	fn cmp(&self, other: &Self) -> Ordering {
		if self == other {
			Ordering::Equal
		} else {
			self.as_bytes().cmp(other.as_bytes())
		}
	}
}

impl PartialOrd for Symbol {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Debug for Symbol {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "Symbol(")?;
		self.write_name(f, false)?;

		let cnt = self.data.counter();
		if cnt > 0 {
			write!(f, "${cnt}")?;
		}

		write!(f, ")")
	}
}

impl Display for Symbol {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		self.write_name(f, true)?;
		write!(f, "$")?;

		let cnt = self.data.counter();
		if cnt > 0 {
			write!(f, "{cnt}")?;
		}

		Ok(())
	}
}

struct SymbolData {
	len: usize,
	cnt: SymbolCounter,
	buf: [u8; 1],
}

enum SymbolCounter {
	Zero(AtomicU32),
	Uniq(u32, &'static AtomicU32),
}

impl SymbolCounter {
	pub const fn new() -> Self {
		Self::Zero(AtomicU32::new(1))
	}

	pub fn next(&'static self) -> Self {
		match self {
			SymbolCounter::Zero(counter) | &SymbolCounter::Uniq(_, counter) => {
				let count = counter.fetch_add(1, SyncOrder::Relaxed);
				SymbolCounter::Uniq(count, counter)
			}
		}
	}
}

impl SymbolData {
	fn as_bytes(&self) -> &'static [u8] {
		let len = self.len;
		let buf = &self.buf as *const [u8] as *const u8;
		unsafe { std::slice::from_raw_parts(buf, len) }
	}

	fn as_ptr(&self) -> *const Self {
		self as *const Self
	}

	fn counter(&self) -> u32 {
		match self.cnt {
			SymbolCounter::Zero(_) => 0,
			SymbolCounter::Uniq(n, _) => n,
		}
	}

	fn store(bytes: &[u8], cnt: SymbolCounter) -> &'static Self {
		let count = bytes.len();
		let size = std::mem::size_of::<SymbolData>() + count - 1;
		let align = std::mem::align_of::<Self>();
		let layout = Layout::from_size_align(size, align).unwrap();
		let data = unsafe {
			let ptr = alloc(layout) as *mut Self;
			&mut *ptr
		};
		data.len = count;
		data.cnt = cnt;
		unsafe {
			std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.buf.as_mut_ptr(), count);
		}
		data
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_symbols() {
		assert_eq!(Symbol::empty(), Symbol::empty());
		assert_eq!(Symbol::empty(), Symbol::default());
		assert_eq!(Symbol::empty(), Symbol::str(""));

		assert_eq!(0, Symbol::empty().len());
		assert_eq!(4, Symbol::str("1234").len());

		assert_eq!(Symbol::str("abc"), Symbol::str("abc"));
		assert_eq!(Symbol::str("123"), Symbol::str("123"));

		assert_eq!(Symbol::str("abc").as_ptr(), Symbol::str("abc").as_ptr());
		assert_eq!(Symbol::str("123").as_ptr(), Symbol::str("123").as_ptr());

		assert_eq!("\"abc\"$", Symbol::str("abc").to_string());
		assert_eq!("\"123\"$", Symbol::str("123").to_string());
		assert_eq!("\"abc\"$", Symbol::get("abc".as_bytes()).to_string());
	}

	#[test]
	fn unique_symbols() {
		let a0 = Symbol::empty();
		let a1 = a0.to_unique();
		let a2 = a0.to_unique();
		let a3 = a1.to_unique();

		let b0 = Symbol::str("abc");
		let b1 = b0.to_unique();
		let b2 = b0.to_unique();
		let b3 = b1.to_unique();

		assert_ne!(a0, a1);
		assert_ne!(a0, a2);
		assert_ne!(a0, a3);
		assert_ne!(a1, a2);
		assert_ne!(a1, a3);
		assert_ne!(a2, a3);

		assert_ne!(b0, b1);
		assert_ne!(b0, b2);
		assert_ne!(b0, b3);
		assert_ne!(b1, b2);
		assert_ne!(b1, b3);
		assert_ne!(b2, b3);

		assert_eq!("\"\"$", a0.to_string());
		assert_eq!("\"\"$1", a1.to_string());
		assert_eq!("\"\"$2", a2.to_string());
		assert_eq!("\"\"$3", a3.to_string());

		assert_eq!("\"abc\"$", b0.to_string());
		assert_eq!("\"abc\"$1", b1.to_string());
		assert_eq!("\"abc\"$2", b2.to_string());
		assert_eq!("\"abc\"$3", b3.to_string());
	}
}
