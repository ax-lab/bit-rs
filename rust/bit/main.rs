use bits::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {}\n", err.detailed());
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	println!("Bit {}", version());
	Ok(())
}
