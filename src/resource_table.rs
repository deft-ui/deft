use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use crate::mrc::Mrc;

#[derive(Clone)]
pub struct ResourceTable {
    inner: Mrc<ResourceTableInner>,
}

impl ResourceTable {
    pub fn new() -> Self {
        Self {
            inner: Mrc::new(ResourceTableInner::new()),
        }
    }
}

impl Deref for ResourceTable {
    type Target = ResourceTableInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ResourceTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct ResourceTableInner {
    resources: HashMap<i32, Box<dyn Any>>,
    type_resources: HashMap<TypeId, Box<dyn Any>>,
    next_resource_id: i32,
}

impl ResourceTableInner {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            next_resource_id: 1,
            type_resources: HashMap::new(),
        }
    }

    pub fn put<T: 'static>(&mut self, value: T) {
        self.next_resource_id += 1;
        self.type_resources.insert(value.type_id(), Box::new(value));
    }

    pub fn get<T: 'static>(&mut self) -> Option<&T> {
        if let Some(v) = self.type_resources.get(&TypeId::of::<T>()) {
            v.downcast_ref::<T>()
        } else {
            None
        }
    }

}

