use bits::*;

use std::io::Write;

#[derive(Default)]
struct Args {
	show_stats: bool,
	show_dump: bool,
	show_result: bool,
}

fn main() {
	let args = Args::default();
	if let Err(err) = run(args) {
		eprintln!("\nError: {}\n", err.detailed());
		std::process::exit(1);
	}
}

fn run(mut args: Args) -> Result<()> {
	let ctx = Context::new();
	let ctx = ctx.get();
	let out = Writer::stdout();
	init_context(ctx)?;

	let sources = ctx.sources();
	for arg in std::env::args().skip(1) {
		if arg == "--mem" || arg == "--stats" {
			args.show_stats = true;
			continue;
		}

		if arg == "--dump" {
			args.show_dump = true;
			args.show_stats = true;
			args.show_result = true;
			continue;
		}

		let src = sources.load_file(arg)?;
		ctx.node(NodeValue::Source(src), src.span());
	}

	let value = execute(ctx, out);

	if args.show_stats || value.is_err() {
		dump_stats(value.is_err());
	}

	if value.is_err() || args.show_dump {
		let mut out = Writer::stderr();
		let _ = write!(out, "\n========== PROGRAM DUMP ==========\n");
		let _ = dump_nodes(&mut Writer::stderr(), ctx);
		let _ = write!(out, "\n==================================\n");
	}

	let value = value?;
	if args.show_result {
		println!("\nResult = {value:?}\n");
	}

	Ok(())
}

