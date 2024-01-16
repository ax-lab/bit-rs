use boot::*;

fn main() {
	if let Err(err) = run() {
		eprintln!("\nError: {err}\n");
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	Ok(())
}
