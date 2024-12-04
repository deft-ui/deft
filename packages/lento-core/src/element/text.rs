pub mod skia_text_paragraph;
pub mod text_paragraph;
mod simple_text_paragraph;

use std::any::Any;
use crate as lento;
use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Error;
use quick_js::{JsValue, ValueError};
use skia_safe::{Canvas, Color, Font, FontMgr, FontStyle, Paint, Typeface};
use skia_safe::textlayout::{FontCollection, TextAlign};
use yoga::{Context, MeasureMode, Node, NodeRef, Size};
use lento_macros::{js_methods, mrc_object};
use crate::base::{ElementEvent, EventContext, MouseDetail, MouseEventType, Rect, TextUpdateDetail};
use crate::color::parse_hex_color;
use crate::element::{ElementBackend, Element, ElementWeak};
use crate::element::text::skia_text_paragraph::{SkiaTextParagraph};
use crate::element::text::text_paragraph::{ParagraphData, Line, ParagraphRef, TextParams};
use crate::{js_call, match_event_type};
use crate::event::{FocusShiftEvent, TextUpdateEvent};
use crate::number::DeNan;
use crate::string::StringUtils;
use crate::style::StylePropKey;

// zero-width space for caret
const ZERO_WIDTH_WHITESPACE: &str = "\u{200B}";

pub type AtomOffset = usize;
pub type RowOffset = usize;
pub type ColOffset = usize;


#[repr(C)]
#[mrc_object]
pub struct Text {
    text_params: TextParams,
    selection_paint: Paint,
    paragraph_ref: ParagraphRef,
    last_width: f32,
    /// Option<(start atom offset, end atom offset)>
    selection: Option<(AtomOffset, AtomOffset)>,
    element: Element,
    selecting_begin: Option<AtomOffset>,
}

thread_local! {
    pub static DEFAULT_TYPE_FACE: Typeface = default_typeface();
    pub static FONT_MGR: FontMgr = FontMgr::new();
    pub static FONT_COLLECTION: FontCollection = FontCollection::new();
}

extern "C" fn measure_label(node_ref: NodeRef, width: f32, width_mode: MeasureMode, _height: f32, height_mode: MeasureMode) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(paragraph_props_ptr) = ctx.downcast_ref::<ParagraphRef>() {
            let paragraph = &mut paragraph_props_ptr.data.borrow_mut();
            let p_list = paragraph.get_line(width);
            let mut height = 0f32;
            let mut text_width = 0f32;
            for p in p_list {
                height += p.paragraph.height();
                text_width = text_width.max(p.paragraph.max_intrinsic_width());
            }
            paragraph.measure_mode = Some((width_mode, height_mode));
            // measure_time::print_time!("text len:{}, width:{}, height:{}", paragraph.paragraphs.len(), text_width, height);
            return Size {
                width: text_width,
                height,
            };
        }
    }
    return Size {
        width: 0.0,
        height: 0.0,
    };
}

impl crate::js::FromJsValue for Text {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        let element = Element::from_js_value(value)?;
        Ok(element.get_backend_as::<Text>().clone())
    }
}


