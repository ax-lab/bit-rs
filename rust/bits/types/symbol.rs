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
		static DATA: SymbolData = SymbolData { len: 0, buf: [0] };
		Symbol { data: &DATA }
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

			let data = SymbolData::store(bytes);
			map.insert(data.as_bytes(), data);
			Self { data }
		}
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
		if let Ok(str) = self.as_str() {
			write!(f, "Symbol({str:?})")
		} else {
			write!(f, "Symbol({:?})", self.as_bytes())
		}
	}
}

impl Display for Symbol {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		if let Ok(str) = self.as_str() {
			write!(f, "#{str:?}")
		} else {
			write!(f, "#({:?})", self.as_bytes())
		}
	}
}

struct SymbolData {
	len: usize,
	buf: [u8; 1],
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

	fn store(bytes: &[u8]) -> &'static Self {
		let count = bytes.len();
		let size = std::mem::size_of::<usize>() + count;
		let align = std::mem::align_of::<Self>();
		let layout = Layout::from_size_align(size, align).unwrap();
		let data = unsafe {
			let ptr = alloc(layout) as *mut Self;
			&mut *ptr
		};
		data.len = count;
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

		assert_eq!("#\"abc\"", Symbol::str("abc").to_string());
		assert_eq!("#\"123\"", Symbol::str("123").to_string());
		assert_eq!("#\"abc\"", Symbol::get("abc".as_bytes()).to_string());
	}
}
