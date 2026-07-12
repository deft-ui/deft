use crate as deft;
use crate::element::{Element, Widget, ElementDelegate, ElementWeak};
use crate::js_module;
use crate::ok_or_return;
use crate::style::StylePropKey;
use crate::text::textbox::{TextBox, TextCoord, TextElement};
use deft_macros::{widget, js_methods};
use yoga::Size;
use crate::render::RenderFn;

struct RichTextState {
    element: ElementWeak,
    text_box: TextBox,
}

#[widget]
pub struct RichText {
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

    #[js_func]
    pub fn create() -> Self {
        let mut element = Element::new("rich-text");
        let mut text_box = TextBox::new();
        {
            let mut el = element.as_weak();
            text_box.set_repaint_callback(move || el.mark_dirty(false));
        }
        {
            let mut el = element.as_weak();
            text_box.set_layout_callback(move |_has_text| el.mark_dirty(true));
        }
        text_box.bind_event(&mut element);
        element.set_delegate(RichTextState {
            element: element.as_weak(),
            text_box: text_box.clone(),
        });
        let mut this = Self {
            el: element,
            text_box,
        };
        let textbox_weak = this.text_box.as_weak();
        this.el
            .style
            .yoga_node
            .set_measure_func(textbox_weak, |textbox_weak, params| {
                if let Ok(mut text_box) = textbox_weak.upgrade() {
                    text_box.set_layout_width(params.width);
                    text_box.layout();
                    return Size {
                        width: text_box.max_intrinsic_width(),
                        height: text_box.height(),
                    };
                }
                return Size {
                    width: 0.0,
                    height: 0.0,
                };
            });
        this
    }
}

impl Widget for RichText {

    // fn on_event(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
    //     self.text_box.on_event(&event, ctx, 0.0, 0.0);
    // }
}

impl ElementDelegate for RichTextState {
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
}

js_module!(RichText);