pub fn dump_stats(error: bool) {
	let stats = Arena::stats();
	let used = stats.used();
	let size = stats.size();
	let max_used = stats.max_used();
	let max_size = stats.max_size();

	let mut out = if error { Writer::stdout() } else { Writer::stderr() };

	let _ = print_bytes(&mut out, "\n[INFO] Memory used: ", used);
	let _ = print_bytes(&mut out, " out of ", size);
	let _ = print_bytes(&mut out, " (max: ", max_used);
	let _ = print_bytes(&mut out, " / ", max_size);
	let _ = write!(out, ")\n");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty() -> Result<()> {
		check(NodeValue::None, "", "")
	}

	#[test]
	fn simple_string() -> Result<()> {
		check(NodeValue::Str("abc"), "", "'abc'")
	}

	#[test]
	fn simple_int() -> Result<()> {
		check(NodeValue::SInt(42), "", "42")
	}

	#[test]
	fn hello_world() -> Result<()> {
		check(NodeValue::Unit, "hello world\n", "print 'hello world'")
	}

	#[test]
	fn simple_variable() -> Result<()> {
		check(NodeValue::SInt(42), "", src(["let x = 42", "x"]))
	}

	#[test]
	#[cfg(off)]
	fn recursive_variable() -> Result<()> {
		check(NodeValue::SInt(42), "", src(["let x = this"]))
	}

	#[test]
	fn variable_shadowing() -> Result<()> {
		check(
			NodeValue::SInt(69),
			"42\n",
			src(["let x = 42", "print x", "let x = 69", "x"]),
		)
	}

	#[test]
	fn simple_expression() -> Result<()> {
		check(
			NodeValue::SInt(42),
			"",
			src(["let x = 5", "let y = 2", "let z = y * y", "x * y * z + y"]),
		)
	}

	#[test]
	fn if_expression() -> Result<()> {
		check(
			NodeValue::Unit,
			"this is true\n",
			src(["if true:", "\tprint 'this is true'", "else:", "\tprint 'this is false'"]),
		)?;

		check(
			NodeValue::Unit,
			"this is false\n",
			src([
				"if false:",
				"\tprint 'this is true'",
				"else:",
				"\tprint 'this is false'",
			]),
		)?;

		check(NodeValue::SInt(42), "", src(["if true:", "\t42", "else:", "\t69"]))?;
		check(NodeValue::SInt(69), "", src(["if false:", "\t42", "else:", "\t69"]))?;

		Ok(())
	}

	#[test]
	fn else_if_expression() -> Result<()> {
		check(
			NodeValue::Unit,
			"start\nA\ndone\n",
			src([
				"print 'start'",
				"if true:",
				"\tprint 'A'",
				"else if true:",
				"\tprint 'B'",
				"else if true:",
				"\tprint 'C'",
				"else:",
				"\tprint 'D'",
				"print 'done'",
			]),
		)?;

		check(
			NodeValue::Unit,
			"start\nB\ndone\n",
			src([
				"print 'start'",
				"if false:",
				"\tprint 'A'",
				"else if true:",
				"\tprint 'B'",
				"else if true:",
				"\tprint 'C'",
				"else:",
				"\tprint 'D'",
				"print 'done'",
			]),
		)?;

		check(
			NodeValue::Unit,
			"start\nC\ndone\n",
			src([
				"print 'start'",
				"if false:",
				"\tprint 'A'",
				"else if false:",
				"\tprint 'B'",
				"else if true:",
				"\tprint 'C'",
				"else:",
				"\tprint 'D'",
				"print 'done'",
			]),
		)?;

		check(
			NodeValue::Unit,
			"start\nD\ndone\n",
			src([
				"print 'start'",
				"if false:",
				"\tprint 'A'",
				"else if false:",
				"\tprint 'B'",
				"else if false:",
				"\tprint 'C'",
				"else:",
				"\tprint 'D'",
				"print 'done'",
			]),
		)?;

		check(
			NodeValue::Unit,
			"start\nD\ndone\n",
			src([
				"print 'start'",
				"if false:",
				"\tprint 'A'",
				"else if false:",
				"\tprint 'B'",
				"else if false:",
				"\tprint 'C'",
				"else if true:",
				"\tprint 'D'",
				"print 'done'",
			]),
		)?;

		check(
			NodeValue::Unit,
			"start\ndone\n",
			src([
				"print 'start'",
				"if false:",
				"\tprint 'A'",
				"else if false:",
				"\tprint 'B'",
				"else if false:",
				"\tprint 'C'",
				"else if false:",
				"\tprint 'D'",
				"print 'done'",
			]),
		)?;

		Ok(())
	}

	#[test]
	#[ignore]
	fn simple_foreach() -> Result<()> {
		check(
			NodeValue::Unit,
			"1\n2\n3\n4\n5\ndone\n",
			src(["for i in 1..6:", "\tprint i", "print 'done'"]),
		)
	}

	fn check<T: Into<String>>(expected_value: NodeValue, expected_output: &str, code: T) -> Result<()> {
		let mut out = String::new();

		// ignore the incoming value lifetime
		let expected_value: NodeValue = unsafe { std::mem::transmute(expected_value) };

		let ctx = Context::new();
		let ctx = ctx.get();
		init_context(ctx)?;

		let src = ctx.sources().from_string("eval", code);
		ctx.node(NodeValue::Source(src), src.span());

		let ans = {
			let w = Writer::fmt(&mut out);
			let ans = execute(ctx, w);

			let ans = match ans {
				Ok(val) => val,
				Err(err) => {
					let mut out = dump_context(ctx);
					let _ = writeln!(out, "\n===[ EVAL ERROR ]===\n\n{err}\n\n====================\n");
					return Err("Eval failed".to_error());
				}
			};
			ans
		};

		if ans != expected_value || expected_output != out.as_str() {
			let mut out = Writer::stderr();
			dump_stats(true);
			let _ = writeln!(out, "\n===[ PROGRAM ]======");
			let _ = dump_nodes(&mut out, ctx);
		}

		assert_eq!(expected_value, ans);
		assert_eq!(expected_output, out);
		Ok(())
	}

	fn src<T: IntoIterator<Item = U>, U: Into<String>>(src: T) -> String {
		let lines = src.into_iter().map(|x| x.into()).collect::<Vec<_>>();
		lines.join("\n")
	}

	fn dump_context(ctx: ContextRef) -> Writer {
		let mut out = Writer::stderr();
		dump_stats(true);
		let _ = writeln!(out, "\n===[ PROGRAM ]======");
		let _ = dump_nodes(&mut out, ctx);
		out
	}
}
