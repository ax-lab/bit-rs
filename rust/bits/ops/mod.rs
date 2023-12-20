use super::*;

pub mod arity;
pub mod matching;
pub mod numeric;

pub use arity::*;
pub use matching::*;
pub use numeric::*;

pub mod add;

pub trait Operator {
	fn arity(&self) -> Arity;

	fn match_args(&self, op: OpArgQuery) -> OpMatch;

	fn as_nullary(&self) -> Option<&dyn OpNullary> {
		None
	}

	fn as_unary(&self) -> Option<&dyn OpUnary> {
		None
	}

	fn as_binary(&self) -> Option<&dyn OpBinary> {
		None
	}

	fn as_ternary(&self) -> Option<&dyn OpTernary> {
		None
	}

	fn as_variadic(&self) -> Option<&dyn OpVariadic> {
		None
	}
}

pub trait OpNullary: Operator {
	fn eval(&self) -> Result<XValueCell>;
}

pub trait OpUnary: Operator {
	fn eval(&self, arg: XValueCell) -> Result<XValueCell>;
}

pub trait OpBinary: Operator {
	fn eval(&self, lhs: XValueCell, rhs: XValueCell) -> Result<XValueCell>;
}

pub trait OpTernary: Operator {
	fn eval(&self, a: XValueCell, b: XValueCell, c: XValueCell) -> Result<XValueCell>;
}

pub trait OpVariadic: Operator {
	fn eval(&self, args: &[XValueCell]) -> Result<XValueCell>;
}

pub struct OpTable {
	ops: Vec<Arc<dyn Operator>>,
}

impl OpTable {
	pub fn query(&self, _query: &OpQuery, _output: &mut OpResult) {
		todo!()
	}

	pub fn add(&mut self, op: Arc<dyn Operator>) {
		self.ops.push(op);
	}
}

#[derive(Default)]
pub struct OpArgQuery {
	output: KindId,
	input: Vec<KindId>,
}

#[derive(Default)]
pub struct OpQuery {
	arity: Arity,
	args: OpArgQuery,
}

impl OpQuery {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_arity(&mut self, arity: Arity) {
		self.arity = arity;
	}

	pub fn with_output(&mut self, kind: KindId) {
		self.args.output = kind;
	}

	pub fn with_input(&mut self, index: usize, kind: KindId) {
		while self.args.input.len() <= index {
			self.args.input.push(KindId::unknown());
		}
		self.args.input[index] = kind;
	}
}

pub struct OpResult {}

impl OpResult {
	pub fn len(&self) -> usize {
		todo!()
	}

	pub fn get(&self, _index: usize) -> Option<Arc<dyn Operator>> {
		todo!()
	}
}