#[js_methods]
impl Text {
    fn new(element: Element) -> Self {
        let text_params = TextParams {
            font: DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0)),
            align: TextAlign::Left,
            paint: Paint::default(),
            line_height: None,
        };
        let text = "".to_string();

        let paragraphs = Self::build_lines(&text, &text_params, true);
        let paragraph_props = ParagraphRef {
            data: Rc::new(RefCell::new(ParagraphData {
                lines: paragraphs,
                text_wrap: true,
                measure_mode: None,
            })),
        };
        let mut selection_paint = Paint::default();
        selection_paint.set_color(parse_hex_color("214283").unwrap());
        TextData {
            paragraph_ref: paragraph_props,
            selection_paint,
            selection: None,
            element,
            last_width: 0.0,
            text_params,
            selecting_begin: None,
        }.to_ref()
    }

    pub fn set_text(&mut self, text: String) {
        let old_text = self.get_text();
        if old_text != text {
            self.selection = None;
            self.rebuild_lines(&text);
            self.mark_dirty(false);
            self.mark_layout_dirty_if_needed();

            self.element.emit(TextUpdateEvent {
                value: text
            })
        }
    }


    pub fn insert_text(&mut self, caret: AtomOffset, text: &str) {
        let (caret_row, caret_col) = self.get_location_by_atom_offset(caret);
        let new_text = {
            let mut pi = self.paragraph_ref.data.borrow_mut();
            let p = pi.lines.get(caret_row).unwrap();
            let mut new_text = p.get_text().to_string();
            let insert_pos = new_text.byte_index(caret_col);
            new_text.insert_str(insert_pos, text);
            new_text
        };
        self.rebuild_line(caret_row, new_text);
    }

    #[js_func]
    pub fn get_line_begin_offset(&self, line: RowOffset) -> AtomOffset {
        let pi = self.paragraph_ref.data.borrow();
        let mut offset = 0;
        let mut i = 0;
        for p in &pi.lines {
            if i >= line {
                break;
            }
            offset += p.atom_count;
            i += 1;
        }
        offset
    }

    #[js_func]
    pub fn insert_line(&mut self, line: RowOffset, text: String) {
        let offset = self.get_line_begin_offset(line);
        self.insert_text(offset, &format!("{}\n", text));
    }

    #[js_func]
    pub fn update_line(&mut self, line: RowOffset, text: String) {
        self.rebuild_line(line, text);
    }

    #[js_func]
    pub fn delete_line(&mut self, line: RowOffset) {
        let start = self.get_line_begin_offset(line);
        let end = self.get_line_begin_offset(line + 1);
        self.delete_text(start, end);
    }


    pub fn delete_text(&mut self, begin: AtomOffset, end: AtomOffset) {
        let (begin_row, begin_col) = self.get_location_by_atom_offset(begin);
        let (end_row, end_col) = self.get_location_by_atom_offset(end);
        let new_text = {
            let mut pi = self.paragraph_ref.data.borrow_mut();
            let mut new_text = String::new();
            let begin_p = pi.lines.get_mut(begin_row).unwrap();
            if begin_col > 0 {
                new_text.push_str(&begin_p.subtext(0, begin_col));
            }
            let end_p = pi.lines.get_mut(end_row).unwrap();
            if end_col < end_p.get_text().len() {
                new_text.push_str(&end_p.subtext(end_col, end_p.atom_count));
            }
            new_text
        };
        self.rebuild_line(begin_row, new_text);
        let delete_rows_count = end_row - begin_row;
        let mut pi = self.paragraph_ref.data.borrow_mut();
        for _ in 0..delete_rows_count {
            pi.lines.remove(begin_row + 1);
        }
    }

    fn rebuild_line(&mut self, line: usize, new_text: String) {
        {
            let mut pi = self.paragraph_ref.data.borrow_mut();
            let is_ending = pi.lines.len() - 1 == line;
            let mut ps = Self::build_lines(&new_text, &self.text_params, is_ending);
            pi.lines.remove(line);
            let mut idx = line;
            for p in ps {
                pi.lines.insert(idx, p);
                idx += 1;
            }
        }
        self.mark_dirty(true);
    }

    pub fn select(&mut self, start: usize, end: usize) {
        //TODO validate params
        self.selection = Some((start, end));
        self.mark_dirty(false);
    }

    pub fn unselect(&mut self) {
        self.selection = None;
        self.mark_dirty(false);
    }

    pub fn delete_selected_text(&mut self) {
        if let Some((start, end)) = self.selection {
            self.unselect();
            self.delete_text(start, end);
        }
    }

    #[js_func]
    pub fn set_selection(&mut self, selection: (usize, usize)) {
        //TODO validate params
        let (start, end) = selection;
        if end > start {
            self.select(start, end);
        } else {
            self.unselect();
        }
        self.mark_dirty(false);
    }

    pub fn get_selection(&self) -> Option<(usize, usize)> {
        self.selection
    }

    pub fn get_selection_text(&self) -> Option<(String)> {
        self.get_selection_data().map(|(text, _, _)| text)
    }

    pub fn get_selection_data(&self) -> Option<(String, AtomOffset, AtomOffset)> {
        if let Some((start, end)) = self.get_selection() {
            let mut result = String::new();
            self.with_lines_mut(|lines| {
                let mut line_offset = 0;
                for p in lines {
                    if let Some((s,e)) = intersect_range(
                        (line_offset, line_offset + p.atom_count),
                        (start, end)
                    ) {
                        result.push_str(p.subtext(s - line_offset, e - line_offset));
                    }
                    line_offset += p.atom_count;
                    if line_offset >= end {
                        break;
                    }
                }
            });
            Some((result, start, end))
        } else {
            None
        }
    }

    pub fn get_text(&self) -> String {
        self.with_lines_mut(|ps| {
            let mut text = String::new();
            for p in ps {
                text.push_str(&p.get_text())
            }
            text
        })
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.text_params.font.set_size(size);
        self.refresh_lines();
        self.mark_dirty(true);
    }

    pub fn get_font(&self) -> &Font {
        &self.text_params.font
    }

    pub fn set_align(&mut self, align: TextAlign) {
        self.text_params.align = align;
        self.refresh_lines();
        self.mark_dirty(false);
    }

    pub fn get_align(&self) -> TextAlign {
        self.text_params.align
    }

    pub fn get_color(&self) -> Color {
        self.text_params.paint.color()
    }

    pub fn rebuild_lines(&mut self, text: &str) {
        let paragraphs = Self::build_lines(text, &self.text_params, true);
        let mut pi = self.paragraph_ref.data.borrow_mut();
        pi.update_line(paragraphs);
    }

    pub fn refresh_lines(&mut self) {
        let text = self.get_text();
        self.rebuild_lines(&text);
    }

    pub fn get_paint(&self) -> &Paint {
        &self.text_params.paint
    }

    #[js_func]
    pub fn set_text_wrap(&mut self, text_wrap: bool) {
        {
            let mut p = self.paragraph_ref.data.borrow_mut();
            p.text_wrap = text_wrap;
        }
        self.mark_dirty(true);
    }

    pub fn get_caret_at_offset_coordinate(&self, offset: (f32, f32)) -> (RowOffset, ColOffset) {
        let (offset_x, offset_y) = offset;
        let (padding_top, _, _, padding_left) = self.element.get_padding();
        let expected_offset = (offset_x - padding_left, offset_y - padding_top);
        self.with_lines_mut(|p_list| {
            let mut paragraph_offset = 0;
            let mut height = 0f32;
            let max_offset = p_list.len() - 1;
            for p in p_list {
                height += p.paragraph.height();
                if paragraph_offset == max_offset || height > expected_offset.1 {
                    let line_pos = (expected_offset.0, expected_offset.1 - (height - p.paragraph.height()));
                    let line_col = p.get_caret_by_coord(line_pos);
                    return (paragraph_offset, line_col);
                }
                paragraph_offset += 1;
            }
            (0, 0)
        })
    }

    pub fn get_caret_offset_coordinate(&self, char_offset: usize) -> ((f32, f32), (f32, f32)) {
        let (caret_row, caret_col) = self.get_location_by_atom_offset(char_offset);
        let (padding_top, _, _, padding_left) = self.element.get_padding();
        let caret_height = self.get_font().size();
        self.with_lines_mut(|p_list| {
            let mut y_offset = 0f32;
            let mut current_row = 0;
            for p in p_list {
                if current_row == caret_row {
                    let gc = p.paragraph.get_char_bounds(caret_col).unwrap();
                    let right = gc.left + padding_left;
                    let middle = (gc.top + gc.bottom) / 2.0 + padding_top + y_offset;
                    return ((right, middle - caret_height / 2.0), (right, middle + caret_height / 2.0));
                }
                y_offset += p.paragraph.height();
                current_row += 1;
            }
            unreachable!()
        })
    }

    pub fn get_location_by_atom_offset(&self, atom_offset: AtomOffset) -> (RowOffset, ColOffset) {
        self.with_lines_mut(|ps| {
            let mut line_atom_offset = 0;
            let mut row = 0;
            for p in ps {
                let line_atom_end_offset = line_atom_offset + p.atom_count;
                if line_atom_end_offset > atom_offset {
                    let col = p.get_column_by_atom_offset(atom_offset - line_atom_offset);
                    return Some((row, col));
                }
                line_atom_offset = line_atom_end_offset;
                row += 1;
            }
            None
        }).unwrap_or(self.get_max_caret())
    }

    #[js_func]
    pub fn get_atom_offset_by_location(&self, location: (RowOffset, ColOffset)) -> AtomOffset {
        self.with_lines_mut(|ps| {
            let (caret_row, caret_col) = location;
            let mut row = 0;
            let mut atom_offset = 0;
            for p in ps {
                let line_atom_count = p.atom_count;
                if row == caret_row {
                    let col = usize::min(caret_col, p.atom_count - 1);
                    return atom_offset + col;
                }
                row += 1;
                atom_offset += line_atom_count;
            }
            atom_offset
        })
    }

    fn begin_select(&mut self, caret: AtomOffset) {
        self.element.emit(FocusShiftEvent);
        self.unselect();
        self.selecting_begin = Some(caret);
    }

    fn end_select(&mut self) {
        self.selecting_begin = None;
    }

    pub fn get_atom_offset_by_coordinate(&self, position: (f32, f32)) -> AtomOffset {
        let (row, col) = self.get_caret_at_offset_coordinate(position);
        self.get_atom_offset_by_location((row, col))
    }

    fn handle_mouse_event(&mut self, event: &MouseDetail) {
        match event.event_type {
            MouseEventType::MouseDown => {
                let caret = self.get_atom_offset_by_coordinate((event.offset_x, event.offset_y));
                self.begin_select(caret);
            }
            MouseEventType::MouseMove => {
                if self.selecting_begin.is_some() {
                    let caret = self.get_atom_offset_by_coordinate((event.offset_x, event.offset_y));
                    if let Some(sb) = &self.selecting_begin {
                        let start = AtomOffset::min(*sb, caret);
                        let end = AtomOffset::max(*sb, caret);
                        self.select(start, end);
                    }
                }
            }
            MouseEventType::MouseUp => {
                self.end_select();
            }
            _ => {},
        }
    }

    pub fn with_lines_mut<R, F: FnOnce(&mut Vec<Line>) -> R>(&self, callback: F) -> R {
        let layout = &self.element.layout;
        let content_width = layout.get_layout_width()
            - layout.get_layout_padding_left().de_nan(0.0)
            - layout.get_layout_padding_right().de_nan(0.0);

        let mut pi = self.paragraph_ref.data.borrow_mut();
        let p = pi.get_line(content_width);
        callback(p)
    }

    pub fn get_atom_count(&self) -> AtomOffset {
        self.with_lines_mut(|ps| {
            let mut result = 0;
            for p in ps {
                result += p.atom_count;
            }
            result
        })
    }

    pub fn get_max_caret(&self) -> (RowOffset, ColOffset) {
        self.with_lines_mut(|ps| {
            let max_row = ps.len() - 1;
            let max_col = unsafe { ps.get_unchecked(max_row).atom_count - 1 };
            (max_row, max_col)
        })
    }

    pub fn get_line_height(&self) -> Option<f32> {
        self.text_params.line_height
    }

    pub fn get_computed_line_height(&self) -> f32 {
        match &self.text_params.line_height {
            None => self.get_font().size(),
            Some(line_height) => *line_height,
        }
    }

    pub fn build_lines(text: &str, params: &TextParams, is_ending: bool) -> Vec<Line> {
        let mut lines: Vec<&str> = if text.is_empty() {
            vec![""]
        } else {
            text.split_inclusive('\n').into_iter().collect()
        };
        if is_ending && text.ends_with('\n') {
            lines.push("");
        }
        let mut result = Vec::new();
        for ln in lines {
            // let p = SimpleTextParagraph::new(ln, params);
            let ln = Self::preprocess_text(ln);
            let p = SkiaTextParagraph::new(ln.to_string(), params);
            result.push(Line {
                atom_count: ln.trim_line_endings().chars().count() + 1,
                paragraph: p,
                paragraph_dirty: true,
            })
        }
        result
    }

    fn mark_dirty(&mut self, layout_dirty: bool) {
        self.element.mark_dirty(layout_dirty);
    }

    fn mark_layout_dirty_if_needed(&mut self) {
        let d = self.paragraph_ref.data.borrow_mut().measure_mode;
        match d {
            None => return,
            Some((width_mode, height_mode)) => {
                if width_mode == MeasureMode::AtMost {
                    self.mark_dirty(true);
                } else if height_mode == MeasureMode::AtMost {
                    //TODO use cached value?
                    let old_height = self.element.get_size().1;
                    let new_height = self.with_lines_mut(|ps| {
                        let mut height = 0f32;
                        for p in ps {
                            height += p.paragraph.height();
                        }
                        height
                    });
                    if new_height != old_height {
                        self.mark_dirty(true);
                    }
                }
            }
        }
    }

    fn preprocess_text(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

}


