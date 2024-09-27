use skia_safe::{Canvas, Surface, surfaces};

pub struct SoftRenderer {
    surface: Surface,
}

impl SoftRenderer {
    pub fn new(width: i32, height: i32) -> Self {
        let surface = surfaces::raster_n32_premul((width, height)).unwrap();
        SoftRenderer {
            surface,
        }
    }

    pub fn surface(&mut self) -> &mut Surface {
        &mut self.surface
    }

    pub fn canvas(&mut self) -> &Canvas {
        self.surface.canvas()
    }

}