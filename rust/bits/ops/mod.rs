use super::*;

pub mod arity;
pub mod matching;
pub mod numeric;

pub use arity::*;
pub use matching::*;
pub use numeric::*;

pub mod add;

pub struct OpContext<'a> {
	ctx: ContextRef<'a>,
	map: RwLock<HashMap<OpKey, OpTable<'a>>>,
}

impl<'a> IsContext<'a> for OpContext<'a> {
	fn new(ctx: ContextRef<'a>) -> Self {
		Self {
			ctx,
			map: Default::default(),
		}
	}
}

impl<'a> OpContext<'a> {
	pub fn get(&self, key: OpKey) -> OpTable<'a> {
		if let Some(table) = self.map.read().unwrap().get(&key) {
			return *table;
		}

		let mut table = self.map.write().unwrap();
		let entry = table.entry(key).or_insert_with_key(|key| {
			let data = OpTableData {
				key: *key,
				ctx: self.ctx,
				nullary: Default::default(),
				unary: Default::default(),
			};
			let data = self.ctx.arena().store(data);
			OpTable { data }
		});
		*entry
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum OpKind {
	Core,
	User(Symbol),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct OpKey(pub OpKind, pub Symbol);

#[derive(Copy, Clone)]
pub struct OpTable<'a> {
	data: &'a OpTableData<'a>,
}

struct OpTableData<'a> {
	key: OpKey,
	ctx: ContextRef<'a>,
	nullary: RwLock<HashMap<Type<'a>, Nullary<'a>>>,
	unary: RwLock<HashMap<(Type<'a>, Type<'a>), Unary<'a>>>,
}

impl<'a> OpTable<'a> {
	pub fn key(&self) -> OpKey {
		self.data.key
	}

	pub fn define_nullary(&self, op: Type<'a>) -> Nullary<'a> {
		Self::define(&self.data.nullary, op, || {
			let data = NullaryData {
				key: self.data.key,
				typ: op,
				eval: Default::default(),
			};
			let data = self.data.ctx.arena().store(data);
			Nullary { data }
		})
	}

	pub fn define_unary(&self, op: (Type<'a>, Type<'a>)) -> Unary<'a> {
		Self::define(&self.data.unary, op, || {
			let data = UnaryData {
				key: self.data.key,
				typ: op,
				eval: Default::default(),
			};
			let data = self.data.ctx.arena().store(data);
			Unary { data }
		})
	}

	pub fn get_nullary(&self, op: Type<'a>) -> Option<Nullary<'a>> {
		self.data.nullary.read().unwrap().get(&op).copied()
	}

	pub fn get_unary(&self, op: (Type<'a>, Type<'a>)) -> Option<Unary<'a>> {
		self.data.unary.read().unwrap().get(&op).copied()
	}

	fn define<K: Hash + Eq, V: Copy, F: FnOnce() -> V>(map: &RwLock<HashMap<K, V>>, key: K, init: F) -> V {
		if let Some(val) = map.read().unwrap().get(&key) {
			return *val;
		}
		let mut map = map.write().unwrap();
		let entry = map.entry(key).or_insert_with(init);
		*entry
	}
}

type NullaryEval<'a> = fn(ContextRef<'a>) -> Result<Value<'a>>;
type UnaryEval<'a> = fn(ContextRef<'a>, Value<'a>) -> Result<Value<'a>>;

#[derive(Copy, Clone)]
pub struct Nullary<'a> {
	data: &'a NullaryData<'a>,
}

struct NullaryData<'a> {
	key: OpKey,
	typ: Type<'a>,
	eval: AtomicPtr<NullaryEval<'a>>,
}

impl<'a> Nullary<'a> {
	pub fn set_eval(&self, func: NullaryEval<'a>) {
		self.data
			.eval
			.store(func as *const NullaryEval as *mut _, SyncOrder::Relaxed);
	}

