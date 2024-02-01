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
	borrow::Cow,
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
mod cmd;
mod code;
mod core;
mod cursor;
mod eval;
mod format;
mod heap;
mod iter;
mod lexer;
mod list;
mod node;
mod queue;
mod result;
mod source;
mod span;
mod symbol;
mod table;
mod temp;
mod term;
mod token;
mod unicode;
mod value;

pub use arena::*;
pub use binding::*;
pub use chars::*;
pub use cmd::*;
pub use code::*;
pub use core::*;
pub use cursor::*;
pub use eval::*;
pub use format::*;
pub use iter::*;
pub use lexer::*;
pub use list::*;
pub use node::*;
pub use queue::*;
pub use result::*;
pub use source::*;
pub use span::*;
pub use symbol::*;
pub use table::*;
pub use temp::*;
pub use term::*;
pub use token::*;
pub use unicode::*;
pub use value::*;

use heap::*;

pub enum Message<'a, 'b> {
	None,
	Output(Node, &'a mut Writer<'b>),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Precedence {
	First,
	Source,
	LineSplit,
	Indent,
	ExpandRaw,
	Comment,
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
	Literal,
	Last,
}

#[derive(Default)]
pub struct Options {
	pub show_program: bool,
	pub dump_code: bool,
	pub compile: bool,
}

pub fn init_core() {
	let lexer = Lexer::new();
	let symbols = Symbols::get();

	let sources = SOURCES.get();
	sources.add_global_init(DefaultLexer(lexer));

	let raw = RAW.get();
	raw.add_eval(SplitLines);
	raw.add_eval(ExpandRaw);

	let comment = COMMENT.get();
	comment.add_eval(RemoveNode(Precedence::Comment));

	let print = WORDS.get(symbols.PRINT);
	print.add_eval(ParsePrint);

	let literal = LITERAL.get();
	literal.add_eval(ParseLiteral);

	let integer = INTEGER.get();
	integer.add_eval(ParseLiteral);

	let float = FLOAT.get();
	float.add_eval(ParseLiteral);

	WORDS.get(symbols.TRUE).add_eval(ParseLiteral);
	WORDS.get(symbols.FALSE).add_eval(ParseLiteral);
}

pub fn execute(input: &[Source], options: Options) -> Result<()> {
	let program = Node::new_at(Program, Span::empty());
	for it in input.iter().copied() {
		let span = it.span();
		let node = Node::new_at(it, span);
		program.push_node(node);
	}

	let err = Queue::process();
	program.set_done(true);

	let err = err.and_then(|_| Node::check_pending());

	if options.show_program {
		let mut out = Writer::stdout();
		write!(out, "\n========= PROGRAM =========\n\n")?;
		program.write(&mut out)?;
		write!(out, "\n\n===========================\n")?;
	}

	let output = err.and_then(|_| {
		let ctx = CodeContext::new();
		program.value().output_code(ctx, program)
	})?;

	if options.dump_code {
		println!("\n{output:#?}\n");
	}

	if options.compile {
		let mut builder = clang::Builder::new();
		let code = output.generate_c(&mut builder)?;

		let mut runner = builder.build(code);
		if options.dump_code {
			println!("\n{}\n", runner.code);
		}

		let status = runner.run()?;
		if !status.success() {
			raise!("finished with status {status}");
		}
	} else {
		let mut rt = Runtime::default();
		let value = output.execute(&mut rt)?;

		if !value.is::<()>() {
			println!("\nanswer = {value}");
		}
	}

	Ok(())
}

pub fn error<T: std::fmt::Display>(msg: T) {
	let _ = term::error(std::io::stderr(), msg);
}
