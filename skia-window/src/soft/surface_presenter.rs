use skia_safe::Canvas;
use winit::window::Window;

pub trait SurfacePresenter {
    fn window(&self) -> &Window;
    fn resize(&mut self, width: u32, height: u32);
    fn render(&mut self, renderer: Box<dyn FnOnce(&Canvas)>);
    fn size(&self) -> (u32, u32);
}