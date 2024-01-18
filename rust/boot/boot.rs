//! Compiler bootstrapper.
//!
//! Provides a minimally viable language environment with a limited set of
//! functionality to build the base language compiler.
//!
//! These are the goals for the bootstrapper language:
//!
//! - Core language parsing
//! - Usable type system
//! - C/C++ code output and compilation
//! - Simplified scripting runtime
//!
//! The core language is designed to provide access to essential features of
//! the C language, but not much else.

use std::{
	any::TypeId,
	cell::UnsafeCell,
	cmp::Ordering,
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	io::Write,
	ops::RangeBounds,
	path::{Path, PathBuf},
	ptr::NonNull,
	sync::{
		atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering as Order},
		Arc, Mutex, Once, RwLock,
	},
};

mod arena;
mod chars;
mod cursor;
mod format;
mod iter;
mod list;
mod result;
mod source;
mod span;
mod table;
mod value;

pub use arena::*;
pub use chars::*;
pub use cursor::*;
pub use format::*;
pub use iter::*;
pub use list::*;
pub use result::*;
pub use source::*;
pub use span::*;
pub use table::*;
pub use value::*;
