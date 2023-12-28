use bits::*;

use std::io::Write;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {}\n", err.detailed());
		std::process::exit(1);
	}

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

fn run() -> Result<()> {
	let sources = SourceMap::new(".")?;
	for path in std::env::args().skip(1) {
		let src = sources.load_file(path)?;
		println!("\n>>> {src:?} <<<\n");
		println!("{}\n", src.text());
	}

	Ok(())
}
