use super::*;

pub mod value;
pub use value::*;

pub mod kind;
pub use kind::*;

pub mod array;
pub mod bool;
pub mod float;
pub mod int;
pub mod str;

pub use array::*;
pub use bool::*;
pub use float::*;
pub use int::*;
pub use str::*;

const _: () = {
	use std::panic::UnwindSafe;

	fn thread_safe<T: Send + Sync + UnwindSafe>() {}

	fn assert() {
		thread_safe::<Data>();
		thread_safe::<Kind>();
	}
};
