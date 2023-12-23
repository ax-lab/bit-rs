use std::{
	cell::UnsafeCell,
	cmp::Ordering,
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
	hash::Hash,
	io::Write,
	marker::PhantomData,
	mem::MaybeUninit,
	ops::Deref,
	panic::UnwindSafe,
	sync::{
		atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU8, AtomicUsize, Ordering as SyncOrder},
		Arc, Mutex, OnceLock, RwLock,
	},
};

pub mod types;
pub use types::*;

pub mod context;
pub use context::*;

pub mod core;
pub mod input;
pub mod ops;
pub mod result;

pub use core::*;
pub use input::*;
pub use ops::*;
pub use result::*;

pub mod mem;
pub use mem::*;

pub fn version() -> &'static str {
	"0.1.0"
}

#[derive(Default)]
pub struct ContextData<'a> {
	pub types: ContextCell<'a, TypeContext<'a>>,
}

impl<'a> ContextData<'a> {
	fn new(&'a self, ctx: ContextRef<'a>) {
		self.types.new(ctx);
	}

	fn init(&self) {
		self.types.init();
	}
}

pub struct TypeContext<'a> {
	#[allow(unused)]
	ctx: ContextRef<'a>,
	init: bool,
}

impl<'a> IsContext<'a> for TypeContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		Self { ctx, init: false }
	}

	fn init(&mut self) {
		self.init = true;
	}
}

const _: () = {
	fn thread_safe<T: Send + Sync + UnwindSafe>() {}

	fn assert() {
		thread_safe::<Type>();
		thread_safe::<Value>();
		thread_safe::<ContextData>();
	}
};

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn context() {
		let ctx = Context::new();
		assert!(ctx.get().types.init);
		assert_eq!(ctx.get().types.ctx.data() as *const _, ctx.get().data() as *const _);
	}
}
