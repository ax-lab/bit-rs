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

	let mut show_program = false;
	let mut dump_code = false;
	for it in std::env::args().skip(1) {
		if it == "--show-program" {
			show_program = true;
		} else if it == "--dump-code" {
			dump_code = true;
		} else if it == "--dump" {
			show_program = true;
			dump_code = true;
		}
		let src = sources.load_file(it)?;
		input.insert(src);
	}

	let mut input = input.into_iter().collect::<Vec<_>>();
	input.sort();

	let options = Options {
		show_program,
		dump_code,
		..Default::default()
	};

	execute(&input, options)?;

	Ok(())
}
