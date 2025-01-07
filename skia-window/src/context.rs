use std::any::{Any, TypeId};
use std::collections::HashMap;
use crate::layer::{ILayer, Layer};
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};

pub trait IRenderContext {
    fn create_layer(&mut self, width: usize, height: usize) -> Option<Box<dyn ILayer>>;
}

pub struct UserContext {
    state: HashMap<TypeId, Box<dyn Any>>
}

impl UserContext {
    pub fn new() -> UserContext {
        UserContext {
            state: HashMap::new()
        }
    }

    pub fn set<T: Any>(&mut self, state: T) {
        self.state.insert(TypeId::of::<T>(), Box::new(state));
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        if let Some(v) = self.state.get(&TypeId::of::<T>()) {
            v.downcast_ref::<T>()
        } else {
            None
        }
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        if let Some(v) = self.state.get_mut(&TypeId::of::<T>()) {
            v.downcast_mut::<T>()
        } else {
            None
        }
    }

    pub fn get_mut_or_create<T: Any, F: FnOnce() -> T>(&mut self, creator: F) -> &mut T {
        let entry = self.state.entry(TypeId::of::<T>());
        entry.or_insert_with(|| Box::new(creator())).downcast_mut::<T>().unwrap()
    }

    pub fn take<T: Any>(&mut self) -> Option<T> {
        let v = self.state.remove(&TypeId::of::<T>())?;
        let v = v.downcast::<T>().ok()?;
        Some(*v)
    }

}

pub struct RenderContext<'a> {
    context: Box<&'a mut dyn IRenderContext>,
    pub user_context: &'a mut UserContext,
}

unsafe impl Send for RenderContext<'_> {}

impl<'a> RenderContext<'a> {
    pub fn new(context: &'a mut impl IRenderContext, user_context: &'a mut UserContext) -> Self {
        Self { context: Box::new(context), user_context }
    }

    pub fn create_layer(&mut self, width: usize, height: usize) -> Option<Layer> {
        let layer = self.context.create_layer(width, height)?;
        Some(Layer::new(layer))
    }

}
