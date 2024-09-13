use std::fs::File;
use std::io::Cursor;

use anyhow::Error;
use base64::Engine;
use base64::prelude::*;
use quick_js::JsValue;
use skia_safe::{Canvas};
use skia_safe::svg::Dom;
use skia_safe::wrapper::PointerWrapper;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};

use crate::element::{ElementBackend, ElementRef};
use crate::element::label::FONT_MGR;
use crate::img_manager::IMG_MANAGER;
use crate::js_call;

extern "C" fn measure_image(node_ref: NodeRef, width: f32, _mode: MeasureMode, _height: f32, _height_mode: MeasureMode) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(img) = ctx.downcast_ref::<ImageData>() {
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
enum ImageData {
    Svg(Dom),
    Img(skia_safe::Image),
    None,
}

impl ImageData {
    pub fn get_size(&self) -> (f32, f32) {
        match self {
            ImageData::Svg(dom) => {
                unsafe {
                    let size = *dom.inner().containerSize();
                    (size.fWidth, size.fHeight)
                }
            }
            ImageData::Img(img) => {
                (img.width() as f32, img.height() as f32)
            }
            ImageData::None => {
                (0.0, 0.0)
            }
        }
    }
}

pub struct Image {
    element: ElementRef,
    src: String,
    img: ImageData,
}

impl Image {

    pub fn set_src(&mut self, src: String) {
        //TODO optimize data-url parsing
        let base64_prefix = "data:image/svg+xml;base64,";
        self.img = if src.starts_with(base64_prefix) {
            if let Ok(dom) = Self::load_svg_base64(&src[base64_prefix.len()..]) {
                ImageData::Svg(dom)
            } else {
                ImageData::None
            }
        } else if src.ends_with(".svg") {
            if let Ok(dom) = Self::load_svg(&src) {
                ImageData::Svg(dom)
            } else {
                ImageData::None
            }
        } else {
            if let Some(img) = IMG_MANAGER.with(|im| im.get_img(&src)) {
                ImageData::Img(img)
            } else {
                ImageData::None
            }
        };
        self.element.layout.set_context(Some(Context::new(self.img.clone())));
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
    fn create(mut element: ElementRef) -> Self {
        element.layout.set_measure_func(Some(measure_image));
        Self {
            element,
            src: "".to_string(),
            img: ImageData::None,
        }
    }

    fn get_name(&self) -> &str {
        "Image"
    }

    fn set_property(&mut self, p: &str, v: JsValue) {
        js_call!("src", String , self, set_src, p, v);
    }

    fn draw(&self, canvas: &Canvas) {
        let (img_width, img_height) = self.img.get_size();
        let (width, height) = self.element.get_size();
        canvas.save();
        canvas.scale((width / img_width, height / img_height));
        match &self.img {
            ImageData::Svg(dom) => {
                dom.render(canvas);
            }
            ImageData::Img(img) => {
                canvas.draw_image(img, (0.0, 0.0), None);
            }
            ImageData::None => {}
        }
        canvas.restore();
    }

}