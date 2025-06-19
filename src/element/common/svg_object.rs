use anyhow::Error;
use resvg::usvg::Tree;
use resvg::{tiny_skia, usvg, RenderOptions};
use skia_safe::Color;
use skia_safe::{surfaces, AlphaType, Canvas, ColorType, ImageInfo};
use std::fs;
use std::sync::{Arc, Mutex};

pub struct SvgState {
    tree: Tree,
    options: RenderOptions,
}

#[derive(Clone)]
pub struct SvgObject {
    state: Arc<Mutex<SvgState>>,
}

impl SvgObject {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let options = usvg::Options::default();
        let tree = Tree::from_data(bytes, &options)?;
        let options = RenderOptions::default();
        Ok(Self {
            state: Arc::new(Mutex::new(SvgState { tree, options })),
        })
    }

    pub fn from_file(file: &str) -> Result<Self, Error> {
        let data = fs::read(file)?;
        Self::from_bytes(&data)
    }
    pub fn container_size(&self) -> (f32, f32) {
        let state = self.state.lock().unwrap();
        let size = state.tree.size();
        (size.width(), size.height())
    }

    pub fn set_color(&self, color: Color) {
        let mut state = self.state.lock().unwrap();
        let color = tiny_skia::Color::from_rgba8(color.r(), color.g(), color.b(), color.a());
        state.options.set_color(color);
    }

    pub fn render(&self, canvas: &Canvas, scale: f32) {
        let state = self.state.lock().unwrap();
        let pixmap_size = state.tree.size().to_int_size().scale_by(scale).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
        resvg::render(
            &state.tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
            &state.options,
        );

        let width = pixmap_size.width() as i32;
        let height = pixmap_size.height() as i32;
        let bi = ImageInfo::new(
            (width, height),
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        );
        let mut surface = surfaces::wrap_pixels(&bi, pixmap.data_mut(), None, None).unwrap();
        let img = surface.image_snapshot();
        canvas.save();
        canvas.scale((1.0 / scale, 1.0 / scale));
        canvas.draw_image(&img, (0.0, 0.0), None);
        canvas.restore();
    }
}
