use crate::{some_or_continue, some_or_return};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ComputedValueInner<T> {
    next_id: u32,
    id2keys: HashMap<u32, Vec<String>>,
    consumers: HashMap<u32, Box<dyn FnMut(Vec<T>)>>,
    keys: HashMap<String, Vec<u32>>,
    values: HashMap<String, T>,
}

impl<T: Clone> ComputedValueInner<T> {
    fn new() -> Self {
        Self {
            next_id: 1,
            consumers: HashMap::new(),
            keys: HashMap::new(),
            values: HashMap::new(),
            id2keys: HashMap::new(),
        }
    }

    fn read_value(&self, key: &str) -> Option<T> {
        self.values.get(key).cloned()
    }

    fn get_values(&self, keys: &Vec<String>) -> Option<Vec<T>> {
        let mut result = Vec::new();
        for k in keys {
            let v = some_or_return!(self.values.get(k), None);
            result.push(v.clone());
        }
        Some(result)
    }

    pub fn register_dep<F: FnMut(Vec<T>)>(
        &mut self,
        keys: &Vec<String>,
        mut consumer: Box<dyn FnMut(Vec<T>)>,
    ) -> u32 {
        if let Some(v) = self.get_values(keys) {
            consumer(v);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.id2keys.insert(id, keys.to_owned());
        self.consumers.insert(id, consumer);
        for k in keys {
            self.keys.entry(k.to_owned()).or_insert(Vec::new()).push(id);
        }
        id
    }
    pub fn update(&mut self, key: &str, value: T) {
        self.values.insert(key.to_owned(), value);
        if let Some(ids) = self.keys.get(key) {
            for id in ids {
                let keys = some_or_continue!(self.id2keys.get(id));
                let values = some_or_continue!(self.get_values(keys));
                let consumer = some_or_continue!(self.consumers.get_mut(id));
                consumer(values);
            }
        }
    }
    pub fn unregister_dep(&mut self, id: u32) {
        self.consumers.remove(&id);
        let keys = some_or_return!(self.id2keys.get(&id));
        for k in keys {
            let list = some_or_continue!(self.keys.get_mut(k));
            list.retain(|&x| x != id);
        }
    }
}

#[derive(Clone)]
pub struct ComputedValue<T> {
    inner: Rc<RefCell<ComputedValueInner<T>>>,
}

impl<T: Clone + 'static> ComputedValue<T> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ComputedValueInner::new())),
        }
    }
    pub fn update_value(&self, key: &str, value: T) {
        self.inner.borrow_mut().update(key, value);
    }
    pub fn dep<F: FnMut(Vec<T>) + 'static>(
        &self,
        keys: &Vec<String>,
        consumer: F,
    ) -> ComputedValueHandle {
        let mut inner = self.inner.borrow_mut();
        let id = inner.register_dep::<F>(keys, Box::new(consumer));
        let inner_clone = self.inner.clone();
        let dropper = Box::new(move || {
            let mut inner = inner_clone.borrow_mut();
            inner.unregister_dep(id);
        });
        ComputedValueHandle { dropper }
    }
}

pub struct ComputedValueHandle {
    dropper: Box<dyn FnMut()>,
}

impl Drop for ComputedValueHandle {
    fn drop(&mut self) {
        (self.dropper)()
    }
}

#[cfg(test)]
mod tests {
    use crate::computed::ComputedValue;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn test_computed_value() {
        let cv = ComputedValue::new();
        let counter = Rc::new(Cell::new(0));
        {
            let counter2 = counter.clone();
            let _h1 = cv.dep(&vec!["counter".to_string()], move |v| {
                counter2.set(v[0]);
            });
            cv.update_value("counter", 1);
            assert_eq!(counter.get(), 1);
        }
        cv.update_value("counter", 2);
        assert_eq!(counter.get(), 1);
    }
}
