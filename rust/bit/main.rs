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
		let stats = Arena::stats();
		let used = stats.used();
		let size = stats.size();
		let max_used = stats.max_used();
		let max_size = stats.max_size();
		let mut out = if args.show_stats && !value.is_err() {
			Writer::stdout()
		} else {
			Writer::stderr()
		};

		let _ = print_bytes(&mut out, "\n[INFO] Memory used: ", used);
		let _ = print_bytes(&mut out, " out of ", size);
		let _ = print_bytes(&mut out, " (max: ", max_used);
		let _ = print_bytes(&mut out, " / ", max_size);
		let _ = write!(out, ")\n");
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

pub fn eval<'a, T: Into<String>>(ctx: &'a Context, output: &'a mut String, code: T) -> Result<Value<'a>> {
	let ctx = ctx.get();
	init_context(ctx)?;

	let src = ctx.sources().from_string("eval", code);
	ctx.node(Value::Source(src), src.span());

	let out = Writer::fmt(output);
	execute(ctx, out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hello_world() -> Result<()> {
		let (mut ctx, mut out) = (Context::new(), String::new());
		let ans = eval(&mut ctx, &mut out, "print 'hello world'")?;
		assert_eq!(Value::Unit, ans);
		assert_eq!("hello world\n", out);
		Ok(())
	}
}
