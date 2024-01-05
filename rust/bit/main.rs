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
		ctx.node(Value::Source(src), src.span());
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
	fn simple_string() -> Result<()> {
		let ctx = Context::new();
		check(&ctx, Value::Str("abc"), "", "'abc'")
	}

	#[test]
	fn simple_int() -> Result<()> {
		let ctx = Context::new();
		check(&ctx, Value::SInt(42), "", "42")
	}

	#[test]
	fn hello_world() -> Result<()> {
		let ctx = Context::new();
		check(&ctx, Value::Unit, "hello world\n", "print 'hello world'")
	}

	#[test]
	fn simple_variable() -> Result<()> {
		let ctx = Context::new();
		check(&ctx, Value::SInt(42), "", src(["let x = 42", "x"]))
	}

	#[test]
	fn variable_shadowing() -> Result<()> {
		let ctx = Context::new();
		check(
			&ctx,
			Value::SInt(69),
			"42\n",
			src(["let x = 42", "print x", "let x = 69", "x"]),
		)
	}

	fn check<'a, T: Into<String>>(
		ctx: &'a Context,
		expected_value: Value<'a>,
		expected_output: &str,
		code: T,
	) -> Result<()> {
		let mut out = String::new();

		let ctx = ctx.get();
		init_context(ctx)?;

		let src = ctx.sources().from_string("eval", code);
		ctx.node(Value::Source(src), src.span());

		let w = Writer::fmt(&mut out);
		let ans = execute(ctx, w);

		let ans = match ans {
			Ok(val) => val,
			Err(err) => {
				let mut out = Writer::stderr();

				dump_stats(true);
				let _ = writeln!(out, "\n===[ PROGRAM ]======");
				let _ = dump_nodes(&mut out, ctx);
				let _ = writeln!(out, "\n===[ EVAL ERROR ]===\n\n{err}\n\n====================\n");
				return Err("Eval failed".to_error());
			}
		};
		assert_eq!(expected_value, ans);
		assert_eq!(expected_output, out);
		Ok(())
	}

	fn src<T: IntoIterator<Item = U>, U: Into<String>>(src: T) -> String {
		let lines = src.into_iter().map(|x| x.into()).collect::<Vec<_>>();
		lines.join("\n")
	}
}
