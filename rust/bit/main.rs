use std::collections::HashSet;

use boot::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	let source = SourceMap::new(".")?;

	let mut input = HashSet::new();
	for it in std::env::args().skip(1) {
		let src = source.load_file(it)?;
		input.insert(src);
	}

	let mut input = input.into_iter().collect::<Vec<_>>();
	input.sort();

	for it in input {
		println!(">>> {}\n", it.name());
		println!("\n{}\n", it.text());
	}

	Ok(())
}
