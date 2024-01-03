use std::{
	cell::{Cell, Ref, RefCell, UnsafeCell},
	cmp::Ordering,
	collections::{HashMap, HashSet},
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

pub fn init_context<'a>(ctx: ContextRef<'a>) -> Result<()> {
	let mut lexer = GrammarLexer::new(DefaultGrammar);
	lexer.add_symbols(["(", ")", "[", "]", "{", "}", "<", ">"]);
	lexer.add_symbols([",", ";", ".", ":"]);
	lexer.add_symbols(["+", "-", "*", "/", "="]);

	ctx.set_lexer(lexer);
	ctx.bindings()
		.match_any(Match::source())
		.with_precedence(Value::SInt(0))
		.bind(TokenizeSource);
	ctx.bindings()
		.match_any(Match::exact(Value::Token(Token::Break)))
		.with_precedence(Value::SInt(1))
		.bind(SplitLine);

	ctx.bindings()
		.match_any(Match::word("print"))
		.with_precedence(Value::SInt(2))
		.bind(Print);

	Ok(())
}

pub fn process<'a>(ctx: ContextRef<'a>) -> Result<Value<'a>> {
	let bindings = ctx.bindings();
	while let Some(next) = bindings.get_next() {
		let eval = next.eval();
		eval.execute(ctx, next)?;
	}

	let mut nodes = bindings.get_pending();
	if nodes.len() > 0 {
		const MAX_BY_SRC: usize = 20;
		const MAX_TOTAL: usize = 50;

		nodes.sort_by_key(|node| (node.span(), node.value()));

		let count = nodes.len();
		let (s, were) = if count > 1 { ("s", "were") } else { ("", "was") };
		let mut mapped = HashSet::new();
		let mut output = Vec::new();
		let mut by_source = HashMap::new();
		for node in nodes {
			let src = node.span().src();
			let count = by_source.entry(src).or_insert(0);
			if *count >= MAX_BY_SRC {
				continue;
			}

			let key = (src, node.value());
			if mapped.insert(key) {
				output.push(node);
				*count += 1;
			}
		}

		let output_len = output.len();
		let nodes = output
			.into_iter()
			.take(MAX_TOTAL)
			.map(|node| {
				let location = node.span().location();
				format!("- at {location}: {node}")
			})
			.collect::<Vec<_>>()
			.join("\n");

		let suffix = if count > output_len {
			let cnt = count - output_len;
			let s = if cnt > 1 { "s" } else { "" };
			format!("\n\n  …skipping remaining {cnt} node{s}")
		} else {
			format!("")
		};

		err!("{count} node{s} {were} not processed:\n\n{nodes}{suffix}")?;
	}
	Ok(Value::None)
}

pub fn dump_nodes(f: &mut Writer, ctx: ContextRef) -> Result<()> {
	let mut cur_src = None;
	for node in ctx.root_nodes(false) {
		let src = node.span().src();
		if Some(src) != cur_src {
			cur_src = Some(src);
			write!(f, "\n>>> {src:?} <<<\n")?;
		}

		let mut f = f.indented();
		write!(f, "\n-> {}:\n\n", node.span())?;

		node.write(&mut f.indented())?;
		write!(f, "\n")?;
	}
	Ok(())
}
