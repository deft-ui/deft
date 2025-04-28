use skia_safe::{Surface, surfaces};
use crate::paint::Canvas;

pub struct SoftRenderer {
    skia_surface: Surface,
}

impl SoftRenderer {
    pub fn new(width: i32, height: i32) -> Self {
        let skia_surface = surfaces::raster_n32_premul((width, height)).unwrap();
        SoftRenderer {
            skia_surface,
        }
    }

    pub fn skia_surface(&mut self) -> &mut Surface {
        &mut self.skia_surface
    }

    pub fn canvas(&mut self) -> &Canvas {
        self.skia_surface.canvas()
    }

}