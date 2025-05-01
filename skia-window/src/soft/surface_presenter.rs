use crate::paint::Canvas;
use winit::window::Window;

pub trait SurfacePresenter {
    fn window(&self) -> &Window;
    fn resize(&mut self, width: u32, height: u32);
    fn render(&mut self, renderer: Box<dyn FnOnce(&Canvas) + Send>, callback: Box<dyn FnOnce(bool) + Send + 'static>);
    fn size(&self) -> (u32, u32);
}