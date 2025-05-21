use crate as deft;
use crate::base::{EventContext, MouseDetail, MouseEventType};
use crate::color::parse_hex_color;
use crate::element::paragraph::simple_paragraph_builder::SimpleParagraphBuilder;
use crate::element::text::simple_text_paragraph::SimpleTextParagraph;
use crate::element::text::text_paragraph::ParagraphRef;
use crate::element::text::{intersect_range, ColOffset, RowOffset};
use crate::element::{text, Element, ElementBackend, ElementWeak};
use crate::event::{
    FocusShiftEvent, KeyDownEvent, KeyEventDetail, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    SelectEndEvent, SelectMoveEvent, SelectStartEvent, TouchEndEvent, TouchMoveEvent,
    TouchStartEvent, KEY_MOD_CTRL, KEY_MOD_SHIFT,
};
use crate::font::family::{FontFamilies, FontFamily};
use crate::js::JsError;
use crate::number::DeNan;
use crate::paint::Painter;
use crate::render::RenderFn;
use crate::string::StringUtils;
use crate::style::{
    parse_color_str, parse_optional_color_str, FontStyle, PropValueParse, StylePropKey,
};
use crate::text::textbox::{TextBox, TextCoord, TextElement, TextUnit};
use crate::text::{TextAlign, TextDecoration, TextStyle};
use crate::{js_deserialize, js_serialize, ok_or_return, some_or_continue};
use deft_macros::{element_backend, js_methods, mrc_object};
use measure_time::print_time;
use serde::{Deserialize, Serialize};
use skia_safe::font_style::{Slant, Weight, Width};
use skia_safe::wrapper::NativeTransmutableWrapper;
use skia_safe::{Canvas, Color, Font, FontMgr, Paint, Point, Rect};
use std::any::Any;
use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::sync::LazyLock;
use swash::Style;
use winit::keyboard::NamedKey;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};
#[element_backend]
pub struct RichText {
    element: ElementWeak,
    text_box: TextBox,
}

extern "C" fn measure_richtext(
    node_ref: NodeRef,
    width: f32,
    width_mode: MeasureMode,
    height: f32,
    height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(rich_text_weak) = ctx.downcast_ref::<RichTextWeak>() {
            let bounds = Rect::new(0.0, 0.0, width, height);
            if let Ok(mut rich_text) = rich_text_weak.upgrade() {
                rich_text.layout(width);
                return Size {
                    width: rich_text.text_box.max_intrinsic_width(),
                    height: rich_text.text_box.height(),
                };
            }
        }
    }
    Size {
        width: 0.0,
        height: 0.0,
    }
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
    fn create(mut element: &mut Element) -> Self
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
        element
            .style
            .yoga_node
            .set_context(Some(Context::new(this.as_weak())));
        element
            .style
            .yoga_node
            .set_measure_func(Some(measure_richtext));
        this
    }

    fn get_name(&self) -> &str {
        "RichText"
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
