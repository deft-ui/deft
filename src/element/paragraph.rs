pub mod typeface_mgr;
pub mod simple_paragraph_builder;

use std::any::Any;
use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use crate as deft;
use crate::color::parse_hex_color;
use crate::element::text::text_paragraph::ParagraphRef;
use crate::element::text::{intersect_range, ColOffset, RowOffset, FONT_COLLECTION, FONT_MGR};
use crate::element::{text, Element, ElementBackend, ElementWeak};
use crate::js::JsError;
use crate::number::DeNan;
use crate::string::StringUtils;
use crate::style::{parse_color_str, parse_optional_color_str, StylePropKey};
use crate::{js_deserialize, js_serialize};
use deft_macros::{element_backend, js_methods, mrc_object};
use rodio::cpal::available_hosts;
use serde::{Deserialize, Serialize};
use skia_safe::font_style::{Slant, Weight, Width};
use skia_safe::textlayout::{
    Decoration, FontFamilies, Paragraph as SkParagraph, ParagraphBuilder, ParagraphStyle,
    PlaceholderStyle, StrutStyle, TextAlign, TextDecoration, TextDirection, TextStyle,
    TypefaceFontProvider,
};
use skia_safe::{Canvas, Color, Font, FontMgr, FontStyle, Paint, Point, Rect};
use std::str::FromStr;
use clipboard::{ClipboardContext, ClipboardProvider};
use measure_time::print_time;
use skia_safe::wrapper::NativeTransmutableWrapper;
use winit::keyboard::NamedKey;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};
use crate::base::{EventContext, MouseDetail, MouseEventType};
use crate::element::paragraph::simple_paragraph_builder::SimpleParagraphBuilder;
use crate::element::text::simple_text_paragraph::SimpleTextParagraph;
use crate::event::{FocusShiftEvent, KeyDownEvent, KeyEventDetail, MouseDownEvent, MouseMoveEvent, MouseUpEvent, SelectEndEvent, SelectMoveEvent, SelectStartEvent, TouchEndEvent, TouchMoveEvent, TouchStartEvent, KEY_MOD_CTRL, KEY_MOD_SHIFT};
use crate::render::RenderFn;
use crate::typeface::get_font_mgr;

const DEFAULT_FONT_NAME: &str = "system-ui";

const ZERO_WIDTH_WHITESPACE: &str = "\u{200B}";

#[derive(Clone)]
pub struct ParagraphParams {
    pub mask_char: Option<char>,
    pub text_wrap: Option<bool>,
    pub line_height: Option<f32>,
    pub align: TextAlign,
    pub color: Color,
    pub font_size: f32,
    pub font_families: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParagraphUnit {
    Text(TextUnit),
}

js_serialize!(ParagraphUnit);
js_deserialize!(ParagraphUnit);

impl ParagraphUnit {
    fn atom_count(&self) -> usize {
        match self {
            ParagraphUnit::Text(text) => {
                text.text.chars_count()
            }
        }
    }
    fn text(&self) -> &str {
        match self {
            ParagraphUnit::Text(t) => {
                t.text.as_str()
            }
        }
    }

    fn get_text(&self, begin: usize, end: usize) -> &str {
        match self {
            ParagraphUnit::Text(t) => {
                t.text.substring(begin, end - begin)
            }
        }
    }

}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextUnit {
    pub text: String,
    pub font_families: Option<Vec<String>>,
    pub font_size: Option<f32>,
    pub color: Option<String>,
    pub text_decoration_line: Option<String>,
    pub weight: Option<String>,
    pub background_color: Option<String>,
}

js_serialize!(TextUnit);
js_deserialize!(TextUnit);

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TextCoord(pub usize, pub usize);

impl TextCoord {
    pub fn new(v: (usize, usize)) -> TextCoord {
        TextCoord(v.0, v.1)
    }
}

js_serialize!(TextCoord);

#[element_backend]
pub struct Paragraph {
    element: Element,
    params: ParagraphParams,
    lines: Vec<Line>,
    /// Option<(start coord, end coord)>
    selection: Option<(TextCoord, TextCoord)>,
    selecting_begin: Option<TextCoord>,
    selection_bg: Paint,
    selection_fg: Paint,
}

#[derive(Clone)]
pub struct Line {
    units: Vec<ParagraphUnit>,
    sk_paragraph: SimpleTextParagraph,
    layout_calculated: bool,
}

impl Line {
    pub fn new(units: Vec<ParagraphUnit>, paragraph_params: &ParagraphParams) -> Self {
        let sk_paragraph = Paragraph::build_paragraph(paragraph_params, &units);
        Self {
            layout_calculated: false,
            units,
            sk_paragraph,
        }
    }

