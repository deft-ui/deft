use std::collections::HashMap;

pub struct IdHashMap<T> {
    next_id: u32,
    hash_map: HashMap<u32, T>,
}

impl<T> IdHashMap<T> {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            hash_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.hash_map.insert(id, value);
        id
    }

    pub fn get(&self, id: &u32) -> Option<&T> {
        self.hash_map.get(id)
    }

    pub fn remove(&mut self, id: u32) -> Option<T> {
        self.hash_map.remove(&id)
    }

    pub fn for_each<F: Fn(u32, &T)>(&self, mut f: F) {
        self.hash_map.iter().for_each(|(id, value)| {
            f(*id, value);
        })
    }

    pub fn for_each_mut<F: Fn(u32, &mut T)>(&mut self, mut f: F) {
        self.hash_map.iter_mut().for_each(|(id, value)| {
            f(*id, value);
        })
    }

}