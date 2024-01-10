use super::*;

pub mod arity;
pub mod matching;

pub use arity::*;
pub use matching::*;

pub fn op_add() -> OpKey {
	OpKey(OpKind::Core, Symbol::str("+"))
}

pub fn op_mul() -> OpKey {
	OpKey(OpKind::Core, Symbol::str("*"))
}

pub fn op_in() -> OpKey {
	OpKey(OpKind::Core, Symbol::str("in"))
}

pub fn op_range() -> OpKey {
	OpKey(OpKind::Core, Symbol::str(".."))
}

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
				binary: Default::default(),
			};
			let data = self.ctx.arena().store(data);
			OpTable { data }
		});
		*entry
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OpKind {
	Core,
	User(Symbol),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct OpKey(pub OpKind, pub Symbol);

impl Default for OpKey {
	fn default() -> Self {
		OpKey(OpKind::Core, Symbol::default())
	}
}

impl Display for OpKey {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let Self(kind, sym) = self;
		match kind {
			OpKind::Core => write!(f, "op")?,
			OpKind::User(s) => {
				s.write_name(f, false)?;
				write!(f, ".op")?
			}
		};
		write!(f, "(")?;
		sym.write_name(f, false)?;
		write!(f, ")")?;
		Ok(())
	}
}

#[derive(Copy, Clone)]
pub struct OpTable<'a> {
	data: &'a OpTableData<'a>,
}

struct OpTableData<'a> {
	key: OpKey,
	ctx: ContextRef<'a>,
	nullary: RwLock<HashMap<RuntimeType<'a>, Nullary<'a>>>,
	unary: RwLock<HashMap<(RuntimeType<'a>, RuntimeType<'a>), Unary<'a>>>,
	binary: RwLock<HashMap<(RuntimeType<'a>, (RuntimeType<'a>, RuntimeType<'a>)), Binary<'a>>>,
}

impl<'a> OpTable<'a> {
	pub fn key(&self) -> OpKey {
		self.data.key
	}

	pub fn define_nullary(&self, out: RuntimeType<'a>) -> Nullary<'a> {
		Self::define(&self.data.nullary, out, || {
			let data = NullaryData {
				key: self.data.key,
				out,
				eval: Default::default(),
			};
			let data = self.data.ctx.arena().store(data);
			Nullary { data }
		})
	}

	pub fn define_unary(&self, out: RuntimeType<'a>, arg: RuntimeType<'a>) -> Unary<'a> {
		Self::define(&self.data.unary, (out, arg), || {
			let data = UnaryData {
				key: self.data.key,
				out,
				arg,
				eval: Default::default(),
			};
			let data = self.data.ctx.arena().store(data);
			Unary { data }
		})
	}

	pub fn define_binary(&self, out: RuntimeType<'a>, args: (RuntimeType<'a>, RuntimeType<'a>)) -> Binary<'a> {
		Self::define(&self.data.binary, (out, args), || {
			let data = BinaryData {
				key: self.data.key,
				out,
				args,
				eval: Default::default(),
			};
			let data = self.data.ctx.arena().store(data);
			Binary { data }
		})
	}

	pub fn get_nullary(&self, op_out: RuntimeType<'a>) -> Vec<Nullary<'a>> {
		let table = self.data.nullary.read().unwrap();
		let mut list = Vec::new();
		for (out, op) in table.iter() {
			if op_out.contains(*out) {
				list.push(*op);
			}
		}
		list
	}

	pub fn get_unary(&self, op_out: RuntimeType<'a>, op_arg: RuntimeType<'a>) -> Vec<Unary<'a>> {
		let table = self.data.unary.read().unwrap();
		let mut list = Vec::new();
		for ((out, arg), op) in table.iter() {
			if op_out.contains(*out) && arg.contains(op_arg) {
				list.push(*op);
			}
		}
		list
	}

	pub fn get_binary_output(&self, out: RuntimeType<'a>, args: (RuntimeType<'a>, RuntimeType<'a>)) -> RuntimeType<'a> {
		let table = self.data.binary.read().unwrap();
		let types = self.data.ctx.types();
		let mut op_out = types.none();
		for op in table.values() {
			if op.matches(out, args) {
				op_out = op_out.sum(op.out());
			}
		}
		op_out
	}

	pub fn get_binary(&self, out: RuntimeType<'a>, args: (RuntimeType<'a>, RuntimeType<'a>)) -> Result<Binary<'a>> {
		let table = self.data.binary.read().unwrap();
		let mut list = Vec::new();
		for op in table.values() {
			if op.matches(out, args) {
				list.push(*op);
			}
		}

		let op = self.data.key;
		let (lhs, rhs) = args;
		match list.len() {
			0 => err!("{op} not defined for ({lhs}, {rhs}) -> {out}"),
			1 => Ok(list[0]),
			_ => {
				let mut output = format!("multiple {op} definitions for ({lhs}, {rhs}) -> {out}:\n");
				for it in list {
					let lhs = it.lhs();
					let rhs = it.rhs();
					let out = it.out();
					output = format!("{output}\n- ({lhs}, {rhs}) -> {out}");
				}
				err!(output.to_error())
			}
		}
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

type NullaryEval<'a> = fn(&mut Runtime<'a>) -> Result<Value<'a>>;
type UnaryEval<'a> = fn(&mut Runtime<'a>, Value<'a>) -> Result<Value<'a>>;
type BinaryEval<'a> = fn(&mut Runtime<'a>, Value<'a>, Value<'a>) -> Result<Value<'a>>;

#[derive(Copy, Clone)]
pub struct Nullary<'a> {
	data: &'a NullaryData<'a>,
}

struct NullaryData<'a> {
	key: OpKey,
	out: RuntimeType<'a>,
	eval: AtomicPtr<NullaryEval<'a>>,
}

impl<'a> Nullary<'a> {
	pub fn set_eval(&self, func: NullaryEval<'a>) {
		self.data
			.eval
			.store(func as *const NullaryEval as *mut _, SyncOrder::Relaxed);
	}

