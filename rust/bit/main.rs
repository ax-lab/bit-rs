use std::collections::HashSet;

use boot::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\n{err}\n");
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	init_core();

	let sources = SourceMap::new(".")?;
	let mut input = HashSet::new();
	for it in std::env::args().skip(1) {
		let src = sources.load_file(it)?;
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