    pub fn atom_count(&self) -> usize {
        let mut count = 0;
        for u in &self.units {
            count += u.atom_count();
        }
        count
    }

    pub fn get_text(&self) -> String {
        let mut result = String::new();
        for u in &self.units {
            result.push_str(u.text());
        }
        result
    }

    pub fn subtext(&self, mut start: ColOffset, mut end: ColOffset) -> String {
        let mut result = String::new();
        let mut iter = self.units.iter();
        let mut processed_atom_count = 0;
        loop {
            let u = match iter.next() {
                Some(u) => u,
                None => break,
            };
            let unit_atom_count = u.atom_count();
            if let Some(intersect) = intersect_range((start, end), (processed_atom_count, unit_atom_count + processed_atom_count)) {
                result.push_str(u.get_text(intersect.0 - processed_atom_count, intersect.1 - processed_atom_count));
            }
            processed_atom_count += unit_atom_count;
            if processed_atom_count >= end {
                break;
            }
        }
        result.to_string()
    }

    pub fn get_column_by_pixel_coord(&self, coord: (f32, f32)) -> usize {
        let (x, y) = coord;
        let atom_count = self.atom_count();
        if atom_count == 0 {
            0
        } else if x > self.sk_paragraph.max_intrinsic_width() {
            atom_count
        } else {
            self.sk_paragraph.get_char_offset_at_coordinate(coord)
        }
    }

    pub fn get_char_bounds(&mut self, char_offset: usize) -> Option<Rect> {
        self.sk_paragraph.get_char_bounds(char_offset)
    }

    pub fn get_utf8_offset(&self, char_offset: usize) -> usize {
        if char_offset == 0 {
            0
        } else {
            self.get_text().substring(0, char_offset).len()
        }
    }

    fn paint_selection(
        &mut self,
        canvas: &Canvas,
        line_offset: (f32,f32),
        selection: (usize, usize),
        bg_paint: &Paint,
        fg_paint: &Paint,
    ) {
        let (start_offset, end_offset) = selection;
        let line_offset = Point { x: line_offset.0, y: line_offset.1 };
        canvas.save();
        canvas.translate(line_offset);
        for i in start_offset..end_offset {
            if let Some(char_rect) = self.get_char_bounds(i) {
                canvas.draw_rect(&char_rect, &bg_paint);
            }
        }

        self.sk_paragraph.paint_chars(canvas, start_offset, end_offset, Some(fg_paint));
        canvas.restore();
    }

    fn rebuild_paragraph(&mut self, paragraph_params: &ParagraphParams) {
        self.sk_paragraph = Paragraph::build_paragraph(paragraph_params, &self.units);
        self.layout_calculated = false;
    }

    fn force_layout(&mut self, available_width: f32) {
        self.layout_calculated = true;
        self.sk_paragraph.layout(available_width);
    }