	pub fn eval(&self, ctx: ContextRef<'a>) -> Result<Value<'a>> {
		let eval = self.data.eval.load(SyncOrder::Relaxed);
		if let Some(eval) = unsafe { eval.as_ref() } {
			(eval)(ctx)
		} else {
			let key = self.data.key;
			let typ = self.data.typ;
			err!("eval not defined for {key:?}<{typ:?}>")
		}
	}
}

#[derive(Copy, Clone)]
pub struct Unary<'a> {
	data: &'a UnaryData<'a>,
}

struct UnaryData<'a> {
	key: OpKey,
	typ: (Type<'a>, Type<'a>),
	eval: AtomicPtr<()>,
}

impl<'a> Unary<'a> {
	pub fn set_eval(&self, func: UnaryEval<'a>) {
		let func = func as *mut ();
		self.data.eval.store(func, SyncOrder::Relaxed);
	}

	pub fn eval(&self, ctx: ContextRef<'a>, value: Value<'a>) -> Result<Value<'a>> {
		let eval = self.data.eval.load(SyncOrder::Relaxed);
		if !eval.is_null() {
			let eval: UnaryEval<'a> = unsafe { std::mem::transmute(eval) };
			(eval)(ctx, value)
		} else {
			let key = self.data.key;
			let typ = self.data.typ;
			err!("eval not defined for {key:?}<{typ:?}>")
		}
	}
}

pub trait OperatorX {
	fn arity(&self) -> Arity;

	fn match_args(&self, op: OpArgQueryX) -> OpMatch;

	fn as_nullary(&self) -> Option<&dyn OpNullaryX> {
		None
	}

	fn as_unary(&self) -> Option<&dyn OpUnaryX> {
		None
	}

	fn as_binary(&self) -> Option<&dyn OpBinaryX> {
		None
	}

	fn as_ternary(&self) -> Option<&dyn OpTernaryX> {
		None
	}

	fn as_variadic(&self) -> Option<&dyn OpVariadicX> {
		None
	}
}

pub trait OpNullaryX: OperatorX {
	fn eval(&self) -> Result<XValueCell>;
}

pub trait OpUnaryX: OperatorX {
	fn eval(&self, arg: XValueCell) -> Result<XValueCell>;
}

pub trait OpBinaryX: OperatorX {
	fn eval(&self, lhs: XValueCell, rhs: XValueCell) -> Result<XValueCell>;
}

pub trait OpTernaryX: OperatorX {
	fn eval(&self, a: XValueCell, b: XValueCell, c: XValueCell) -> Result<XValueCell>;
}

pub trait OpVariadicX: OperatorX {
	fn eval(&self, args: &[XValueCell]) -> Result<XValueCell>;
}

pub struct OpTableX {
	ops: Vec<Arc<dyn OperatorX>>,
}

impl OpTableX {
	pub fn query(&self, _query: &OpQueryX, _output: &mut OpResultX) {
		todo!()
	}

	pub fn add(&mut self, op: Arc<dyn OperatorX>) {
		self.ops.push(op);
	}
}

#[derive(Default)]
pub struct OpArgQueryX {
	output: KindId,
	input: Vec<KindId>,
}

#[derive(Default)]
pub struct OpQueryX {
	arity: Arity,
	args: OpArgQueryX,
}

impl OpQueryX {
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

pub struct OpResultX {}

impl OpResultX {
	pub fn len(&self) -> usize {
		todo!()
	}

	pub fn get(&self, _index: usize) -> Option<Arc<dyn OperatorX>> {
		todo!()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn test() {
		let ctx = Context::new();
		let ctx = ctx.get();

		let str = ctx.types().builtin(Primitive::String);
		let val = ctx.str("abc123");

		assert_eq!(str, val.get_type());

		let to_string = OpKey(OpKind::Core, Symbol::str("to_string"));
		let to_string = ctx.ops().get(to_string);
		to_string.define_unary((str, str)).set_eval(|ctx, val| {
			let val = val.get_str().unwrap();
			let val = format!("{val}!!!");
			Ok(ctx.str(val))
		});

		assert_eq!("abc123!!!", format!("{val}"));
	}
}
