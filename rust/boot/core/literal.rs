use super::*;

#[derive(Debug)]
pub enum Literal {
	Bool(bool),
	Int(i64),
	Float(f64),
	Str(&'static str),
}

impl IsValue for Literal {}

#[derive(Debug)]
pub struct ParseLiteral;

impl Eval for ParseLiteral {
	fn precedence(&self) -> Precedence {
		Precedence::Literal
	}

	fn execute(&self, nodes: &[Node]) -> Result<()> {
		let symbols = Symbols::get();
		for it in nodes {
			if let Some(token) = it.cast::<Token>() {
				let value = match token {
					&Token::Word(s, ..) => {
						if s == symbols.TRUE {
							Literal::Bool(true)
						} else if s == symbols.FALSE {
							Literal::Bool(false)
						} else {
							continue;
						}
					}
					Token::Integer(span) => {
						let value = parse_int(span)?;
						Literal::Int(value)
					}
					Token::Float(span) => {
						let value = parse_float(span)?;
						Literal::Float(value)
					}
					Token::Literal(span) => {
						let value = parse_str(span)?;
						Literal::Str(value)
					}
					_ => continue,
				};
				it.set_done(true);
				let node = Node::new_at(value, it.span());
				node.set_done(true);
				it.replace([node]);
			}
		}
		Ok(())
	}
}

fn parse_int(span: &Span) -> Result<i64> {
	let text = span.text();
	let (text, base) = if text.starts_with("0x") || text.starts_with("0X") {
		let text = &text[2..];
		(text, 16)
	} else if text.starts_with("0b") || text.starts_with("0B") {
		let text = &text[2..];
		(text, 2)
	} else if text.starts_with("0c") || text.starts_with("0C") {
		let text = &text[2..];
		(text, 8)
	} else {
		(text, 10)
	};

	parse_digits(text, base, span)
}

fn parse_float(span: &Span) -> Result<f64> {
	let text = span.text();
	let (int, dec) = if let Some(index) = text.find('.') {
		(&text[..index], &text[index + 1..])
	} else {
		(text, "")
	};

	let (int, exp) = if let Some(index) = int.find(|c| matches!(c, 'e' | 'E')) {
		(&text[..index], &text[index + 1..])
	} else {
		(text, "")
	};

	let mut num = String::new();
	for chr in int.chars() {
		if chr == '_' {
			continue;
		}
		num.push(chr);
	}

	let mut has_dec = false;
	for chr in dec.chars() {
		if chr == '_' {
			continue;
		}

		if !has_dec {
			has_dec = true;
			num.push('.');
		}
		num.push(chr);
	}

	let mut has_exp = false;
	for chr in exp.chars() {
		if chr == '_' {
			continue;
		}
		if !has_exp {
			has_exp = true;
			num.push('e');
		}
		num.push(chr);
	}

	let value = match num.parse::<f64>() {
		Ok(value) => value,
		Err(err) => raise!(@span => "invalid floating point literal ({err})"),
	};

	Ok(value)
}

fn parse_str(span: &Span) -> Result<&'static str> {
	let text = span.text();
	let delim = if text.starts_with('\'') {
		"'"
	} else if text.starts_with('\"') {
		"\""
	} else {
		raise!(@span => "invalid delimited string literal")
	};

	if !text.ends_with(delim) {
		raise!(@span => "string missing end `{delim}` delimiter");
	};

	let text = &text[delim.len()..text.len() - delim.len()];
	Ok(text)
}

fn parse_digits(text: &str, base: i64, span: &Span) -> Result<i64> {
	let mut output: i64 = 0;
	for chr in text.chars() {
		if chr == '_' {
			continue;
		}

		let d = match chr {
			'0'..='9' => chr as i64 - ('0' as i64),
			'a'..='z' => chr as i64 - ('a' as i64) + 0xA,
			'A'..='Z' => chr as i64 - ('A' as i64) + 0xA,
			_ => raise!(@span => "invalid digit `{chr}` in numeric literal"),
		};

		if d >= base {
			raise!(@span => "invalid digit `{chr}` for numeric literal in base {base}");
		}

		output = output
			.checked_mul(base)
			.and_then(|v| v.checked_add(base))
			.ok_or_else(|| err!(@span => "numeric literal overflow"))?;
	}
	Ok(output)
}
