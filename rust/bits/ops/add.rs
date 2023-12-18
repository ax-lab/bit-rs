use super::*;

pub struct OpAdd {}

impl Operator for OpAdd {
	fn arity(&self) -> Arity {
		Arity::exact(2)
	}

	fn match_args(&self, op: OpArgQuery) -> OpMatch {
		let mut output = op.input.first().copied().unwrap_or_else(|| KindId::none());
		for &it in op.input.iter().skip(1) {
			output = get_numeric_result(output, it);
			if output.is_none() {
				break;
			}
		}

		output = output.get_result_kind(op.output);
		let _ = output;

		todo!()
	}
}
