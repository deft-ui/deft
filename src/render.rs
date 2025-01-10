pub mod paint_node;
pub mod paint_layer;
pub mod paint_tree;
pub mod paint_object;

use skia_safe::Canvas;

pub struct RenderFn {
    render: Box<dyn FnOnce(&Canvas) + Send>,
}

impl RenderFn {
    pub fn empty() -> RenderFn {
        RenderFn::new(|_canvas| {})
    }

    pub fn new<F: FnOnce(&Canvas) + Send + 'static>(render: F) -> RenderFn {
        Self {
            render: Box::new(render),
        }
    }
    pub fn new_multiple<F: FnOnce(&Canvas) + Send + 'static>(renders: Vec<F>) -> RenderFn {
        Self::new(move |canvas| {
            for render in renders {
                render(canvas);
            }
        })
    }

    pub fn run(self, canvas: &Canvas) {
        (self.render)(canvas);
    }
}
