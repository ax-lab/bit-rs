#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Arity {
	Nullary,
	Unary,
	Binary,
	Ternary,
	Variadic { min: Option<usize>, max: Option<usize> },
}

impl Default for Arity {
	fn default() -> Self {
		Arity::any()
	}
}

impl Arity {
	pub fn any() -> Arity {
		Arity::Variadic { min: None, max: None }
	}

	pub fn exact(n: usize) -> Arity {
		match n {
			0 => Arity::Nullary,
			1 => Arity::Unary,
			2 => Arity::Binary,
			3 => Arity::Ternary,
			n => Arity::Variadic {
				min: Some(n),
				max: Some(n),
			},
		}
	}

	pub fn min(n: usize) -> Arity {
		Arity::Variadic {
			min: Some(n),
			max: None,
		}
	}

	pub fn max(n: usize) -> Arity {
		if n == 0 {
			Arity::Nullary
		} else {
			Arity::Variadic {
				min: None,
				max: Some(n),
			}
		}
	}

	pub fn between(min: usize, max: usize) -> Arity {
		let (min, max) = if max < min { (max, min) } else { (min, max) };
		if min == max {
			Self::exact(min)
		} else {
			Arity::Variadic {
				min: Some(min),
				max: Some(max),
			}
		}
	}

	pub fn allows(&self, other: &Arity) -> bool {
		match other {
			Arity::Nullary => self.allows_count(0),
			Arity::Unary => self.allows_count(1),
			Arity::Binary => self.allows_count(2),
			Arity::Ternary => self.allows_count(3),
			Arity::Variadic { min, max } => {
				if let &Some(min) = min {
					if !self.allows_count(min) {
						return false;
					}
				}
				if let &Some(max) = max {
					if !self.allows_count(max) {
						return false;
					}
				}
				true
			}
		}
	}

	pub fn allows_count(&self, arg_count: usize) -> bool {
		match self {
			Arity::Nullary => arg_count == 0,
			Arity::Unary => arg_count == 1,
			Arity::Binary => arg_count == 2,
			Arity::Ternary => arg_count == 3,
			Arity::Variadic { min, max } => {
				if let &Some(min) = min {
					if arg_count < min {
						return false;
					}
				}
				if let &Some(max) = max {
					if arg_count > max {
						return false;
					}
				}
				true
			}
		}
	}
}
