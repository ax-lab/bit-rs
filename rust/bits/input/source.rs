use std::path::{Path, PathBuf};

use super::*;

pub struct SourceMap {
	base_dir: PathBuf,
	sources: RwLock<HashMap<PathBuf, Result<Source>>>,
}

impl SourceMap {
	pub fn new<T: AsRef<Path>>(base_dir: T) -> Result<Self> {
		let base_dir = norm_path(base_dir, "base path")?;
		Ok(Self {
			base_dir,
			sources: Default::default(),
		})
	}

	pub fn from_string<T: Into<String>, U: Into<String>>(name: T, text: U) -> Source {
		Source::new(name, text)
	}

	pub fn load_file<T: AsRef<Path>>(&self, path: T) -> Result<Source> {
		let path = path.as_ref();
		let full_path = get_full_path(&self.base_dir, path)?;

		let sources = self.sources.read().unwrap();
		if let Some(src) = sources.get(&full_path) {
			src.clone()
		} else {
			drop(sources);

			let mut sources = self.sources.write().unwrap();
			let entry = sources.entry(full_path).or_insert_with_key(|full_path| {
				let name = full_path
					.strip_prefix(&self.base_dir)
					.unwrap_or(full_path)
					.to_string_lossy()
					.into();
				let text = std::fs::read_to_string(&full_path).map_err(|err| err!(="loading `{name}`: {err}"))?;
				let data = SourceData {
					name,
					text,
					path: Some(full_path.clone()),
				}
				.into();
				Ok(Source { data })
			});
			entry.clone()
		}
	}
}

#[derive(Clone)]
pub struct Source {
	data: Arc<SourceData>,
}

struct SourceData {
	name: String,
	text: String,
	path: Option<PathBuf>,
}

impl Source {
	pub fn new<T: Into<String>, U: Into<String>>(name: T, text: U) -> Self {
		let name = name.into();
		let text = text.into();
		let data = SourceData { name, text, path: None }.into();
		Source { data }
	}

	pub fn empty() -> Self {
		static EMPTY: OnceLock<Arc<SourceData>> = OnceLock::new();
		let data = EMPTY.get_or_init(|| {
			SourceData {
				name: String::new(),
				text: String::new(),
				path: None,
			}
			.into()
		});
		Source { data: data.clone() }
	}

	pub fn name(&self) -> &str {
		self.data.name.as_str()
	}

	pub fn text(&self) -> &str {
		self.data.text.as_str()
	}

	pub fn len(&self) -> usize {
		self.data.text.len()
	}

	pub fn path(&self) -> Option<&Path> {
		self.data.path.as_ref().map(|x| x.as_path())
	}

	pub fn span(&self) -> Span {
		Span::new(0, self.len(), self.clone())
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
		Arc::as_ptr(&self.data) == Arc::as_ptr(&other.data)
	}
}

impl Hash for Source {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		Arc::as_ptr(&self.data).hash(state);
	}
}

impl Display for Source {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "<empty>" } else { name };
		write!(f, "{name}")
	}
}

impl Debug for Source {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let name = self.name();
		let name = if name == "" { "()" } else { name };
		write!(f, "Source(`{name}` with {} bytes)", self.len())
	}
}

impl Ord for Source {
	fn cmp(&self, other: &Self) -> Ordering {
		let a = self.data.as_ref();
		let b = other.data.as_ref();

		// sort files first...
		let a_str = a.path.is_none();
		let b_str = b.path.is_none();
		(a_str.cmp(&b_str))
			.then_with(|| a.path.cmp(&b.path))
			// ...then string sources by name
			.then_with(|| a.name.cmp(&b.name))
			// ...finally fallback to the pointer so there's always a global order
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
		.map_err(|err| err!(="{desc} is not valid: {} -- {err}", path.to_string_lossy()))?;
	Ok(path)
}
