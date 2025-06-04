mod line;
mod util;

use crate as deft;
use crate::base::EventContext;
use crate::color::parse_hex_color;
use crate::element::paragraph::simple_paragraph_builder::SimpleParagraphBuilder;
use crate::element::paragraph::ParagraphParams;
use crate::element::text::intersect_range;
use crate::element::text::simple_text_paragraph::SimpleTextParagraph;
use crate::element::{ElementBackend, ElementWeak};
use crate::event::{
    ClickEvent, KeyDownEvent, KeyEventDetail, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    KEY_MOD_CTRL,
};
use crate::font::family::{FontFamilies, FontFamily};
use crate::number::DeNan;
use crate::paint::Painter;
use crate::render::RenderFn;
use crate::string::StringUtils;
use crate::style::color::parse_optional_color_str;
use crate::style::font::FontStyle;
use crate::style::PropValueParse;
use crate::text::textbox::line::Line;
use crate::text::textbox::util::{parse_optional_text_decoration, parse_optional_weight};
use crate::text::{TextAlign, TextStyle};
use crate::{base, js_deserialize, js_serialize, some_or_continue};
use serde::{Deserialize, Serialize};
use skia_safe::font_style::{Weight, Width};
use skia_safe::{Color, Paint};
use std::any::Any;

#[cfg(target_os = "windows")]
pub const DEFAULT_FALLBACK_FONTS: &str = "sans-serif,Microsoft YaHei,Segoe UI Emoji";
#[cfg(target_os = "linux")]
pub const DEFAULT_FALLBACK_FONTS: &str = "sans-serif,Noto Sans CJK SC,Noto Sans CJK TC,Noto Sans CJK HK,Noto Sans CJK KR,Noto Sans CJK JP,Noto Color Emoji";
#[cfg(target_os = "macos")]
pub const DEFAULT_FALLBACK_FONTS: &str = "PingFang SC,Heiti SC,sans-serif,Apple Color Emoji";
#[cfg(target_os = "android")]
pub const DEFAULT_FALLBACK_FONTS: &str = "Roboto,Noto Sans CJK SC,Noto Sans CJK TC,Noto Sans CJK HK,Noto Sans CJK KR,Noto Sans CJK JP,Noto Color Emoji";
#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android"
)))]
pub const DEFAULT_FALLBACK_FONTS: &str = "sans-serif";

const ZERO_WIDTH_WHITESPACE: &str = "\u{200B}";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TextElement {
    Text(TextUnit),
}

js_serialize!(TextElement);
js_deserialize!(TextElement);

impl TextElement {
    fn atom_count(&self) -> usize {
        match self {
            TextElement::Text(text) => text.text.chars_count(),
        }
    }
    fn text(&self) -> &str {
        match self {
            TextElement::Text(t) => t.text.as_str(),
        }
    }

