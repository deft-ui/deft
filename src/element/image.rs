use crate as deft;
use std::fs::File;
use std::io::Cursor;

use anyhow::Error;
use base64::Engine;
use base64::prelude::*;
use deft_macros::{element_backend, js_methods};
use quick_js::JsValue;
use skia_safe::{Canvas};
use skia_safe::svg::Dom;
use skia_safe::wrapper::PointerWrapper;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};

use crate::element::{ElementBackend, Element, ElementWeak};
use crate::element::label::FONT_MGR;
use crate::img_manager::IMG_MANAGER;
use crate::{js_call, ok_or_return};
use crate::render::RenderFn;

extern "C" fn measure_image(node_ref: NodeRef, width: f32, _mode: MeasureMode, _height: f32, _height_mode: MeasureMode) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(img) = ctx.downcast_ref::<ImageSrc>() {
            let (width, height) = img.get_size();
            return Size {
                width,
                height,
            };
        }
    }
    return Size {
        width: 0.0,
        height: 0.0,
    };
}

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
            ImageSrc::Svg(dom) => {
                unsafe {
                    let size = *dom.inner().containerSize();
                    (size.fWidth, size.fHeight)
                }
            }
            ImageSrc::Img(img) => {
                (img.width() as f32, img.height() as f32)
            }
            ImageSrc::None => {
                (0.0, 0.0)
            }
        }
    }
}

#[element_backend]
pub struct Image {
    element: ElementWeak,
    src: String,
    img: ImageSrc,
}

#[js_methods]
impl Image {

    #[js_func]
    pub fn set_src(&mut self, src: String) {
        //TODO optimize data-url parsing
        let base64_prefix = "data:image/svg+xml;base64,";
        self.img = if src.starts_with(base64_prefix) {
            if let Ok(dom) = Self::load_svg_base64(&src[base64_prefix.len()..]) {
                ImageSrc::Svg(dom)
            } else {
                ImageSrc::None
            }
        } else if src.ends_with(".svg") {
            if let Ok(dom) = Self::load_svg(&src) {
                ImageSrc::Svg(dom)
            } else {
                ImageSrc::None
            }
        } else {
            if let Some(img) = IMG_MANAGER.with(|im| im.get_img(&src)) {
                ImageSrc::Img(img)
            } else {
                ImageSrc::None
            }
        };
        let context = Context::new(self.img.clone());
        let mut element = ok_or_return!(self.element.upgrade_mut());
        element.style.set_context(Some(context));
        self.element.mark_dirty(true);
    }

    fn load_svg_base64(data: &str) -> Result<Dom, Error> {
        let bytes = BASE64_STANDARD.decode(data)?;
        let fm = FONT_MGR.with(|fm| fm.clone());
        Ok(Dom::read(Cursor::new(bytes), fm)?)
    }

    fn load_svg(src: &str) -> Result<Dom, Error> {
        let fm = FONT_MGR.with(|fm| fm.clone());
        let data = File::open(src)?;
        Ok(Dom::read(data, fm)?)
    }

}

impl ElementBackend for Image {
    fn create(mut element: &mut Element) -> Self {
        element.style.set_measure_func(Some(measure_image));
        ImageData {
            element: element.as_weak(),
            src: "".to_string(),
            img: ImageSrc::None,
        }.to_ref()
    }

    fn get_name(&self) -> &str {
        "Image"
    }

    fn render(&mut self) -> RenderFn {
        let (img_width, img_height) = self.img.get_size();
        let element = self.element.upgrade_mut().unwrap();
        let (width, height) = element.get_size();
        let img = self.img.clone();
        RenderFn::new(move |canvas| {
            canvas.save();
            canvas.scale((width / img_width, height / img_height));
            match &img {
                ImageSrc::Svg(dom) => {
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

}