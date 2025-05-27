use crate as deft;
use crate::base::EventContext;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::ok_or_return;
use crate::render::RenderFn;
use crate::style::StylePropKey;
use crate::text::textbox::{TextBox, TextCoord, TextElement};
use deft_macros::{element_backend, js_methods};
use std::any::Any;
use yoga::Size;
#[element_backend]
pub struct RichText {
    element: ElementWeak,
    text_box: TextBox,
}

#[js_methods]
impl RichText {
    #[js_func]
    pub fn add_line(&mut self, units: Vec<TextElement>) {
        self.text_box.add_line(units);
    }

    #[js_func]
    pub fn insert_line(&mut self, index: usize, units: Vec<TextElement>) {
        self.text_box.insert_line(index, units);
    }

    #[js_func]
    pub fn delete_line(&mut self, line: usize) {
        self.text_box.delete_line(line);
    }

    #[js_func]
    pub fn update_line(&mut self, index: usize, units: Vec<TextElement>) {
        self.text_box.update_line(index, units);
    }

    #[js_func]
    pub fn clear(&mut self) {
        self.text_box.clear();
    }

    #[js_func]
    pub fn measure_line(&self, units: Vec<TextElement>) -> (f32, f32) {
        self.text_box.measure_line(units)
    }

    #[js_func]
    pub fn get_text_coord_by_char_offset(&self, caret: usize) -> Option<TextCoord> {
        self.text_box.get_text_coord_by_char_offset(caret)
    }

    #[js_func]
    pub fn get_selection_text(&self) -> Option<String> {
        self.text_box.get_selection_text()
    }

    fn layout(&mut self, width: f32) {
        //TODO twice layout occurs here?
        self.text_box.set_layout_width(width);
        self.text_box.layout();
    }
}

impl ElementBackend for RichText {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        let mut text_box = TextBox::new();
        {
            let mut el = element.as_weak();
            text_box.set_repaint_callback(move || el.mark_dirty(false));
        }
        {
            let mut el = element.as_weak();
            text_box.set_layout_callback(move || el.mark_dirty(true));
        }
        let this = RichTextData {
            element: element.as_weak(),
            text_box,
        }
        .to_ref();
        element.style.yoga_node.set_measure_func(this.as_weak(), |rich_text_weak, params| {
            if let Ok(mut rich_text) = rich_text_weak.upgrade() {
                rich_text.layout(params.width);
                return Size {
                    width: rich_text.text_box.max_intrinsic_width(),
                    height: rich_text.text_box.height(),
                };
            }
            return Size {
                width: 0.0,
                height: 0.0,
            }
        });
        this
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let element = ok_or_return!(self.element.upgrade());
        match key {
            StylePropKey::Color => {
                self.text_box.set_color(element.style.color);
            }
            StylePropKey::FontSize => {
                self.text_box.set_font_size(element.style.font_size);
            }
            StylePropKey::FontFamily => {
                self.text_box
                    .set_font_families(element.style.font_family.clone());
            }
            StylePropKey::FontWeight => {
                self.text_box.set_font_weight(element.style.font_weight);
            }
            StylePropKey::FontStyle => {
                self.text_box.set_font_style(element.style.font_style);
            }
            StylePropKey::LineHeight => {
                self.text_box.set_line_height(element.style.line_height);
            }
            _ => {}
        }
    }

    fn render(&mut self) -> RenderFn {
        self.text_box.render()
    }

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        self.text_box.on_event(&event, ctx, 0.0, 0.0);
    }
}