    fn get_text(&self, begin: usize, end: usize) -> &str {
        match self {
            TextElement::Text(t) => t.text.substring(begin, end - begin),
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
    pub style: Option<String>,
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

pub struct TextBox {
    params: ParagraphParams,
    lines: Vec<Line>,
    /// Option<(start coord, end coord)>
    selection: Option<(TextCoord, TextCoord)>,
    selecting_begin: Option<TextCoord>,
    selection_bg: Paint,
    selection_fg: Paint,
    width: f32,
    padding: (f32, f32, f32, f32),
    /// (row_offset, column_offset)
    caret: TextCoord,
    vertical_caret_moving_coord_x: f32,
    repaint_callback: Box<dyn FnMut()>,
    layout_callback: Box<dyn FnMut()>,
    caret_change_callback: Box<dyn FnMut()>,
}

impl TextBox {
    pub fn add_line(&mut self, units: Vec<TextElement>) {
        let line = Line::new(units, &self.params);
        self.lines.push(line);
        self.request_layout();
    }

    pub fn insert_line(&mut self, index: usize, units: Vec<TextElement>) {
        let line = Line::new(units, &self.params);
        self.lines.insert(index, line);
        self.request_layout();
    }

    pub fn delete_line(&mut self, line: usize) {
        self.lines.remove(line);
        self.request_layout();
    }

    pub fn update_line(&mut self, index: usize, units: Vec<TextElement>) {
        self.lines[index] = Line::new(units, &self.params);
        self.request_layout();
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.request_layout();
    }

    pub fn measure_line(&self, units: Vec<TextElement>) -> (f32, f32) {
        let mut sk_paragraph = Self::build_paragraph(&self.params, &units);
        sk_paragraph.layout(f32::NAN);
        (sk_paragraph.max_intrinsic_width(), sk_paragraph.height())
    }

    pub fn set_text_wrap(&mut self, wrap: bool) {
        if self.params.text_wrap != Some(wrap) {
            self.params.text_wrap = Some(wrap);
            self.rebuild_paragraph();
        }
    }

    pub fn set_font_size(&mut self, size: f32) {
        if self.params.font_size != size {
            self.params.font_size = size;
            self.rebuild_paragraph();
        }
    }

    pub fn set_color(&mut self, color: Color) {
        if self.params.color != color {
            self.params.color = color;
            self.rebuild_paragraph();
        }
    }

    pub fn set_font_families(&mut self, font_families: FontFamilies) {
        if self.params.font_families != font_families {
            self.params.font_families = font_families;
            self.rebuild_paragraph();
        }
    }

    pub fn set_font_weight(&mut self, weight: Weight) {
        if self.params.font_weight != weight {
            self.params.font_weight = weight;
            self.rebuild_paragraph();
        }
    }

    pub fn set_font_style(&mut self, style: FontStyle) {
        if self.params.font_style != style {
            self.params.font_style = style;
            self.rebuild_paragraph();
        }
    }

    pub fn set_line_height(&mut self, line_height: Option<f32>) {
        if self.params.line_height != line_height {
            self.params.line_height = line_height;
            self.rebuild_paragraph();
        }
    }

    pub fn get_paragraph_params(&self) -> &ParagraphParams {
        &self.params
    }

    pub fn set_padding(&mut self, padding: (f32, f32, f32, f32)) {
        self.padding = padding;
    }

    pub fn get_size_without_padding(&self) -> (f32, f32) {
        let width = self.max_intrinsic_width();
        let height = self.height();
        (
            width - self.padding.1 - self.padding.0,
            height - self.padding.0 - self.padding.2,
        )
    }

    pub fn set_layout_width(&mut self, width: f32) {
        if self.width != width {
            self.width = width;
            self.invalid_all_lines();
            self.request_layout();
        }
    }

    pub fn layout(&mut self) {
        let (_, padding_right, _, padding_left) = self.padding;
        for ln in &mut self.lines {
            if !ln.layout_calculated {
                ln.force_layout(self.width - padding_left - padding_right);
                ln.layout_calculated = true;
            }
        }
    }

    pub fn height(&self) -> f32 {
        let mut height = 0.0;
        for ln in &self.lines {
            height += ln.sk_paragraph.height().de_nan(0.0);
        }
        let (padding_top, _, padding_bottom, _) = self.padding;
        height + padding_top + padding_bottom
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        let (_, padding_right, _, padding_left) = self.padding;
        let mut max_width = 0.0;
        for ln in &self.lines {
            max_width = f32::max(max_width, ln.sk_paragraph.max_intrinsic_width());
        }
        max_width + padding_right + padding_left
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
        self.request_repaint();
    }

    pub fn is_selecting(&self) -> bool {
        self.selecting_begin.is_some()
    }

    pub fn unselect(&mut self) {
        self.selection = None;
        self.request_repaint();
    }

    fn begin_select(&mut self, caret: TextCoord) {
        // self.element.emit(FocusShiftEvent);
        self.unselect();
        self.selecting_begin = Some(caret);
        // self.element.emit(SelectStartEvent {
        //     row: caret.0,
        //     col: caret.1,
        // });
    }

    fn selection_start(&mut self, begin_coord: TextCoord) {
        self.begin_select(begin_coord);
    }

    fn selection_update(&mut self, caret: TextCoord) -> bool {
        if self.selecting_begin.is_some() {
            if let Some(sb) = self.selecting_begin {
                // self.element.emit(SelectMoveEvent{
                //     row: caret.0,
                //     col: caret.1,
                // });
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
            // self.element.emit(SelectEndEvent);
            return true;
        }
        false
    }

    fn end_select(&mut self) {
        self.selecting_begin = None;
    }

    fn on_mouse_down(&mut self, point: (f32, f32)) {
        let begin_coord = self.get_text_coord_by_pixel_coord(point);
        self.begin_select(begin_coord);
    }

    fn on_mouse_up(&mut self) {
        self.end_select();
    }

    fn on_mouse_move(&mut self, point: (f32, f32)) {
        if self.selecting_begin.is_some() {
            let caret = self.get_text_coord_by_pixel_coord(point);
            if let Some(sb) = self.selecting_begin {
                let start = TextCoord::min(sb, caret);
                let end = TextCoord::max(sb, caret);
                self.select(start, end);
            }
        }
    }

    pub fn get_text_coord_by_pixel_coord(&self, mut coord: (f32, f32)) -> TextCoord {
        let (padding_top, _, _, padding_left) = self.padding;
        coord.0 -= padding_left;
        coord.1 -= padding_top;
        let expected_offset = coord;
        let mut row = 0;
        let mut height = 0f32;

        let lines = &self.lines;
        let max_offset = if lines.is_empty() { 0 } else { lines.len() - 1 };
        if expected_offset.1 > 0.0 {
            let mut last_cols = 0;
            for p in lines {
                height += p.sk_paragraph.height();
                if height > expected_offset.1 {
                    let line_pixel_coord = (
                        expected_offset.0,
                        expected_offset.1 - (height - p.sk_paragraph.height()),
                    );
                    let line_column = p.get_column_by_pixel_coord(line_pixel_coord);
                    return TextCoord(row, line_column);
                }
                row += 1;
                last_cols = p.atom_count();
            }
            TextCoord(max_offset, last_cols)
        } else {
            TextCoord(0, 0)
        }
    }

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

    pub fn get_caret_rect(&mut self) -> Option<base::Rect> {
        self.get_char_rect(self.caret)
    }

    pub fn get_char_rect(&mut self, coord: TextCoord) -> Option<crate::base::Rect> {
        let (row, col) = (coord.0, coord.1);
        let line = self.lines.get_mut(row)?;
        let layout = line.sk_paragraph.layout.as_ref()?;
        let bounds = layout.get_char_bounds(col)?;
        let mut y_offset = 0.0;
        if row > 0 {
            for i in 0..row {
                y_offset += unsafe { self.lines.get_unchecked(i).sk_paragraph.height() }
            }
        }
        let (padding_top, _, _, padding_left) = self.padding;
        Some(crate::base::Rect::new(
            bounds.left + padding_left,
            y_offset + bounds.top + padding_top,
            bounds.width(),
            bounds.height(),
        ))
    }

    pub fn on_event(
        &mut self,
        event: &Box<&mut dyn Any>,
        _ctx: &mut EventContext<ElementWeak>,
        scroll_x: f32,
        scroll_y: f32,
    ) -> bool {
        if let Some(d) = event.downcast_ref::<KeyDownEvent>() {
            return self.on_key_down(&d.0);
        } else if let Some(e) = event.downcast_ref::<MouseDownEvent>() {
            if e.0.button == 1 {
                let event = e.0;
                let begin_coord = self.get_text_coord_by_pixel_coord((
                    event.offset_x + scroll_x,
                    event.offset_y + scroll_y,
                ));
                self.update_caret(begin_coord);
                self.selection_start(begin_coord);
                return true;
            }
        } else if let Some(e) = event.downcast_ref::<MouseMoveEvent>() {
            if self.selecting_begin.is_some() {
                let event = e.0;
                let caret = self.get_text_coord_by_pixel_coord((
                    event.offset_x + scroll_x,
                    event.offset_y + scroll_y,
                ));
                self.update_caret(caret);
                return self.selection_update(caret);
            }
        } else if let Some(e) = event.downcast_ref::<MouseUpEvent>() {
            if e.0.button == 1 {
                return self.selection_end();
            }
        } else if let Some(e) = event.downcast_ref::<ClickEvent>() {
            let caret = self
                .get_text_coord_by_pixel_coord((e.0.offset_x + scroll_x, e.0.offset_y + scroll_y));
            self.update_caret(caret);
        }
        false
    }

    pub fn on_key_down(&mut self, event: &KeyEventDetail) -> bool {
        if event.modifiers == KEY_MOD_CTRL {
            if let Some(text) = &event.key_str {
                match text.as_str() {
                    #[cfg(feature = "clipboard")]
                    "c" => {
                        use clipboard::{ClipboardContext, ClipboardProvider};
                        if let Some(sel) = self.get_selection_text() {
                            let sel = sel.to_string();
                            if let Ok(mut ctx) = ClipboardContext::new() {
                                if let Err(e) = ctx.set_contents(sel) {
                                    log::error!("Failed to write clipboard: {:?}", e);
                                }
                            }
                        }
                        return true;
                    }
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

    pub fn get_selection_text(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let start = selection.0;
        let end = selection.1;
        let start_line = self.lines.get(start.0)?;
        let end_line = self.lines.get(end.0)?;
        let text = if start.0 == end.0 {
            start_line.subtext(start.1, end.1)
        } else {
            let mut result = start_line.subtext(start.1, start_line.atom_count());
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
        if self.params.mask_char != mask_char {
            self.params.mask_char = mask_char;
            self.rebuild_paragraph();
            self.request_layout();
        }
    }

    pub fn move_caret(&mut self, mut delta: isize) {
        let mut row = self.caret.0;
        let mut col = self.caret.1 as isize;
        loop {
            let lines = self.get_lines();
            let line = match lines.get(row) {
                None => return,
                Some(ln) => ln,
            };
            let atom_count = line.atom_count() as isize;
            col += delta;
            if col > atom_count {
                delta -= col - atom_count;
                row += 1;
                col = 0;
                continue;
            } else if col < 0 {
                if row == 0 {
                    return;
                }
                delta += -col;
                row -= 1;
                let prev_line = lines.get(row);
                col = prev_line.unwrap().atom_count() as isize;
                continue;
            } else {
                let new_caret = (row, col as usize);
                self.update_caret_value(TextCoord::new(new_caret), false);
                break;
            }
        }
    }

    pub fn move_caret_vertical(&mut self, is_up: bool) {
        let caret = self.caret;
        let (current_row, current_col) = (self.caret.0, self.caret.1);
        let line_height = match self.get_soft_line_height(current_row, current_col) {
            None => return,
            Some(height) => height,
        };
        let caret_coord = match self.get_char_rect(caret) {
            None => return,
            Some(rect) => rect,
        };

        if self.vertical_caret_moving_coord_x <= 0.0 {
            self.vertical_caret_moving_coord_x = caret_coord.x;
        }
        let new_coord_y = if is_up {
            caret_coord.y - line_height
        } else {
            caret_coord.y + line_height
        };
        let new_coord = (self.vertical_caret_moving_coord_x, new_coord_y);
        self.update_caret_by_offset_coordinate(new_coord.0, new_coord.1, true);
    }

    pub fn update_caret_value(&mut self, new_caret: TextCoord, is_kb_vertical: bool) {
        if !is_kb_vertical {
            self.vertical_caret_moving_coord_x = 0.0;
        }
        self.update_caret(new_caret);
    }

    pub fn update_caret_by_offset_coordinate(&mut self, x: f32, y: f32, is_kb_vertical: bool) {
        let text_coord = self.get_text_coord_by_pixel_coord((x, y));
        self.update_caret_value(text_coord, is_kb_vertical);
    }

    fn rebuild_paragraph(&mut self) {
        let params = self.params.clone();
        for ln in &mut self.lines {
            ln.rebuild_paragraph(&params);
        }
        self.request_layout();
    }

    fn parse_font_style(value: &Option<String>, default: FontStyle) -> FontStyle {
        let mut result = None;
        if let Some(value) = value {
            result = FontStyle::parse_prop_value(value);
        }
        result.unwrap_or(default)
    }

    pub fn build_paragraph(
        paragraph_params: &ParagraphParams,
        units: &Vec<TextElement>,
    ) -> SimpleTextParagraph {
        // let mut text = text.trim_line_endings().to_string();
        // text.push_str(ZERO_WIDTH_WHITESPACE);

        let mut pb = SimpleParagraphBuilder::new(paragraph_params);
        let p_color = paragraph_params.color;
        let mask_char = paragraph_params.mask_char;
        for u in units {
            match u {
                TextElement::Text(unit) => {
                    let mut text_style = TextStyle::new();
                    let unit_font_families = match &unit.font_families {
                        Some(list) => {
                            let list = list.iter().map(|it| FontFamily::new(it.as_str())).collect();
                            FontFamilies::new(list)
                        }
                        None => FontFamilies::default(),
                    };
                    text_style.set_font_families(Some(unit_font_families));
                    let font_size = unit.font_size.unwrap_or(paragraph_params.font_size);
                    text_style.set_font_size(font_size);

                    let weight = parse_optional_weight(unit.weight.as_ref())
                        .unwrap_or(paragraph_params.font_weight);

                    let unit_style =
                        Self::parse_font_style(&unit.style, paragraph_params.font_style);
                    let font_style =
                        skia_safe::FontStyle::new(weight, Width::NORMAL, unit_style.to_slant());
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

    pub fn new() -> Self {
        let font_families: FontFamilies = FontFamilies::default();

        let params = ParagraphParams {
            text_wrap: Some(true),
            line_height: None,
            align: TextAlign::Left,
            color: Color::from_rgb(0, 0, 0),
            font_size: 12.0,
            font_families,
            font_weight: Weight::NORMAL,
            font_style: FontStyle::Normal,
            mask_char: None,
        };

        let mut selection_bg = Paint::default();
        selection_bg.set_color(parse_hex_color("214283").unwrap());
        let mut selection_fg = Paint::default();
        selection_fg.set_color(Color::from_rgb(255, 255, 255));
        Self {
            lines: Vec::new(),
            params,
            selection: None,
            selecting_begin: None,
            selection_bg,
            selection_fg,
            width: f32::NAN,
            padding: (0.0, 0.0, 0.0, 0.0),
            caret: TextCoord(0, 0),
            vertical_caret_moving_coord_x: 0.0,
            repaint_callback: Box::new(|| {}),
            layout_callback: Box::new(|| {}),
            caret_change_callback: Box::new(|| {}),
        }
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    pub fn get_caret(&self) -> TextCoord {
        self.caret
    }

    pub fn render(&mut self) -> RenderFn {
        let mut consumed_top = 0.0;
        let mut consumed_rows = 0usize;

        let selection = self.selection;
        let selection_bg = self.selection_bg.clone();
        let selection_fg = self.selection_fg.clone();

        let mut line_painters = Vec::with_capacity(self.lines.len());
        for ln in &mut self.lines {
            let ln_row = consumed_rows;
            consumed_rows += 1;

            let ln_height = ln.sk_paragraph.height();
            let ln_top = consumed_top;
            consumed_top += ln_height;
            let ln_bottom = consumed_top;
            let atom_count = ln.atom_count();
            let ln_layout = some_or_continue!(ln.sk_paragraph.layout.clone());

            let selection_bg = selection_bg.clone();
            let selection_fg = selection_fg.clone();
            let ln_renderer = move |painter: &Painter| {
                let clip_rect = painter.canvas.local_clip_bounds();
                if let Some(cp) = clip_rect {
                    if ln_bottom < cp.top {
                        return true;
                    } else if ln_top > cp.bottom {
                        return false;
                    }
                }
                ln_layout.paint(painter, (0.0, ln_top).into());

                if atom_count > 0 {
                    if let Some(selection_range) = selection {
                        let ln_range = (TextCoord(ln_row, 0), TextCoord(ln_row, atom_count));
                        if let Some((begin, end)) = intersect_range(selection_range, ln_range) {
                            ln_layout.paint_selection(
                                painter,
                                (0.0, ln_top),
                                ln_height,
                                (begin.1, end.1),
                                &selection_bg,
                                &selection_fg,
                            );
                        }
                    }
                }
                true
            };
            line_painters.push(ln_renderer);
        }

        let (padding_top, _, _, padding_left) = self.padding;

        RenderFn::new(move |painter| {
            let canvas = painter.canvas;
            canvas.translate((padding_left, padding_top));
            for lp in line_painters {
                if !lp(painter) {
                    break;
                }
            }
        })
    }

    pub fn set_layout_callback<F: FnMut() + 'static>(&mut self, callback: F) {
        self.layout_callback = Box::new(callback);
    }

    pub fn set_repaint_callback<F: FnMut() + 'static>(&mut self, callback: F) {
        self.repaint_callback = Box::new(callback);
    }

    pub fn set_caret_change_callback<F: FnMut() + 'static>(&mut self, callback: F) {
        self.caret_change_callback = Box::new(callback);
    }

    fn invalid_all_lines(&mut self) {
        for ln in &mut self.lines {
            ln.layout_calculated = false;
        }
    }

    fn update_caret(&mut self, caret: TextCoord) {
        self.caret = caret;
        (self.caret_change_callback)();
    }

    fn request_layout(&mut self) {
        (self.layout_callback)();
    }

    fn request_repaint(&mut self) {
        (self.repaint_callback)();
    }
}

#[cfg(test)]
mod tests {
    use crate::element::common::editable::Editable;
    use crate::element::paragraph::ParagraphParams;
    use crate::font::family::{FontFamilies, FontFamily};
    use crate::style::font::FontStyle;
    use crate::text::textbox::{TextBox, TextElement, TextUnit};
    use crate::text::TextAlign;
    use measure_time::print_time;
    use skia_safe::font_style::Weight;

    // #[test]
    fn text_text_layout() {
        let mut text = TextBox::new();
        text.add_line(Editable::build_line("你好".to_string()));
        text.layout();
        assert_ne!(0.0, text.max_intrinsic_width());
    }

    #[test]
    fn test_measure() {
        let text_demo = include_str!("../../Cargo.lock");
        let mut text = String::new();
        for _ in 0..200 {
            text.push_str(text_demo);
        }
        // let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
        // debug!("font {:?}", font.typeface().family_name());
        // print_time!("measure time");
        // let result = font.measure_text(text.as_str(), None);
    }

    #[cfg(test)]
    fn test_layout_performance() {
        let text_demo = include_str!("../../Cargo.lock");
        let params = ParagraphParams {
            line_height: Some(20.0),
            align: TextAlign::Left,
            color: Default::default(),
            font_size: 16.0,
            font_families: FontFamilies::new(vec![FontFamily::new("monospace")]),
            font_weight: Weight::NORMAL,
            font_style: FontStyle::Normal,
            text_wrap: Some(false),
            mask_char: None,
        };
        let mut text = String::new();
        for _ in 0..200 {
            text.push_str(text_demo);
        }
        //let mut file = File::create("target/test.txt").unwrap();
        // file.write_all(text.as_bytes()).unwrap();

        print_time!("build paragraph time");
        let unit = TextElement::Text(TextUnit {
            text: text.clone(),
            font_families: None,
            font_size: None,
            color: None,
            text_decoration_line: None,
            weight: None,
            background_color: None,
            style: None,
        });
        let mut p = TextBox::build_paragraph(&params, &vec![unit]);
        p.layout(600.0);
    }
}