fn default_typeface() -> Typeface {
    let font_mgr = FontMgr::new();
    font_mgr.legacy_make_typeface(None, FontStyle::default()).unwrap()
}

pub fn intersect_range<T: Ord>(range1: (T, T), range2: (T, T)) -> Option<(T, T)> {
    let start = T::max(range1.0, range2.0);
    let end = T::min(range1.1, range2.1);
    if end > start {
        Some((start, end))
    } else {
        None
    }
}

impl ElementBackend for Text {
    fn create(mut ele: Element) -> Self {
        let mut label = Self::new(ele.clone());
        ele.layout.set_context(Some(Context::new(label.paragraph_ref.clone())));
        ele.layout.set_measure_func(Some(measure_label));
        label
    }

    fn get_name(&self) -> &str {
        "Text"
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        match key {
            StylePropKey::Color => {
                let color = self.element.layout.computed_style.color;
                self.text_params.paint.set_color(color);
                self.refresh_lines();
                self.mark_dirty(false);
            },
            StylePropKey::FontSize => {
                let font_size = self.element.layout.font_size;
                self.text_params.font.set_size(font_size);
                self.refresh_lines();
                self.mark_dirty(true);
            }
            _ => {}
        }
    }

    fn draw(&self, canvas: &Canvas) {
        let clip_rect = canvas.local_clip_bounds();
        // if let Some(clip_r) = canvas.local_clip_bounds() {
        //     println!("clip_r:{:?}", clip_r);
        //     let mut paint = Paint::default();
        //     paint.set_color(parse_hex_color("ccc").unwrap());
        //     canvas.draw_rect(clip_r, &paint);
        // }
        self.with_lines_mut(|p_list| {
            let mut top = 0.0;
            let mut line_atom_offset = 0;
            for p in p_list {
                let p_height = p.paragraph.height();
                let p_top = top;
                let p_bottom = top + p_height;
                let p_atom_begin = line_atom_offset;
                let p_atom_count = p.atom_count;
                let p_atom_end = p_atom_begin + p_atom_count;

                top += p_height;
                line_atom_offset += p_atom_count;
                if let Some(cp) = clip_rect {
                    if p_bottom < cp.top {
                        continue;
                    } else if p_top > cp.bottom {
                        break;
                    }
                }
                if let Some(si_range) = self.selection {
                    let p_range = (p_atom_begin, p_atom_end);
                    if let Some((begin, end)) = intersect_range(si_range, p_range) {
                        let begin = begin - p_atom_begin;
                        let end = end - p_atom_begin;
                        for offset in begin..end {
                            if let Some(g) = p.paragraph.get_char_bounds(offset) {
                                let bounds = g.with_offset((0.0, p_top));
                                canvas.draw_rect(&bounds, &self.selection_paint);
                            }
                        }
                    }
                }
                p.paragraph.paint(canvas, (0.0, p_top));
            }
        });
    }

