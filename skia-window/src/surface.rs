use skia_safe::Canvas;
use winit::window::Window;

pub trait RenderBackend {
    fn window(&self) -> &Window;

    fn render(&mut self, renderer: Box<dyn FnOnce(&Canvas) + Send>, callback: Box<dyn FnOnce(bool) + Send + 'static>);

    fn resize(&mut self, width: u32, height: u32);
}