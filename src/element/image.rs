use std::sync::{Arc, Mutex};
use crate as deft;
use crate::base::Rect;
use crate::js_module;
use crate::element::common::image_object::ImageObject;
use crate::element::{Element, Widget, ElementDelegate, ElementWeak};
use crate::ok_or_return;
use crate::style::StylePropKey;
use deft_macros::{widget, js_methods};
use yoga::Size;
use crate::render::RenderFn;

#[widget]
pub struct Image {
    src: String,
    img: Arc<Mutex<ImageObject>>,
}

#[js_methods]
impl Image {
    
    #[js_func]
    pub fn set_src(&mut self, src: String) {
        self.update_img(ImageObject::new(&src));
    }

    pub fn set_src_svg_raw(&mut self, svg: &[u8]) {
        self.update_img(ImageObject::from_svg_bytes(svg));
    }

    fn update_img(&mut self, img: ImageObject) {
        *self.img.lock().unwrap() = img;
        self.el.mark_dirty(true);
    }

    #[js_func]
    pub fn create() -> Self {
        let element = Element::new("image");
        let mut img = Self {
            el: element,
            src: "".to_string(),
            img: Arc::new(Mutex::new(ImageObject::none())),
        };
        //TODO use weak ref?
        let img_obj = img.img.clone();
        img.el
            .style
            .yoga_node
            .set_measure_func(img_obj, |img, _params| {
                let (width, height) = img.lock().unwrap().get_size();
                return Size { width, height };
            });
        let element = img.el.as_weak();
        let img2 = img.img.clone();
        img.el.set_delegate(ImageDelegate {
            element,
            img: img2,
        });
        img
    }
}

impl Widget for Image {

}

impl ElementDelegate for ImageDelegate {
    fn handle_style_changed(&mut self, key: StylePropKey) {
        let element = ok_or_return!(self.element.upgrade());
        match key {
            StylePropKey::Color => {
                let changed = {
                    let mut img = self.img.lock().unwrap();
                    img.set_color(element.style.color)
                };
                if changed {
                    self.element.mark_dirty(false);
                }
            }
            _ => {}
        }
    }
    fn render(&mut self) -> RenderFn {
        self.img.lock().unwrap().render()
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.img.lock().unwrap().set_container_size((bounds.width, bounds.height));
    }

}

js_module!(Image);

#[derive(Clone)]
pub struct ImageDelegate {
    element: ElementWeak,
    img: Arc<Mutex<ImageObject>>,
}