    fn layout(&mut self, available_width: Option<f32>, element_width: f32) {
        if let Some(w) = available_width {
            self.force_layout(w);
        } else if (!self.layout_calculated) {
            self.force_layout(element_width);
        }
    }

}

extern "C" fn measure_paragraph(
    node_ref: NodeRef,
    width: f32,
    width_mode: MeasureMode,
    _height: f32,
    height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(paragraph) = ctx.downcast_ref::<ParagraphWeak>() {
            if let Ok(mut p) = paragraph.upgrade() {
                p.layout(Some(width));
                return Size {
                    width: p.max_intrinsic_width(),
                    height: p.height(),
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
impl Paragraph {
    #[js_func]
    pub fn new_element() -> Element {
        Element::create(Paragraph::create)
    }

    #[js_func]
    pub fn add_line(&mut self, units: Vec<ParagraphUnit>) {
        let line = Line::new(units, &self.params);
        self.lines.push(line);
        self.mark_dirty();
    }

    #[js_func]
    pub fn insert_line(&mut self, index: usize, units: Vec<ParagraphUnit>) {
        let line = Line::new(units, &self.params);
        self.lines.insert(index, line);
        self.mark_dirty();
    }

    #[js_func]
    pub fn delete_line(&mut self, line: usize) {
        self.lines.remove(line);
        self.mark_dirty();
    }

    #[js_func]
    pub fn update_line(&mut self, index: usize, units: Vec<ParagraphUnit>) {
        self.lines[index] = Line::new(units, &self.params);
        self.mark_dirty();
    }

    #[js_func]
    pub fn clear(&mut self) {
        self.lines.clear();
        self.mark_dirty();
    }

    #[js_func]
    pub fn measure_line(&self, units: Vec<ParagraphUnit>) -> (f32, f32) {
        let mut sk_paragraph = Self::build_paragraph(&self.params, &units);
        sk_paragraph.layout(f32::NAN);
        (sk_paragraph.max_intrinsic_width(), sk_paragraph.height())
    }

    pub fn set_text_wrap(&mut self, wrap: bool) {
        self.params.text_wrap = Some(wrap);
        self.element.mark_dirty(true);
    }

    fn layout(&mut self, mut available_width: Option<f32>) {
        let mut layout_width = f32::NAN;
        if self.params.text_wrap.unwrap_or(false) {
            layout_width = self.element.style.get_content_bounds().width;
        } else {
            available_width = available_width.map(|_| f32::NAN);
        }
        for ln in &mut self.lines {
            ln.layout(available_width, layout_width);
        }
    }

    pub fn height(&self) -> f32 {
        let mut height = 0.0;
        for ln in &self.lines {
            height += ln.sk_paragraph.height().de_nan(0.0);
        }
        height
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        let mut max_width = 0.0;
        for ln in &self.lines {
            max_width = f32::max(max_width, ln.sk_paragraph.max_intrinsic_width());
        }
        max_width
    }

    pub fn get_atom_count(&self) -> usize {
        let mut count = 0;
        for ln in &self.lines {
            count += ln.atom_count();
        }
        count
    }

    pub fn get_text(&self) -> String {
        let mut text = String::new();
        let lines_count = self.lines.len();
        if lines_count > 1 {
            for i in 0..lines_count - 1 {
                let line = unsafe { self.lines.get_unchecked(i) };
                text.push_str(line.get_text().as_str());
                text.push_str("\n");
            }
        }
        if lines_count > 0 {
            text.push_str(self.lines.last().unwrap().get_text().as_str());
        }
        text
    }

    pub fn get_soft_line_height(&self, row: usize, col: usize) -> Option<f32> {
        let line = self.lines.get(row)?;
        let ln = line.sk_paragraph.get_line_number_at_utf16_offset(col)?;
        let lm = line.sk_paragraph.get_line_height_at(ln).unwrap();
        Some(lm)
    }

    pub fn get_lines(&self) -> &Vec<Line> {
        &self.lines
    }

    pub fn select(&mut self, start: TextCoord, end: TextCoord) {
        //TODO validate params
        self.selection = Some((start, end));
        self.element.mark_dirty(false);
    }

    pub fn is_selecting(&self) -> bool {
        self.selecting_begin.is_some()
    }

    pub fn unselect(&mut self) {
        self.selection = None;
        self.element.mark_dirty(false);
    }

    fn begin_select(&mut self, caret: TextCoord) {
        self.element.emit(FocusShiftEvent);
        self.unselect();
        self.selecting_begin = Some(caret);
        self.element.emit(SelectStartEvent {
            row: caret.0,
            col: caret.1,
        });
    }

    fn selection_start(&mut self, point: (f32, f32)) {
        let begin_coord = self.get_text_coord_by_pixel_coord(point);
        self.begin_select(begin_coord);
    }

    fn selection_update(&mut self, point: (f32, f32)) -> bool {
        if self.selecting_begin.is_some() {
            let caret = self.get_text_coord_by_pixel_coord(point);
            if let Some(sb) = self.selecting_begin {
                self.element.emit(SelectMoveEvent{
                    row: caret.0,
                    col: caret.1,
                });
                let start = TextCoord::min(sb, caret);
                let end = TextCoord::max(sb, caret);
                self.select(start, end);
                return true;
            }
        }
        false
    }

    fn selection_end(&mut self) -> bool {
        if self.selecting_begin.is_some() {
            self.end_select();
            self.element.emit(SelectEndEvent);
            return true;
        }
        false
    }

    fn end_select(&mut self) {
        self.selecting_begin = None;
    }

    fn handle_mouse_event(&mut self, event: &MouseDetail) {
        match event.event_type {
            MouseEventType::MouseDown => {
                let begin_coord = self.get_text_coord_by_pixel_coord((event.offset_x, event.offset_y));
                self.begin_select(begin_coord);
            }
            MouseEventType::MouseMove => {
                if self.selecting_begin.is_some() {
                    let caret = self.get_text_coord_by_pixel_coord((event.offset_x, event.offset_y));
                    if let Some(sb) = self.selecting_begin {
                        let start = TextCoord::min(sb, caret);
                        let end = TextCoord::max(sb, caret);
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

    pub fn get_text_coord_by_pixel_coord(&self, coord: (f32, f32)) -> TextCoord {
        let (offset_x, offset_y) = coord;
        let (padding_top, _, _, padding_left) = self.element.get_padding();
        let expected_offset = (offset_x - padding_left, offset_y - padding_top);
        let mut row = 0;
        let mut height = 0f32;

        let lines = &self.lines;
        let max_offset = if lines.is_empty() { 0 } else { lines.len() - 1 };
        for p in lines {
            height += p.sk_paragraph.height();
            if row == max_offset || height > expected_offset.1 {
                let line_pixel_coord = (expected_offset.0, expected_offset.1 - (height - p.sk_paragraph.height()));
                let line_column = p.get_column_by_pixel_coord(line_pixel_coord);
                return TextCoord(row, line_column);
            }
            row += 1;
        }
        TextCoord(0, 0)
    }

    #[js_func]
    pub fn get_text_coord_by_char_offset(&self, caret: usize) -> Option<TextCoord> {
        let mut col = caret;
        let mut row = 0;
        for ln in &self.lines {
            if col <= ln.atom_count() {
                return Some(TextCoord(row, col));
            }
            row += 1;
            col -= ln.atom_count() + 1;
        }
        None
    }

    pub fn get_char_rect(&mut self, coord: TextCoord) -> Option<crate::base::Rect> {
        let (row, col) = (coord.0, coord.1);
        let line = self.lines.get_mut(row)?;
        let bounds = line.sk_paragraph.get_char_bounds(col)?;
        let mut y_offset = 0.0;
        if row > 0 {
            for i in 0..row {
                y_offset += unsafe {
                    self.lines.get_unchecked(i).sk_paragraph.height()
                }
            }
        }
        Some(crate::base::Rect::new(bounds.left, y_offset + bounds.top, bounds.width(), bounds.height()))
    }

    fn handle_key_down(&mut self, event: &KeyEventDetail) -> bool {
        if event.modifiers == KEY_MOD_CTRL {
            if let Some(text) = &event.key_str {
                match text.as_str() {
                    "c" => {
                        if let Some(sel) = self.get_selection_text() {
                            let sel=  sel.to_string();
                            if let Ok(mut ctx) = ClipboardContext::new() {
                                ctx.set_contents(sel);
                            }
                        }
                        return true
                    },
                    _ => {}
                }
            }
        }
        false
    }

    pub fn get_line_text(&self, row: usize) -> Option<String> {
        Some(self.lines.get(row)?.get_text())
    }

    pub fn get_selection(&self) -> Option<(TextCoord, TextCoord)> {
        self.selection
    }

    #[js_func]
    pub fn get_selection_text(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let start = selection.0;
        let end = selection.1;
        let start_line = self.lines.get(start.0)?;
        let end_line = self.lines.get(end.0)?;
        let text = if start.0 == end.0 {
            start_line.subtext(start.1, end.1)
        } else {
            let mut result =  start_line.subtext(start.1, start_line.atom_count());
            if end.0 - start.0 > 1 {
                for i in start.0 + 1..end.0 {
                    let ln = self.lines.get(i)?;
                    result.push_str("\n");
                    result.push_str(&ln.get_text())
                }
            }
            result.push_str("\n");
            result.push_str(&end_line.subtext(0, end.1));
            result
        };
        Some(text)
    }

    pub fn set_mask_char(&mut self, mask_char: Option<char>) {
        self.params.mask_char = mask_char;
        self.mark_dirty();
    }

    fn rebuild_paragraph(&mut self) {
        let params = self.params.clone();
        for ln in &mut self.lines {
            ln.rebuild_paragraph(&params);
        }
    }

    fn mark_dirty(&mut self) {
        self.element.mark_dirty(true);
    }

    pub fn build_paragraph(
        paragraph_params: &ParagraphParams,
        units: &Vec<ParagraphUnit>,
    ) -> SimpleTextParagraph {
        // let mut text = text.trim_line_endings().to_string();
        // text.push_str(ZERO_WIDTH_WHITESPACE);
        let mut paragraph_style = ParagraphStyle::new();
        paragraph_style.set_text_align(paragraph_params.align);

        let default_font_families:Vec<&str> = DEFAULT_FONT_NAME.split(",").collect();
        if let Some(line_height) = paragraph_params.line_height {
            let mut strut_style = StrutStyle::default();
            strut_style.set_font_families(default_font_families.as_slice());
            strut_style.set_strut_enabled(true);
            strut_style.set_font_size(line_height);
            strut_style.set_force_strut_height(true);
            paragraph_style.set_strut_style(strut_style);
        }

        let mut pb = SimpleParagraphBuilder::new(&paragraph_params);
        let p_color = paragraph_params.color;
        let mask_char = paragraph_params.mask_char;
        for u in units {
            match u {
                ParagraphUnit::Text(unit) => {
                    let mut text_style = TextStyle::new();
                    let font_families = unit
                        .font_families
                        .as_ref()
                        .unwrap_or(&paragraph_params.font_families);
                    let font_families = if font_families.is_empty() {
                        &paragraph_params.font_families
                    } else {
                        &font_families
                    };
                    let font_size = unit.font_size.unwrap_or(paragraph_params.font_size);
                    if !font_families.is_empty() {
                        text_style.set_font_families(&font_families);
                    }
                    text_style.set_font_size(font_size);

                    let weight =
                        parse_optional_weight(unit.weight.as_ref()).unwrap_or(Weight::NORMAL);
                    let font_style = FontStyle::new(weight, Width::NORMAL, Slant::Upright);
                    text_style.set_font_style(font_style);

                    let decoration =
                        parse_optional_text_decoration(unit.text_decoration_line.as_ref());
                    text_style.set_decoration_type(decoration);

                    let color = parse_optional_color_str(unit.color.as_ref()).unwrap_or(p_color);
                    let mut paint = Paint::default();
                    paint.set_color(color);
                    text_style.set_foreground_paint(&paint);

                    if let Some(bg) = parse_optional_color_str(unit.background_color.as_ref()) {
                        let mut bg_paint = Paint::default();
                        bg_paint.set_color(bg);
                        text_style.set_background_paint(&bg_paint);
                    }

                    pb.push_style(&text_style);
                    if let Some(mc) = mask_char {
                        let mask_str = mc.to_string().repeat(unit.text.chars_count());
                        pb.add_text(&mask_str);
                    } else {
                        pb.add_text(&unit.text);
                    }
                }
            }
        }
        pb.add_text(ZERO_WIDTH_WHITESPACE);

        pb.build()
    }
}

impl ElementBackend for Paragraph {
    fn create(mut element: &mut Element) -> Self
    where
        Self: Sized,
    {
        let font_families:Vec<String> = DEFAULT_FONT_NAME.split(",").map(|i| i.to_string()).collect();

        let params = ParagraphParams {
            text_wrap: Some(true),
            line_height: None,
            align: TextAlign::Left,
            color: Color::default(),
            font_size: 12.0,
            font_families,
            mask_char: None,
        };
        let units = Vec::new();
        let paragraph = Self::build_paragraph(&params, &units);

        let mut selection_bg = Paint::default();
        selection_bg.set_color(parse_hex_color("214283").unwrap());
        let mut selection_fg = Paint::default();
        selection_fg.set_color(Color::from_rgb(255, 255, 255));
        let this = ParagraphData {
            lines: Vec::new(),
            element: element.clone(),
            params,
            selection: None,
            selecting_begin: None,
            selection_bg,
            selection_fg,
        }
        .to_ref();
        element
            .style
            .yoga_node.set_context(Some(Context::new(this.as_weak())));
        element.style.yoga_node.set_measure_func(Some(measure_paragraph));
        this
    }

    fn get_name(&self) -> &str {
        "Paragraph"
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let mut rebuild = true;
        match key {
            StylePropKey::Color => {
                self.params.color = self.element.style.computed_style.color;
            }
            StylePropKey::FontSize => {
                self.params.font_size = self.element.style.computed_style.font_size;
            }
            StylePropKey::LineHeight => {
                self.params.line_height = Some(self.element.style.computed_style.line_height);
            }
            _ => {
                rebuild = false;
            }
        }
        if rebuild {
            self.rebuild_paragraph();
        }
    }

    fn render(&mut self) -> RenderFn {
        let padding = self.element.get_padding();
        let mut p = self.clone();
        p.layout(None);

        let mut me = self.clone();
        let mut consumed_top = 0.0;
        let mut consumed_rows = 0usize;
        let mut consumed_columns = 0usize;
        let mut lines = me.lines.clone();
        let selection = self.selection;
        let selection_bg = self.selection_bg.clone();
        let selection_fg = self.selection_fg.clone();

        RenderFn::new(move |canvas| {
            canvas.translate((padding.3, padding.0));
            let clip_rect = canvas.local_clip_bounds();
            for ln in &mut lines {
                let ln_row = consumed_rows; consumed_rows += 1;
                let ln_column = consumed_columns; consumed_columns += 1;

                let ln_height = ln.sk_paragraph.height();
                let ln_top = consumed_top; consumed_top += ln_height;
                let ln_bottom = consumed_top;


                if let Some(cp) = clip_rect {
                    if ln_bottom < cp.top {
                        continue;
                    } else if ln_top > cp.bottom {
                        break;
                    }
                }
                ln.sk_paragraph.paint(canvas, (0.0, ln_top));

                let atom_count = ln.atom_count();
                if atom_count > 0 {
                    if let Some(selection_range) = selection {
                        let ln_range = (TextCoord(ln_row, 0), TextCoord(ln_row, atom_count));
                        if let Some((begin, end)) = intersect_range(selection_range, ln_range) {
                            ln.paint_selection(
                                canvas,
                                (0.0, ln_top),
                                (begin.1, end.1),
                                &selection_bg,
                                &selection_fg
                            );
                        }
                    }
                }
            }
        })
    }

    fn execute_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
        if let Some(d) = event.downcast_ref::<KeyDownEvent>() {
            self.handle_key_down(&d.0);
        } else {
            if let Some(e) = event.downcast_ref::<MouseDownEvent>() {
                if e.0.button == 1 {
                    let event = e.0;
                    self.selection_start((event.offset_x, event.offset_y));
                    return true;
                }
            } else if let Some(e) = event.downcast_ref::<MouseMoveEvent>() {
                let event = e.0;
                return self.selection_update((event.offset_x, event.offset_y))
            } else if let Some(e) = event.downcast_ref::<MouseUpEvent>() {
                if e.0.button == 1 {
                    return self.selection_end();
                }
            }
        }
        return false;
    }

}

pub fn parse_optional_weight(value: Option<&String>) -> Option<Weight> {
    if let Some(v) = value {
        parse_weight(v)
    } else {
        None
    }
}
pub fn parse_weight(value: &str) -> Option<Weight> {
    let w = match value.to_lowercase().as_str() {
        "invisible" => Weight::INVISIBLE,
        "thin" => Weight::THIN,
        "extra-light" => Weight::EXTRA_LIGHT,
        "light" => Weight::LIGHT,
        "normal" => Weight::NORMAL,
        "medium" => Weight::MEDIUM,
        "semi-bold" => Weight::SEMI_BOLD,
        "bold" => Weight::BOLD,
        "extra-bold" => Weight::EXTRA_BOLD,
        "black" => Weight::BLACK,
        "extra-black" => Weight::EXTRA_BLACK,
        _ => return i32::from_str(value).ok().map(|w| Weight::from(w)),
    };
    Some(w)
}

fn parse_optional_text_decoration(value: Option<&String>) -> TextDecoration {
    if let Some(v) = value {
        parse_text_decoration(v)
    } else {
        TextDecoration::default()
    }
}

fn parse_text_decoration(value: &str) -> TextDecoration {
    let mut decoration = TextDecoration::default();
    for ty in value.split(" ") {
        let t = match value {
            "none" => TextDecoration::NO_DECORATION,
            "underline" => TextDecoration::UNDERLINE,
            "overline" => TextDecoration::OVERLINE,
            "line-through" => TextDecoration::LINE_THROUGH,
            _ => continue,
        };
        decoration.set(t, true);
    }
    decoration
}

#[test]
fn test_measure() {
    let text_demo = include_str!("../../Cargo.lock");
    let mut text = String::new();
    for i in 0..200 {
        text.push_str(text_demo);
    }
    // let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
    // debug!("font {:?}", font.typeface().family_name());
    // print_time!("measure time");
    // let result = font.measure_text(text.as_str(), None);
}

// #[test]
fn test_layout_performance() {
    let text_demo = include_str!("../../Cargo.lock");
    let params = ParagraphParams {
        line_height: Some(20.0),
        align: Default::default(),
        color: Default::default(),
        font_size: 16.0,
        font_families: vec!["monospace".to_string()],
        text_wrap: Some(false),
        mask_char: None,
    };
    let mut text = String::new();
    for i in 0..200 {
        text.push_str(text_demo);
    }
    //let mut file = File::create("target/test.txt").unwrap();
    // file.write_all(text.as_bytes()).unwrap();

    print_time!("build paragraph time");
    let unit = ParagraphUnit::Text(TextUnit {
        text: text.clone(),
        font_families: None,
        font_size: None,
        color: None,
        text_decoration_line: None,
        weight: None,
        background_color: None,
    });
    let mut p = Paragraph::build_paragraph(&params, &vec![unit]);
    p.layout(600.0);
}