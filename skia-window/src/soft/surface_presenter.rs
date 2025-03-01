use winit::window::Window;

pub trait SurfacePresenter {
    fn window(&self) -> &Window;
    fn resize(&mut self, width: u32, height: u32);
    fn present_surface(&mut self, skia_surface: &mut skia_safe::Surface);
    fn size(&self) -> (u32, u32);
}