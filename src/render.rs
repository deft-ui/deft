pub mod layout_tree;
pub mod paint_object;
pub mod painter;

use crate::paint::Painter;

pub struct RenderFn {
    render: Box<dyn FnOnce(&Painter) + Send>,
}

impl RenderFn {
    pub fn empty() -> RenderFn {
        RenderFn::new(|_painter| {})
    }

    pub fn new<F: FnOnce(&Painter) + Send + 'static>(render: F) -> RenderFn {
        Self {
            render: Box::new(render),
        }
    }
    pub fn new_multiple<F: FnOnce(&Painter) + Send + 'static>(renders: Vec<F>) -> RenderFn {
        Self::new(move |canvas| {
            for render in renders {
                render(canvas);
            }
        })
    }

    pub fn merge(renders: Vec<RenderFn>) -> RenderFn {
        RenderFn::new(move |painter| {
            for render in renders {
                render.run(painter);
            }
        })
    }

    pub fn run(self, canvas: &Painter) {
        (self.render)(canvas);
    }
}