	pub fn eval(&self, rt: &mut Runtime<'a>) -> Result<Value<'a>> {
		let eval = self.data.eval.load(SyncOrder::Relaxed);
		if let Some(eval) = unsafe { eval.as_ref() } {
			(eval)(rt)
		} else {
			let key = self.data.key;
			let out = self.data.out;
			err!("eval not defined for {key:?}<{out:?}>")
		}
	}
}

#[derive(Copy, Clone)]
pub struct Unary<'a> {
	data: &'a UnaryData<'a>,
}

struct UnaryData<'a> {
	key: OpKey,
	out: RuntimeType<'a>,
	arg: RuntimeType<'a>,
	eval: AtomicPtr<()>,
}

impl<'a> Unary<'a> {
	pub fn set_eval(&self, func: UnaryEval<'a>) {
		let func = func as *mut ();
		self.data.eval.store(func, SyncOrder::Relaxed);
	}

	pub fn eval(&self, rt: &mut Runtime<'a>, value: Value<'a>) -> Result<Value<'a>> {
		let eval = self.data.eval.load(SyncOrder::Relaxed);
		if !eval.is_null() {
			let eval: UnaryEval<'a> = unsafe { std::mem::transmute(eval) };
			(eval)(rt, value)
		} else {
			let key = self.data.key;
			let arg = self.data.arg;
			let out = self.data.out;
			err!("eval not defined for {key:?}<{arg:?} -> {out:?}>")
		}
	}
}

#[derive(Copy, Clone)]
pub struct Binary<'a> {
	data: &'a BinaryData<'a>,
}

struct BinaryData<'a> {
	key: OpKey,
	out: RuntimeType<'a>,
	args: (RuntimeType<'a>, RuntimeType<'a>),
	eval: AtomicPtr<()>,
}

impl<'a> Binary<'a> {
	pub fn set_eval(&self, func: BinaryEval<'a>) {
		let func = func as *mut ();
		self.data.eval.store(func, SyncOrder::Relaxed);
	}

	pub fn matches(&self, out: RuntimeType<'a>, (lhs, rhs): (RuntimeType<'a>, RuntimeType<'a>)) -> bool {
		out.contains(self.out()) && self.lhs().contains(lhs) && self.rhs().contains(rhs)
	}

	pub fn out(&self) -> RuntimeType<'a> {
		self.data.out
	}

	pub fn lhs(&self) -> RuntimeType<'a> {
		self.data.args.0
	}

	pub fn rhs(&self) -> RuntimeType<'a> {
		self.data.args.1
	}

	pub fn eval(&self, rt: &mut Runtime<'a>, lhs: Value<'a>, rhs: Value<'a>) -> Result<Value<'a>> {
		let eval = self.data.eval.load(SyncOrder::Relaxed);
		if !eval.is_null() {
			let eval: BinaryEval<'a> = unsafe { std::mem::transmute(eval) };
			(eval)(rt, lhs, rhs)
		} else {
			let key = self.data.key;
			let args = self.data.args;
			let out = self.data.out;
			err!("eval not defined for {key:?}<{args:?} -> {out:?}>")
		}
	}
}

impl<'a> Eq for Binary<'a> {}

impl<'a> PartialEq for Binary<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.data as *const _ == other.data as *const _
	}
}

impl<'a> Hash for Binary<'a> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(self.data as *const BinaryData).hash(state);
	}
}

impl<'a> Debug for Binary<'a> {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		let op = self.data.key;
		let out = self.out();
		let lhs = self.lhs();
		let rhs = self.rhs();
		let ptr = self.data.eval.load(SyncOrder::Relaxed);
		write!(f, "{op} = ({lhs}, {rhs}) -> {out} #{ptr:?}")
	}
}

#[cfg(test)]
#[cfg(off)]
mod tests {
	use super::*;

	#[test]
	pub fn test() {
		let ctx = Context::new();
		let ctx = ctx.get();

		let str = ctx.types().builtin(Primitive::String);
		let node = ctx.str("abc123");

		assert_eq!(str, node.get_type());

		let to_string = OpKey(OpKind::Core, Symbol::str("to_string"));
		let to_string = ctx.ops().get(to_string);
		to_string.define_unary((str, str)).set_eval(|ctx, val| {
			let val = if let Value::Str(val) = val { val } else { unreachable!() };
			let val = format!("{val}!!!");
			Ok(Value::Str(ctx.arena().str(val)))
		});

		assert_eq!("abc123!!!", format!("{node}"));
	}
}
