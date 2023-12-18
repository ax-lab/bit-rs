use super::*;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OpMatchItem {
	Exact,
	Compatible,
	Convertible,
	Polymorphic,
	Any,
}

const OP_MATCH_COUNT: usize = OpMatchItem::Any as usize + 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OpMatch {
	Exact,
	Score { input: OpMatchCount, output: OpMatchItem },
	None,
}

impl OpMatch {
	pub fn begin() -> Self {
		OpMatch::Score {
			input: Default::default(),
			output: OpMatchItem::Any,
		}
	}
}

impl Default for OpMatch {
	fn default() -> Self {
		Self::begin()
	}
}

impl OpMatch {
	pub fn fail(&mut self) {
		*self = OpMatch::None;
	}

	pub fn set_output(&mut self, mode: OpMatchItem) {
		if let OpMatch::Score { output, .. } = self {
			*output = mode;
		}
	}

	pub fn add_input(&mut self, mode: OpMatchItem) {
		if let OpMatch::Score { input, .. } = self {
			input.add(mode);
		}
	}

	pub fn add_ignored_input(&mut self) {
		if let OpMatch::Score { input, .. } = self {
			input.add_ignored();
		}
	}

	pub fn end(self) -> Self {
		if let OpMatch::Score { input, output } = self {
			if output == OpMatchItem::Exact && input.is_exact() {
				return Self::Exact;
			}
		}
		self
	}
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct OpMatchCount {
	count: [usize; OP_MATCH_COUNT],
}

impl OpMatchCount {
	pub fn add(&mut self, mode: OpMatchItem) {
		self.count[mode as usize] += 1;
	}

	pub fn add_ignored(&mut self) {
		self.add(OpMatchItem::Any);
	}

	pub fn is_exact(&self) -> bool {
		for i in 0..OP_MATCH_COUNT {
			if i == (OpMatchItem::Exact as usize) {
				continue;
			}
			if self.count[i] > 0 {
				return false;
			}
		}
		true
	}
}

impl Ord for OpMatchCount {
	fn cmp(&self, other: &Self) -> Ordering {
		// higher matching count comes first
		self.count.cmp(&other.count).reverse()
	}
}

impl PartialOrd for OpMatchCount {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
