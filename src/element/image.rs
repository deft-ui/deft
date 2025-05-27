use crate as deft;
use crate::base::Rect;
use crate::element::common::image_object::ImageObject;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::ok_or_return;
use crate::render::RenderFn;
use crate::style::StylePropKey;
use deft_macros::{element_backend, js_methods};
use yoga::Size;
#[element_backend]
pub struct Image {
    element: ElementWeak,
    src: String,
    img: ImageObject,
}

#[js_methods]
impl Image {
    #[js_func]
    pub fn set_src(&mut self, src: String) {
        self.img = ImageObject::new(&src);
        self.element.mark_dirty(true);
    }

    pub fn set_src_svg_raw(&mut self, svg: &[u8]) {
        self.img = ImageObject::from_svg_bytes(svg);
        self.element.mark_dirty(true);
    }
}

impl ElementBackend for Image {
    fn create(element: &mut Element) -> Self {
        let img = ImageData {
            element: element.as_weak(),
            src: "".to_string(),
            img: ImageObject::none(),
        }
        .to_ref();
        element
            .style
            .yoga_node
            .set_measure_func(img.as_weak(), |img, _params| {
                if let Ok(img) = img.upgrade() {
                    let (width, height) = img.img.get_size();
                    return Size { width, height };
                }
                return Size {
                    width: 0.0,
                    height: 0.0,
                };
            });
        img
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let element = ok_or_return!(self.element.upgrade());
        match key {
            StylePropKey::Color => {
                if self.img.set_color(element.style.color) {
                    self.element.mark_dirty(false);
                }
            }
            _ => {}
        }
    }

    fn render(&mut self) -> RenderFn {
        self.img.render()
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.img.set_container_size((bounds.width, bounds.height));
    }
}
