pub mod cssborder;
pub mod layout_tree;
pub mod paint_object;
pub mod painter;

use crate::paint::Painter;

pub struct RenderFn {
    render: Box<dyn FnMut(&Painter) + Send>,
}

impl RenderFn {
    pub fn empty() -> RenderFn {
        RenderFn::new(|_painter| {})
    }

    pub fn new<F: FnMut(&Painter) + Send + 'static>(render: F) -> RenderFn {
        Self {
            render: Box::new(render),
        }
    }
    pub fn new_multiple<F: FnMut(&Painter) + Send + 'static>(mut renders: Vec<F>) -> RenderFn {
        Self::new(move |canvas| {
            for render in &mut renders {
                render(canvas);
            }
        })
    }

    pub fn merge(mut renders: Vec<RenderFn>) -> RenderFn {
        RenderFn::new(move |painter| {
            for render in &mut renders {
                render.run(painter);
            }
        })
    }

    pub fn run(&mut self, canvas: &Painter) {
        (self.render)(canvas);
    }
}
