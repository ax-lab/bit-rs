use std::collections::HashSet;

use boot::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	SOURCES.set_init(PrintSource);

	let source = SourceMap::new(".")?;

	let mut input = HashSet::new();
	for it in std::env::args().skip(1) {
		let src = source.load_file(it)?;
		input.insert(src);
	}

	let mut input = input.into_iter().collect::<Vec<_>>();
	input.sort();

	let options = Options {
		show_output: true,
		..Default::default()
	};

	execute(&input, options)?;

	Ok(())
}
