use bits::*;

use std::io::Write;

fn main() {
	if let Err(err) = run(false) {
		eprintln!("\nError: {}\n", err.detailed());
		std::process::exit(1);
	}
}

fn run(mut show_stats: bool) -> Result<()> {
	let ctx = Context::new();
	let ctx = ctx.get();
	init_context(ctx)?;

	let sources = ctx.sources();
	for arg in std::env::args().skip(1) {
		if arg == "--mem" || arg == "--stats" {
			show_stats = true;
			continue;
		}

		let src = sources.load_file(arg)?;
		ctx.node(Value::Source(src), src.span());
	}

	let value = process(ctx);

	if show_stats || value.is_err() {
		let stats = Arena::stats();
		let used = stats.used();
		let size = stats.size();
		let max_used = stats.max_used();
		let max_size = stats.max_size();
		let mut out = if show_stats && !value.is_err() {
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

	if value.is_err() {
		let mut out = Writer::stderr();
		let _ = write!(out, "\n========== PROGRAM DUMP ==========\n");
		let _ = dump_nodes(&mut Writer::stderr(), ctx);
		let _ = write!(out, "\n==================================\n");
	}

	let value = value?;
	println!("\nResult = {value:?}\n");

	Ok(())
}
