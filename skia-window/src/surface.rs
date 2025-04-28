use winit::window::Window;
use crate::renderer::Renderer;

pub trait RenderBackend {
    fn window(&self) -> &Window;

    fn render(
        &mut self,
        renderer: Renderer,
        callback: Box<dyn FnOnce(bool) + Send + 'static>,
    );

    fn resize(&mut self, width: u32, height: u32);
}