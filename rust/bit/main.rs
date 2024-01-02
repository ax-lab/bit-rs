use bits::*;

use std::io::Write;

fn main() {
	if let Err(err) = run(true) {
		eprintln!("\nError: {}\n", err.detailed());
		std::process::exit(1);
	}
}

fn run(show_stats: bool) -> Result<()> {
	let ctx = Context::new();
	let ctx = ctx.get();

	ctx.bindings().match_any(Match::source()).bind(DebugPrint("sources"));

	let sources = ctx.sources();
	for path in std::env::args().skip(1) {
		let src = sources.load_file(path)?;
		ctx.node(Value::Source(src), src.span());
	}

	let value = process(ctx)?;
	println!("\nResult = {value:?}\n");

	if show_stats {
		let stats = Arena::stats();
		let used = stats.used();
		let size = stats.size();
		let max_used = stats.max_used();
		let max_size = stats.max_size();
		let out = &mut std::io::stdout();

		let _ = print_bytes(out, "\nMemory used: ", used);
		let _ = print_bytes(out, " out of ", size);
		let _ = print_bytes(out, " (max: ", max_used);
		let _ = print_bytes(out, " / ", max_size);
		let _ = write!(out, ")\n\n");
	}

	Ok(())
}
