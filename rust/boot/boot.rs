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
	collections::{HashMap, VecDeque},
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
mod binding;
mod chars;
mod cursor;
mod eval;
mod format;
mod heap;
mod iter;
mod list;
mod node;
mod queue;
mod result;
mod source;
mod span;
mod table;
mod value;

pub use arena::*;
pub use binding::*;
pub use chars::*;
pub use cursor::*;
pub use eval::*;
pub use format::*;
pub use iter::*;
pub use list::*;
pub use node::*;
pub use queue::*;
pub use result::*;
pub use source::*;
pub use span::*;
pub use table::*;
pub use value::*;

use heap::*;

pub enum Message {
	SayHi(&'static str),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Precedence {
	First,
	LineSplit,
	Indent,
	LetDecl,
	LetExpr,
	BlockParse,
	VarBinding,
	OpIn,
	OpRange,
	OpAdd,
	OpMul,
	Print,
	BlockEval,
	Output,
	Last,
}
