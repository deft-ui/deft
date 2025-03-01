mod soft_renderer;
pub mod soft_surface;
mod layer;
mod context;
pub mod surface_presenter;
#[cfg(not(target_os = "android"))]
pub mod softbuffer_surface_presenter;
pub mod gl_presenter;

pub use soft_surface::SoftSurface;