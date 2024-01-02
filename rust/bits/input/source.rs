use std::path::{Path, PathBuf};

use super::*;

const TAB_SIZE: usize = 4;

pub struct SourceContext<'a> {
	ctx: ContextRef<'a>,
	map: SourceMap<'a>,
}

impl<'a> IsContext<'a> for SourceContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		Self {
			ctx,
			map: SourceMap::new(".").unwrap(),
		}
	}
}

impl<'a> SourceContext<'a> {
	pub fn set_base_dir<T: AsRef<Path>>(&self, base_dir: T) -> Result<PathBuf> {
		let base_dir = norm_path(base_dir, "base path")?;
		let previous = std::mem::replace(&mut *self.map.base_dir.write().unwrap(), base_dir);
		Ok(previous)
	}

	pub fn from_string<T: Into<String>, U: Into<String>>(&self, name: T, text: U) -> Source<'a> {
		let data = SourceData {
			name: name.into(),
			text: text.into(),
			path: None,
			tabs: TAB_SIZE.into(),
		};
		let data = self.ctx.store(data);
		Source { data }
	}

	pub fn load_file<T: AsRef<Path>>(&self, path: T) -> Result<Source> {
		let path = path.as_ref();
		let base_dir = self.map.base_dir.read().unwrap().clone();
		let full_path = get_full_path(&base_dir, path)?;

		let sources = self.map.sources.read().unwrap();
		if let Some(src) = sources.get(&full_path) {
			src.clone()
		} else {
			drop(sources);

			let mut sources = self.map.sources.write().unwrap();
			let entry = sources.entry(full_path).or_insert_with_key(|full_path| {
				let name = full_path
					.strip_prefix(&base_dir)
					.unwrap_or(full_path)
					.to_string_lossy()
					.into();
				let text = std::fs::read_to_string(&full_path).map_err(|err| err!(="loading `{name}`: {err}"))?;
				let data = SourceData {
					name,
					text,
					path: Some(full_path.clone()),
					tabs: TAB_SIZE.into(),
				};
				let data = self.ctx.store(data);
				Ok(Source { data })
			});
			entry.clone()
		}
	}
}

pub struct SourceMap<'a> {
	base_dir: RwLock<PathBuf>,
	sources: RwLock<HashMap<PathBuf, Result<Source<'a>>>>,
}

impl<'a> SourceMap<'a> {
	pub fn new<T: AsRef<Path>>(base_dir: T) -> Result<Self> {
		let base_dir = norm_path(base_dir, "base path")?.into();
		Ok(Self {
			base_dir,
			sources: Default::default(),
		})
	}
}

#[derive(Copy, Clone)]
pub struct Source<'a> {
	data: &'a SourceData,
}

struct SourceData {
	name: String,
	text: String,
	tabs: AtomicUsize,
	path: Option<PathBuf>,
}

impl<'a> Source<'a> {
	pub fn empty() -> Self {
		static EMPTY: OnceLock<Arc<SourceData>> = OnceLock::new();
		let data = EMPTY.get_or_init(|| {
			SourceData {
				name: String::new(),
				text: String::new(),
				path: None,
				tabs: TAB_SIZE.into(),
			}
			.into()
		});
		Source { data }
	}

	pub fn name(&self) -> &'a str {
		self.data.name.as_str()
	}

	#[inline]
	pub fn text(&self) -> &'a str {
		self.data.text.as_str()
	}

	pub fn len(&self) -> usize {
		self.data.text.len()
	}

	pub fn path(&self) -> Option<&'a Path> {
		self.data.path.as_ref().map(|x| x.as_path())
	}

	pub fn span(&self) -> Span<'a> {
		Span::new(0, self.len(), *self)
	}

	pub fn tab_size(&self) -> usize {
		self.data.tabs.load(SyncOrder::Relaxed)
	}

	pub fn range<T: RangeBounds<usize>>(&self, range: T) -> Span<'a> {
		let sta = match range.start_bound() {
			std::ops::Bound::Included(&n) => n,
			std::ops::Bound::Excluded(&n) => n + 1,
			std::ops::Bound::Unbounded => 0,
		};
		let end = match range.end_bound() {
			std::ops::Bound::Included(&n) => n + 1,
			std::ops::Bound::Excluded(&n) => n,
			std::ops::Bound::Unbounded => self.len(),
		};
		Span::new(sta, end, *self)
	}

	fn as_ptr(&self) -> *const SourceData {
		self.data
	}
}

impl<'a> Default for Source<'a> {
	fn default() -> Self {
		Source::empty()
	}
}

impl<'a> Eq for Source<'a> {}

impl<'a> PartialEq for Source<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl<'a> Hash for Source<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_ptr().hash(state);
	}
}

impl<'a> Display for Source<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "<empty>" } else { name };
		write!(f, "{name}")
	}
}

impl<'a> Debug for Source<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "()" } else { name };
		write!(f, "Source(`{name}` with {} bytes)", self.len())
	}
}

impl<'a> Ord for Source<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		let a = self.data;
		let b = other.data;

		// sort files first...
		let a_str = a.path.is_none();
		let b_str = b.path.is_none();
		(a_str.cmp(&b_str))
			.then_with(|| a.path.cmp(&b.path))
			// ...then string sources by name, length, and text
			.then_with(|| a.name.cmp(&b.name))
			.then_with(|| a.text.len().cmp(&b.text.len()))
			.then_with(|| a.text.cmp(&b.text))
			// ...finally fallback to the pointer so there's always a global order
			.then_with(|| (a as *const SourceData).cmp(&(b as *const SourceData)))
	}
}

impl<'a> PartialOrd for Source<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

fn get_full_path<T: AsRef<Path>, U: AsRef<Path>>(base: T, path: U) -> Result<PathBuf> {
	let base = norm_path(base, "base path")?;
	let full = norm_path(base.join(path.as_ref()), "path")?;
	Ok(full)
}

fn norm_path<T: AsRef<Path>>(path: T, desc: &'static str) -> Result<PathBuf> {
	let path = path.as_ref();
	let path = path
		.canonicalize()
		.map_err(|err| err!(="{desc} is not valid: {} -- {err}", path.to_string_lossy()))?;
	Ok(path)
}
