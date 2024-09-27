use skia_safe::Canvas;
use winit::window::Window;

pub trait RenderBackend {
    fn window(&self) -> &Window;

    fn render(&mut self, renderer: Box<dyn FnOnce(&Canvas)>);

    fn resize(&mut self, width: u32, height: u32);
}