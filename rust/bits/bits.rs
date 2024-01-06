use std::{
	borrow::Cow,
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

pub mod code;
pub use code::*;

pub mod context;
pub mod core;
pub mod eval;
pub mod input;
pub mod ops;
pub mod result;

pub use context::*;
pub use core::*;
pub use eval::*;
pub use input::*;
pub use ops::*;
pub use result::*;

pub mod mem;
pub use mem::*;

const DEBUG_CODE: bool = false;
const DEBUG_EVAL: bool = false;
const DEBUG_EVAL_EMPTY: bool = false;

pub fn version() -> &'static str {
	"0.1.0"
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Precedence {
	First,
	LineSplit,
	Indent,
	Let,
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

pub fn init_context<'a>(ctx: ContextRef<'a>) -> Result<()> {
	let mut lexer = GrammarLexer::new(DefaultGrammar);
	lexer.add_symbols(["(", ")", "[", "]", "{", "}", "<", ">"]);
	lexer.add_symbols([",", ";", ".", ":", ".."]);
	lexer.add_symbols(["+", "-", "*", "/", "="]);
	ctx.set_lexer(lexer);

	let ops = ctx.ops();
	let types = ctx.types();

	let t_sint = types.sint();

	let mul = ops.get(OpKey(OpKind::Core, Symbol::str("*")));
	mul.define_binary(t_sint, (t_sint, t_sint)).set_eval(|_rt, lhs, rhs| {
		let lhs = if let Value::SInt(v) = lhs { v } else { unreachable!() };
		let rhs = if let Value::SInt(v) = rhs { v } else { unreachable!() };
		Ok(Value::SInt(lhs * rhs))
	});

	let add = ops.get(OpKey(OpKind::Core, Symbol::str("+")));
	add.define_binary(t_sint, (t_sint, t_sint)).set_eval(|_rt, lhs, rhs| {
		let lhs = if let Value::SInt(v) = lhs { v } else { unreachable!() };
		let rhs = if let Value::SInt(v) = rhs { v } else { unreachable!() };
		Ok(Value::SInt(lhs + rhs))
	});

	let bindings = ctx.bindings();

	bindings
		.match_any(Match::source())
		.with_precedence(Precedence::First)
		.bind(TokenizeSource);

	bindings
		.match_any(Match::exact(Value::Token(Token::Break)))
		.with_precedence(Precedence::LineSplit)
		.bind(EvalLineBreak);

	bindings
		.match_any(Match::indent())
		.with_precedence(Precedence::Indent)
		.bind(EvalIndent);

	bindings
		.match_any(Match::symbol(":"))
		.with_precedence(Precedence::Indent)
		.bind(EvalIndentedBlock);

	bindings
		.match_any(Match::word("if"))
		.with_precedence(Precedence::BlockParse)
		.bind(EvalBlock("if statement", eval_if));

	bindings
		.match_any(Match::word("else"))
		.with_precedence(Precedence::BlockParse)
		.bind(EvalBlock("else", eval_else));

	bindings
		.match_any(Match::word("for"))
		.with_precedence(Precedence::BlockParse)
		.bind(EvalBlock("for statement", eval_for));

	bindings
		.match_any(Match::word("print"))
		.with_precedence(Precedence::Print)
		.bind(EvalPrint);

	bindings
		.match_any(Match::word("let"))
		.with_precedence(Precedence::Let)
		.bind(EvalLet);

	// Operators

	bindings
		.match_any(Match::word("in"))
		.with_precedence(Precedence::OpIn)
		.bind(EvalBinaryOp {
			op: OpKey(OpKind::Core, Symbol::str("in")),
			group_right: false,
		});

	bindings
		.match_any(Match::symbol(".."))
		.with_precedence(Precedence::OpRange)
		.bind(EvalBinaryOp {
			op: OpKey(OpKind::Core, Symbol::str("..")),
			group_right: false,
		});

	bindings
		.match_any(Match::symbol("+"))
		.with_precedence(Precedence::OpAdd)
		.bind(EvalBinaryOp {
			op: OpKey(OpKind::Core, Symbol::str("+")),
			group_right: false,
		});

	bindings
		.match_any(Match::symbol("*"))
		.with_precedence(Precedence::OpMul)
		.bind(EvalBinaryOp {
			op: OpKey(OpKind::Core, Symbol::str("*")),
			group_right: false,
		});

	// Block eval

	bindings
		.match_any(Match::kind_of(Value::For))
		.with_precedence(Precedence::BlockEval)
		.bind(EvalFor);

	bindings
		.match_any(Match::kind_of(Value::If))
		.with_precedence(Precedence::BlockEval)
		.bind(EvalIf);

	// Output

	bindings
		.match_any(Match::token_kind(Token::Literal))
		.with_precedence(Precedence::Output)
		.bind(Output);

	bindings
		.match_any(Match::token_kind(Token::Integer))
		.with_precedence(Precedence::Output)
		.bind(Output);

	bindings
		.match_any(Match::token(Token::Word(Symbol::str("true"))))
		.with_precedence(Precedence::Output)
		.bind(EvalBool(true));

	bindings
		.match_any(Match::token(Token::Word(Symbol::str("false"))))
		.with_precedence(Precedence::Output)
		.bind(EvalBool(false));

	bindings
		.match_any(Match::kind_of(Value::Bool(true)))
		.with_precedence(Precedence::Output)
		.bind(Output);

	bindings
		.match_any(Match::kind_of(Value::Group { scoped: false }))
		.with_precedence(Precedence::Output)
		.bind(Output);

	bindings
		.match_any(Match::kind_of(Value::Sequence {
			scoped: false,
			indented: false,
		}))
		.with_precedence(Precedence::Output)
		.bind(Output);

	bindings
		.match_any(Match::kind_of(Value::BinaryOp(OpKey(OpKind::Core, Symbol::empty()))))
		.with_precedence(Precedence::Output)
		.bind(Output);

	Ok(())
}

pub fn execute<'a, 'b>(ctx: ContextRef<'a>, out: Writer<'b>) -> Result<Value<'a>> {
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

	let nodes = bindings.root_nodes(false);
	let mut program = Vec::new();
	for it in nodes {
		let code = it.compile()?;
		program.push(code);
	}

	if DEBUG_CODE {
		println!("\n========== PROGRAM ==========");
		println!("\n{program:#?}");
		println!("\n=============================\n");
	}

	// SAFETY: rust dumb-assery makes it impossible to split the lifetime of
	// the writer from the context without infecting the whole thing with an
	// additional lifetime parameter for the runtime writer, but we only
	// care the that the writer is valid during this function so f- that.
	let mut rt = Runtime::new(ctx, unsafe { std::mem::transmute(out) });
	let mut output = Value::None;
	for it in program {
		output = it.execute(&mut rt)?;
	}

	Ok(output)
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
