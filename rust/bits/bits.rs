use std::{
	cmp::Ordering,
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	sync::{Arc, OnceLock, RwLock},
};

pub mod core;

pub use core::*;

pub fn version() -> &'static str {
	"0.1.0"
}
