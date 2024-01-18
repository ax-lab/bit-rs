use super::*;

pub struct Table<K: Hash + Clone + Eq + PartialEq, V: 'static> {
	data: RwLock<HashMap<K, &'static V>>,
}

impl<K: Hash + Clone + Eq + PartialEq, V: 'static> Table<K, V> {
	#[inline(always)]
	pub fn get(&self, key: &K) -> &'static V
	where
		V: Default,
	{
		self.get_or_init(key, |_| V::default())
	}

	#[inline(always)]
	pub fn get_or_init<F: FnOnce(&K) -> V>(&self, key: &K, init: F) -> &'static V {
		if let Some(value) = self.data.read().unwrap().get(key) {
			return *value;
		}

		let mut data = self.data.write().unwrap();
		let entry = data
			.entry(key.clone())
			.or_insert_with_key(|key| Arena::get().store(init(key)));
		*entry
	}
}

impl<K: Hash + Clone + Eq + PartialEq, V: 'static> Default for Table<K, V> {
	fn default() -> Self {
		Self {
			data: Default::default(),
		}
	}
}
