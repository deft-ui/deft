use crate::layer::{ILayer, Layer};
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};

pub trait IRenderContext {
    fn create_layer(&mut self, width: usize, height: usize) -> Option<Box<dyn ILayer>>;
}

pub struct RenderContext<'a> {
    context: Box<&'a mut dyn IRenderContext>,
}

unsafe impl Send for RenderContext<'_> {}

impl<'a> RenderContext<'a> {
    pub fn new(context: &'a mut impl IRenderContext) -> Self {
        Self { context: Box::new(context) }
    }

    pub fn create_layer(&mut self, width: usize, height: usize) -> Option<Layer> {
        let layer = self.context.create_layer(width, height)?;
        Some(Layer::new(layer))
    }

}
