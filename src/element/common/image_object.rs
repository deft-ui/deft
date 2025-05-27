use crate::element::label::FONT_MGR;
use crate::img_manager::{dyn_image_to_skia_image, IMG_MANAGER};
use crate::render::RenderFn;
use anyhow::Error;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use image::{EncodableLayout, ImageReader};
use log::error;
use skia_safe::svg::Dom;
use skia_safe::wrapper::PointerWrapper;
use skia_safe::Color;
use std::fs::File;
use std::io::Cursor;

#[derive(Clone)]
enum ImageSrc {
    Svg(Dom),
    Img(skia_safe::Image),
    None,
}

unsafe impl Send for ImageSrc {}

impl ImageSrc {
    pub fn get_size(&self) -> (f32, f32) {
        match self {
            ImageSrc::Svg(dom) => unsafe {
                let size = *dom.inner().containerSize();
                (size.fWidth, size.fHeight)
            },
            ImageSrc::Img(img) => (img.width() as f32, img.height() as f32),
            ImageSrc::None => (0.0, 0.0),
        }
    }
}

#[derive(Clone)]
pub struct ImageObject {
    container_size: (f32, f32),
    color: Color,
    img: ImageSrc,
}

impl ImageObject {
    pub fn new(data: &str) -> Self {
        Self {
            container_size: (0.0, 0.0),
            img: Self::load(data),
            color: Color::from_rgb(0, 0, 0),
        }
    }

    pub fn none() -> Self {
        Self {
            container_size: (0.0, 0.0),
            img: ImageSrc::None,
            color: Color::from_rgb(0, 0, 0),
        }
    }

    pub fn from_svg_bytes(data: &[u8]) -> Self {
        let img = Self::load_svg_from_data(data);
        Self {
            container_size: (0.0, 0.0),
            img,
            color: Color::from_rgb(0, 0, 0),
        }
    }

    pub fn set_container_size(&mut self, size: (f32, f32)) {
        self.container_size = size;
    }

    pub fn set_color(&mut self, color: Color) -> bool {
        if let ImageSrc::Svg(_) = self.img {
            self.color = color;
            true
        } else {
            false
        }
    }

    pub fn get_size(&self) -> (f32, f32) {
        self.img.get_size()
    }

    pub fn render(&self) -> RenderFn {
        let (width, height) = self.container_size;
        let (img_width, img_height) = self.img.get_size();
        let img = self.img.clone();
        let color = self.color;
        RenderFn::new(move |painter| {
            let canvas = painter.canvas;
            canvas.save();
            canvas.scale((width / img_width, height / img_height));
            match img {
                ImageSrc::Svg(dom) => {
                    dom.root().set_color(color);
                    dom.render(canvas);
                }
                ImageSrc::Img(img) => {
                    canvas.draw_image(img, (0.0, 0.0), None);
                }
                ImageSrc::None => {}
            }
            canvas.restore();
        })
    }

    fn load(src: &str) -> ImageSrc {
        if let Some(data_url) = src.strip_prefix("data:") {
            if let Some((mime, data)) = Self::parse_data_url(data_url) {
                if mime.starts_with("image/svg") {
                    return Self::load_svg_from_data(data.as_bytes());
                } else {
                    return Self::load_image_from_data(&data);
                }
            }
        } else if src.ends_with(".svg") {
            return if let Ok(dom) = Self::load_svg(&src) {
                ImageSrc::Svg(dom)
            } else {
                ImageSrc::None
            };
        } else {
            return if let Some(img) = IMG_MANAGER.with(|im| im.get_img(&src)) {
                ImageSrc::Img(img)
            } else {
                ImageSrc::None
            };
        }
        ImageSrc::None
    }

    fn load_svg_from_data(data: &[u8]) -> ImageSrc {
        let fm = FONT_MGR.with(|fm| fm.clone());
        match Dom::read(Cursor::new(data), fm) {
            Ok(dom) => ImageSrc::Svg(dom),
            Err(_) => ImageSrc::None,
        }
    }

    fn load_image_from_data(data: &Vec<u8>) -> ImageSrc {
        match ImageReader::new(Cursor::new(data)).decode() {
            Ok(img) => {
                let sk_img = dyn_image_to_skia_image(&img);
                ImageSrc::Img(sk_img)
            }
            Err(e) => {
                error!("Failed to load image: {:?}", e);
                ImageSrc::None
            }
        }
    }

    fn load_svg(src: &str) -> Result<Dom, Error> {
        let fm = FONT_MGR.with(|fm| fm.clone());
        let data = File::open(src)?;
        Ok(Dom::read(data, fm)?)
    }

    fn parse_data_url(url: &str) -> Option<(String, Vec<u8>)> {
        let (params, data) = url.split_once(",")?;
        let (mime, encoding) = params.split_once(";")?;
        if encoding == "base64" {
            Some((mime.to_string(), BASE64_STANDARD.decode(data).ok()?))
        } else {
            None
        }
    }
}
