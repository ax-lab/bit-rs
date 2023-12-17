use bits::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {}\n", err.detailed());
		std::process::exit(1);
	}
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
