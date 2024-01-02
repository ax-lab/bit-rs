use std::{
	cell::{Cell, Ref, RefCell, UnsafeCell},
	cmp::Ordering,
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	io::Write,
	marker::PhantomData,
	mem::MaybeUninit,
	ops::{Deref, RangeBounds},
	panic::UnwindSafe,
	sync::{
		atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU8, AtomicUsize, Ordering as SyncOrder},
		Arc, Mutex, OnceLock, RwLock,
	},
};

pub mod types;
pub use types::*;

pub mod values;
pub use values::*;

pub mod nodes;
pub use nodes::*;

pub mod context;
pub use context::*;

pub mod core;
pub mod eval;
pub mod input;
pub mod ops;
pub mod result;

pub use core::*;
pub use eval::*;
pub use input::*;
pub use ops::*;
pub use result::*;

pub mod mem;
pub use mem::*;

pub fn version() -> &'static str {
	"0.1.0"
}

pub fn process<'a>(ctx: ContextRef<'a>) -> Result<Value<'a>> {
	let bindings = ctx.bindings();
	while let Some(next) = bindings.get_next() {
		let eval = next.eval();
		eval.parse(ctx, next)?;
	}

	let nodes = bindings.get_pending();
	if nodes.len() > 0 {
		let nodes = nodes
			.into_iter()
			.take(100)
			.map(|node| {
				let location = node.span().location();
				format!("- at {location}: {node}")
			})
			.collect::<Vec<_>>()
			.join("\n");

		err!("the following nodes were not processed:\n\n{nodes}")?;
	}
	Ok(Value::None)
}
