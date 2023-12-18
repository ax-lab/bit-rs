use std::{
	cmp::Ordering,
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	io::Write,
	sync::{Arc, OnceLock, RwLock},
};

pub mod core;
pub mod input;
pub mod ops;
pub mod result;

pub use core::*;
pub use input::*;
pub use ops::*;
pub use result::*;

pub fn version() -> &'static str {
	"0.1.0"
}
