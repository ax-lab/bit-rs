use super::*;

pub struct Expr {}

impl Expr {
	pub fn resolve(&self) -> Result<Expr> {
		todo!()
	}
}

pub struct Source {} // Source("let x = a + b")

pub struct TokenList {} // TokenList("let", "x", "=", "a", "+", "b")

pub struct Id(Symbol);

pub struct OpExpr {} // OpExpr(...) ==> OpAssign(Let("x"), OpAdd(Id("a"), Id("b")) ==> OpAssign(Var(x:i32), OpAddI32(CastI8toI32(Var(a:i8)), Var(b:i32)))

pub struct OpAdd {
	pub lhs: Expr,
	pub rhs: Expr,
}

pub struct OpAddI32 {
	pub lhs: Expr,
	pub rhs: Expr,
}

pub struct CastI8toI32 {
	pub val: Expr,
}

pub struct OpAssign {
	pub lhs: Expr,
	pub rhs: Expr,
}
