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

	let mut options = Options::default();
	for it in std::env::args().skip(1) {
		if it == "--show-program" {
			options.show_program = true;
		} else if it == "--dump-code" {
			options.dump_code = true;
		} else if it == "--dump" {
			options.show_program = true;
			options.dump_code = true;
		} else if it == "--compile" {
			options.compile = true;
		} else {
			let src = sources.load_file(it)?;
			input.insert(src);
		}
	}

	let mut input = input.into_iter().collect::<Vec<_>>();
	input.sort();

	execute(&input, options)?;

	Ok(())
}
