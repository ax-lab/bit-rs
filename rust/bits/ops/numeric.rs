use super::*;

pub fn get_numeric_result(a: KindId, b: KindId) -> KindId {
	if a.is_known() && !a.is_numeric() {
		return KindId::none();
	}

	if b.is_known() && !b.is_numeric() {
		return KindId::none();
	}

	if !a.is_known() || !b.is_known() {
		return KindId::unknown();
	}

	if a == b {
		return a;
	}

	if a.can_promote_to(b) {
		b
	} else if b.can_promote_to(a) {
		a
	} else {
		KindId::none()
	}
}

impl KindId {
	pub fn is_numeric(self) -> bool {
		match self.as_kind() {
			Kind::None => false,
			Kind::Unknown => false,
			Kind::Any => false,
			Kind::Unit => false,
			Kind::Bool => false,
			Kind::Int(_) => true,
			Kind::Float(_) => true,
			Kind::Str => false,
			Kind::Array(_) => false,
		}
	}

	pub fn can_promote_to(self, to: KindId) -> bool {
		match self.as_kind() {
			Kind::Int(ta) => {
				if let Kind::Int(tb) = to.as_kind() {
					ta.can_promote_to(*tb)
				} else {
					false
				}
			}
			Kind::Float(ta) => {
				if let Kind::Float(tb) = to.as_kind() {
					tb.bits() >= ta.bits()
				} else {
					false
				}
			}
			_ => false,
		}
	}

	pub fn get_result_kind(self, expected_output: KindId) -> KindId {
		if self.is_none() || expected_output.is_none() {
			KindId::none()
		} else if self.is_unknown() {
			KindId::unknown()
		} else if expected_output.is_unknown() {
			self
		} else if self.can_promote_to(expected_output) {
			expected_output
		} else if expected_output.as_kind() == &Kind::Int(IntKind::Lit) {
			self
		} else {
			KindId::none()
		}
	}
}

impl IntKind {
	pub fn can_promote_to(self, to: IntKind) -> bool {
		if self == to {
			true
		} else if self.signed() == to.signed() {
			self.bits() <= to.bits()
		} else if to.signed() {
			self.bits() < to.bits()
		} else {
			false
		}
	}
}

impl FloatKind {
	pub fn can_promote_to(self, to: FloatKind) -> bool {
		self == to || self.bits() <= to.bits()
	}
}
