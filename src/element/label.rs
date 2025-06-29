use crate as deft;
use crate::base::Rect;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::TextUpdateEvent;
use crate::mrc::Mrc;
use crate::ok_or_return;
use crate::render::RenderFn;
use crate::style::StylePropKey;
use crate::text::textbox::{TextBox, TextElement, TextUnit};
use crate::text::TextAlign;
use deft_macros::{element_backend, js_methods};
use yoga::Size;

pub fn parse_align(align: &str) -> TextAlign {
    match align {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        _ => TextAlign::Left,
    }
}

#[element_backend]
pub struct Label {
    text: String,
    state: Mrc<LabelState>,
    element: ElementWeak,
}

struct LabelState {
    text_box: TextBox,
    layout_calculated: bool,
}

#[js_methods]
impl Label {
    #[js_func]
    pub fn set_text(&mut self, text: String) {
        let old_text = self.get_text();
        if old_text != text {
            self.text = text.clone();
            self.state.text_box.clear();
            let text_unit = self.build_text_unit(text.clone());
            self.state
                .text_box
                .add_line(vec![TextElement::Text(text_unit)]);
            self.mark_dirty(true);

            self.element.emit(TextUpdateEvent { value: text })
        }
    }

    #[js_func]
    pub fn get_text(&self) -> String {
        self.text.clone()
    }

    fn mark_dirty(&mut self, layout_dirty: bool) {
        self.element.mark_dirty(layout_dirty);
    }

    fn build_text_unit(&self, text: String) -> TextUnit {
        TextUnit {
            text,
            font_families: None,
            font_size: None,
            color: None,
            text_decoration_line: None,
            weight: None,
            background_color: None,
            style: None,
        }
    }
}

impl ElementBackend for Label {
    fn create(ele: &mut Element) -> Self {
        let text = "".to_string();
        let state = LabelState {
            text_box: TextBox::new(),
            layout_calculated: false,
        };
        let label = LabelData {
            text,
            state: Mrc::new(state),
            element: ele.as_weak(),
        }
        .to_ref();
        ele.style
            .yoga_node
            .set_measure_func(label.state.clone(), |state, params| {
                state.text_box.set_layout_width(params.width);
                state.text_box.layout();
                state.layout_calculated = true;
                let width = state.text_box.max_intrinsic_width();
                let height = state.text_box.height();
                // log::debug!("text measure params:{}x{}", params.width, params.height);
                // log::debug!("text measure result:{}x{}, {}", width, height, state.text_box.get_text());
                return Size { width, height };
            });
        label
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let element = ok_or_return!(self.element.upgrade());
        match key {
            StylePropKey::Color => {
                let color = element.style.color;
                self.state.text_box.set_color(color);
                //TODO optimize dont relayout
                self.mark_dirty(true);
            }
            StylePropKey::FontSize => {
                let font_size = element.style.font_size;
                self.state.text_box.set_font_size(font_size);
                self.mark_dirty(true);
            }
            StylePropKey::FontFamily => {
                let font_families = element.style.font_family.clone();
                self.state.text_box.set_font_families(font_families);
                self.mark_dirty(true);
            }
            StylePropKey::FontWeight => {
                let font_weight = element.style.font_weight;
                self.state.text_box.set_font_weight(font_weight);
                self.mark_dirty(true);
            }
            StylePropKey::FontStyle => {
                let font_style = element.style.font_style.clone();
                self.state.text_box.set_font_style(font_style);
                self.mark_dirty(true);
            }
            StylePropKey::LineHeight => {
                let line_height = element.style.line_height;
                self.state.text_box.set_line_height(line_height);
                self.mark_dirty(true);
            }
            _ => {}
        }
    }

    fn render(&mut self) -> RenderFn {
        let el = ok_or_return!(self.element.upgrade(), RenderFn::empty());
        let (pt, _, _, pl) = el.get_padding();
        let text_renderer = self.state.text_box.render();
        RenderFn::new(move |painter| {
            painter.canvas.translate((pl, pt));
            text_renderer.run(painter);
        })
    }

    fn before_layout(&mut self) {
        self.state.layout_calculated = false;
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        if !self.state.layout_calculated {
            self.state.text_box.set_layout_width(bounds.width);
            self.state.text_box.layout();
            self.state.layout_calculated = true;
        }
    }
}
