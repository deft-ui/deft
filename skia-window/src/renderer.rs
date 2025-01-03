use crate::context::RenderContext;
use skia_safe::Canvas;

pub struct Renderer {
    renderer: Box<dyn FnOnce(&Canvas, &mut RenderContext) + Send + 'static>,
}

impl Renderer {
    pub fn new(renderer: impl FnOnce(&Canvas, &mut RenderContext) + Send + 'static) -> Renderer {
        Renderer {
            renderer: Box::new(renderer),
        }
    }

    pub fn render(self, canvas: &Canvas, ctx: &mut RenderContext) {
        (self.renderer)(canvas, ctx)
    }
}
