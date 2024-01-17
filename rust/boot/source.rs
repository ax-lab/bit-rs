use super::*;

pub const DEFAULT_TAB_SIZE: usize = 4;

pub struct SourceMap {
	base_dir: RwLock<PathBuf>,
	sources: RwLock<HashMap<PathBuf, Result<Source>>>,
}

impl SourceMap {
	pub fn new<T: AsRef<Path>>(base_dir: T) -> Result<Self> {
		let output = Self {
			base_dir: Default::default(),
			sources: Default::default(),
		};
		output.set_base_dir(base_dir)?;
		Ok(output)
	}

	pub fn set_base_dir<T: AsRef<Path>>(&self, base_dir: T) -> Result<PathBuf> {
		let base_dir = norm_path(base_dir, "base path")?;
		let previous = std::mem::replace(&mut *self.base_dir.write().unwrap(), base_dir);
		Ok(previous)
	}

	pub fn from_string<T: AsRef<str>, U: AsRef<str>>(&self, name: T, text: U) -> Source {
		let name = Box::leak(name.as_ref().into());
		let text = Box::leak(text.as_ref().into());
		let data = SourceData {
			name,
			text,
			path: None,
			tabs: 0.into(),
		};
		let data = Arena::get().store(data);
		Source { data }
	}

	pub fn load_file<T: AsRef<Path>>(&self, path: T) -> Result<Source> {
		let path = path.as_ref();
		let base_dir = self.base_dir.read().unwrap().clone();
		let full_path = get_full_path(&base_dir, path)?;

		let sources = self.sources.read().unwrap();
		if let Some(src) = sources.get(&full_path) {
			src.clone()
		} else {
			drop(sources);

			let mut sources = self.sources.write().unwrap();
			let entry = sources.entry(full_path).or_insert_with_key(|full_path| {
				let name = full_path.strip_prefix(&base_dir).unwrap_or(full_path).to_string_lossy();
				let name = Box::leak(name.into());
				let text = std::fs::read_to_string(&full_path).map_err(|err| err!("loading `{name}`: {err}"))?;
				let text = Box::leak(text.as_str().into());
				let path = Box::leak(full_path.as_path().into());
				let data = SourceData {
					name,
					text,
					path: Some(path),
					tabs: 0.into(),
				};
				let data = Arena::get().store(data);
				Ok(Source { data })
			});
			entry.clone()
		}
	}
}

#[derive(Copy, Clone)]
pub struct Source {
	data: &'static SourceData,
}

struct SourceData {
	name: &'static str,
	text: &'static str,
	tabs: AtomicUsize,
	path: Option<&'static Path>,
}

impl Source {
	pub fn empty() -> Self {
		static EMPTY: Init<SourceData> = Init::new(|| SourceData {
			name: "",
			text: "",
			tabs: 0.into(),
			path: None,
		});
		let data = EMPTY.get();
		Source { data }
	}

	pub fn name(&self) -> &'static str {
		self.data.name
	}

	pub fn text(&self) -> &'static str {
		self.data.text
	}

	pub fn len(&self) -> usize {
		self.data.text.len()
	}

	pub fn path(&self) -> Option<&'static Path> {
		self.data.path
	}

	pub fn tabs(&self) -> usize {
		let tabs = self.data.tabs.load(Order::Relaxed);
		if tabs == 0 {
			DEFAULT_TAB_SIZE
		} else {
			tabs
		}
	}

	fn as_ptr(&self) -> *const SourceData {
		self.data
	}
}

impl Default for Source {
	fn default() -> Self {
		Source::empty()
	}
}

impl Eq for Source {}

impl PartialEq for Source {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl Hash for Source {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_ptr().hash(state);
	}
}

impl Display for Source {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "<empty>" } else { name };
		write!(f, "{name}")
	}
}

impl Debug for Source {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "()" } else { name };
		write!(f, "Source(`{name}` with ")?;
		write_bytes(&mut f.write(), self.len())?;
		write!(f, ")")
	}
}

impl Ord for Source {
	fn cmp(&self, other: &Self) -> Ordering {
		let a = self.data;
		let b = other.data;

		// sort source files first (i.e., no_path == false)
		let a_no_path = a.path.is_none();
		let b_no_path = b.path.is_none();
		(a_no_path.cmp(&b_no_path))
			.then_with(|| a.path.cmp(&b.path))
			// ...then string sources by name, length, and text
			.then_with(|| a.name.cmp(&b.name))
			.then_with(|| a.text.len().cmp(&b.text.len()))
			.then_with(|| a.text.cmp(&b.text))
			// ... fallback to the pointer for creation order (assuming an arena)
			.then_with(|| (a as *const SourceData).cmp(&(b as *const SourceData)))
	}
}

impl PartialOrd for Source {
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
		.map_err(|err| err!("{desc} is not valid: {} -- {err}", path.to_string_lossy()))?;
	Ok(path)
}
