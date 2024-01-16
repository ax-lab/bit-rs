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

use std::fmt::{Debug, Display, Formatter};

mod result;

pub use result::*;
