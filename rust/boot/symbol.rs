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
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Symbol {
	data: NonNull<SymbolData>,
}

struct SymbolData {
	len: usize,
	buf: *const u8,
}

unsafe impl Send for SymbolData {}
unsafe impl Sync for SymbolData {}

impl Symbol {
	pub fn empty() -> Symbol {
		static DATA: SymbolData = SymbolData {
			len: 0,
			buf: [0].as_ptr(),
		};
		Symbol { data: (&DATA).into() }
	}

	pub fn new<T: AsRef<str>>(str: T) -> Self {
		static MAP: Init<Table<SymbolKey, SymbolData>> = Init::default();

		let str = str.as_ref();
		if str.len() == 0 {
			return Self::empty();
		}

		let key = SymbolKey::from_str(str);

		let data = MAP.get().get_or_init_ref(&key, |arena, key| {
			debug_assert!(key.own);
			let data = arena.store(SymbolData {
				len: key.len,
				buf: key.buf,
			});
			data
		});

		Symbol { data: data.into() }
	}

	pub fn len(&self) -> usize {
		let data = self.data();
		data.len
	}

	#[inline(always)]
	pub fn as_str(&self) -> &'static str {
		let data = self.data();
		unsafe {
			let data = std::slice::from_raw_parts(data.buf, data.len);
			std::str::from_utf8_unchecked(data)
		}
	}

	fn write_name(&self, f: &mut Formatter) -> Result<()> {
		let str = self.as_str();
		let mut safe = str.len() > 0;
		for chr in str.chars() {
			safe = safe
				&& match chr {
					'A'..='Z' | 'a'..='z' | '0'..='9' | '_' => true,
					_ => false,
				};
			if !safe {
				break;
			}
		}

		if safe { write!(f, "{str}") } else { write!(f, "{str:?}") }?;
		Ok(())
	}

	#[inline(always)]
	fn data(&self) -> &'static SymbolData {
		unsafe { self.data.as_ref() }
	}
}

impl Default for Symbol {
	fn default() -> Self {
		Self::empty()
	}
}

impl<T: AsRef<str>> From<T> for Symbol {
	fn from(value: T) -> Self {
		Symbol::new(value)
	}
}

impl Ord for Symbol {
	fn cmp(&self, other: &Self) -> Ordering {
		if self == other {
			Ordering::Equal
		} else {
			self.as_str().cmp(other.as_str())
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
		self.write_name(f)?;
		write!(f, ")")?;
		Ok(())
	}
}

impl Display for Symbol {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "$")?;
		self.write_name(f)?;
		Ok(())
	}
}

struct SymbolKey {
	buf: *const u8,
	len: usize,
	own: bool,
}

impl SymbolKey {
	#[inline(always)]
	pub fn from_str(str: &str) -> SymbolKey {
		SymbolKey {
			buf: str.as_ptr(),
			len: str.len(),
			own: false,
		}
	}

	#[inline(always)]
	pub fn as_str(&self) -> &str {
		unsafe {
			let buf = std::slice::from_raw_parts(self.buf, self.len);
			std::str::from_utf8_unchecked(buf)
		}
	}
}

impl Clone for SymbolKey {
	fn clone(&self) -> Self {
		if self.own {
			Self {
				buf: self.buf,
				len: self.len,
				own: true,
			}
		} else {
			let str = Arena::get().str(self.as_str());
			Self {
				buf: str.as_ptr(),
				len: self.len,
				own: true,
			}
		}
	}
}

impl Eq for SymbolKey {}

impl PartialEq for SymbolKey {
	fn eq(&self, other: &Self) -> bool {
		self.as_str().eq(other.as_str())
	}
}

impl Hash for SymbolKey {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_str().hash(state)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_symbols() {
		assert_eq!(Symbol::empty(), Symbol::empty());
		assert_eq!(Symbol::empty(), Symbol::default());
		assert_eq!(Symbol::empty(), Symbol::new(""));

		assert_eq!(0, Symbol::empty().len());
		assert_eq!(4, Symbol::new("1234").len());

		assert_eq!(Symbol::new("abc"), Symbol::new("abc"));
		assert_eq!(Symbol::new("123"), Symbol::new("123"));

		assert_eq!(Symbol::new("abc").data.as_ptr(), Symbol::new("abc").data.as_ptr());
		assert_eq!(Symbol::new("123").data.as_ptr(), Symbol::new("123").data.as_ptr());

		assert_eq!("$abc", Symbol::new("abc").to_string());
		assert_eq!("$123", Symbol::new("123").to_string());
	}
}