    fn set_property(&mut self, p: &str, v: JsValue) {
        js_call!("text", String, self, set_text, p, v);
        js_call!("fontsize", f32, self, set_font_size, p, v);
        js_call!("align", TextAlign, self, set_align, p, v);
        js_call!("selection", (usize, usize), self, set_selection, p, v);
    }

    fn get_property(&mut self, property_name: &str) -> Result<Option<JsValue>, Error> {
        match property_name {
            "text" => Ok(Some(JsValue::String(self.get_text().to_string()))),
            _ => {
                Ok(None)
            }
        }
    }

    fn execute_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
        if let Some(d) = event.downcast_ref::<FocusShiftEvent>() {
            self.unselect();
        }
        false
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        //TODO check font/color changed?
        let last_width = if bounds.width != self.last_width {
            let mut pi = self.paragraph_ref.data.borrow_mut();
            pi.lines.iter_mut().for_each(|p| {
                p.paragraph_dirty = true;
            });
            Some(bounds.width)
        } else {
            None
        };
        if let Some(last_width) = last_width {
            self.last_width = last_width;
        }
    }

    fn handle_event(&mut self, _event_type: &str, event: &mut ElementEvent) {
        match_event_type!(event, MouseDetail, self, handle_mouse_event);
    }
}

pub fn parse_align(align: &str) -> TextAlign {
    match align {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        _ => TextAlign::Left,
    }
}

#[test]
pub fn test_get_caret_at_offset_coordinate() {
    let mut el = Element::create(Text::create);
    let text = el.get_backend_mut_as::<Text>();
    let (row, col) = text.get_caret_at_offset_coordinate((100.0, 100.0));
    assert_eq!(0, row);
    assert_eq!(0, col);
    let _pos = text.get_caret_offset_coordinate(0);
}

#[test]
pub fn test_get_caret_by_char_offset() {
    let mut el = Element::create(Text::create);
    let text = el.get_backend_mut_as::<Text>();
    text.set_text("abc".to_string());
    assert_eq!((0, 2), text.get_location_by_atom_offset(2));
    assert_eq!((0, 3), text.get_location_by_atom_offset(3));
}