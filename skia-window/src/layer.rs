use crate::paint::{Canvas, Image};

pub trait ILayer {
    fn canvas(&mut self) -> &Canvas;
    fn as_image(&mut self) -> Image;
}

pub struct Layer {
    layer: Box<dyn ILayer>,
}

impl Layer {
    pub fn new(layer: Box<dyn ILayer>) -> Self {
        Layer { layer }
    }

    pub fn canvas(&mut self) -> &Canvas {
        self.layer.canvas()
    }

    pub fn as_image(&mut self) -> Image {
        self.layer.as_image()
    }
}
