use super::*;

#[derive(Debug)]
pub struct Group;

impl IsValue for Group {
	fn is_collection(&self) -> bool {
		true
	}
}
