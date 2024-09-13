use std::cell::Cell;
use std::rc::Rc;
use std::string::ToString;
use anyhow::Error;
use clipboard::{ClipboardContext, ClipboardProvider};
use quick_js::JsValue;
use skia_safe::{Canvas, Font, Paint};
use skia_safe::textlayout::{TextAlign};
use winit::keyboard::NamedKey;
use winit::window::CursorIcon;
use crate::base::{CaretDetail, ElementEvent, MouseDetail, MouseEventType, Rect, TextChangeDetail, TextUpdateDetail};
use crate::element::{ElementBackend, ElementRef};
use crate::element::text::{AtomOffset, Text as Label};
use crate::number::DeNan;
use crate::{js_call, match_event, match_event_type, timer};
use crate::element::edit_history::{EditHistory, EditOpType};
use crate::element::text::text_paragraph::Line;
use crate::event::{CaretEventBind, KEY_MOD_CTRL, KEY_MOD_SHIFT, KeyDownEvent, KeyEventDetail};
use crate::string::StringUtils;
use crate::timer::TimerHandle;

const COPY_KEY: &str = "\x03";
const PASTE_KEY: &str = "\x16";
const KEY_BACKSPACE: &str = "\x08";
const KEY_ENTER: &str = "\x0D";

pub struct Entry {
    base: Label,
    /// (row_offset, column_offset)
    caret: AtomOffset,
    // paint_offset: f32,
    // text_changed_listener: Vec<Box<TextChangeHandler>>,
    caret_visible: Rc<Cell<bool>>,
    caret_timer_handle: Option<TimerHandle>,
    selecting_begin: Option<AtomOffset>,
    focusing: bool,
    align: TextAlign,
    multiple_line: bool,
    element: ElementRef,
    vertical_caret_moving_coord_x: f32,
    edit_history: EditHistory,
}

pub type TextChangeHandler = dyn FnMut(&str);

impl Entry {

    pub fn get_text(&self) -> String {
        self.base.get_text()
    }

    pub fn with_paragraph<R, F: FnOnce(&mut Vec<Line>) -> R>(&self, callback: F) -> R {
        self.base.with_lines_mut(callback)
    }

    pub fn set_text(&mut self, text: String) {
        let old_text = self.base.get_text();
        if text != old_text {
            self.base.set_text(text);
            self.update_caret_value(self.base.get_atom_count() - 1, false);
        }
    }

    pub fn set_align(&mut self, align: TextAlign) {
        self.align = align;
        //self.update_paint_offset(self.context.layout.get_layout_width(), self.context.layout.get_layout_height());
        self.element.clone().mark_dirty(false);
    }

    pub fn set_multiple_line(&mut self, multiple_line: bool) {
        self.multiple_line = multiple_line;
        self.base.set_text_wrap(multiple_line);
        //self.update_paint_offset(self.context.layout.get_layout_width(), self.context.layout.get_layout_height());
        self.element.clone().mark_dirty(true);
    }

    pub fn get_font(&self) -> &Font {
        &self.base.get_font()
    }

    pub fn get_line_height(&self) -> Option<f32> {
        self.base.get_line_height()
    }

    pub fn get_computed_line_height(&self) -> f32 {
        self.base.get_computed_line_height()
    }

    pub fn set_caret(&mut self, atom_offset: usize) {
        self.update_caret_value(atom_offset, false);
    }

    fn move_caret(&mut self, delta: isize) {
        let max_atom_offset = (self.base.get_atom_count() - 1) as isize;
        let new_atom_offset = (self.caret as isize + delta).clamp(0, max_atom_offset) as AtomOffset;
        self.update_caret_value(new_atom_offset, false);
    }

    fn move_caret_vertical(&mut self, is_up: bool) {
        let (current_row, current_col) = self.base.get_location_by_atom_offset(self.caret);
        let line_height = self.with_paragraph(|ps| {
            let p = unsafe { ps.get_unchecked_mut(current_row) };
            p.get_soft_line_height(current_col)
        });

        let (caret_coord, _) = self.base.get_caret_offset_coordinate(self.caret);
        if self.vertical_caret_moving_coord_x <= 0.0 {
            let (caret_start, _) = self.base.get_caret_offset_coordinate(self.caret);
            self.vertical_caret_moving_coord_x = caret_start.0
        }
        let new_coord_y = if is_up {
            caret_coord.1 - line_height
        } else {
            caret_coord.1 + line_height
        };
        let new_coord = (self.vertical_caret_moving_coord_x, new_coord_y);
        self.update_caret_by_offset_coordinate(new_coord.0, new_coord.1, true);
    }

    fn update_caret_by_offset_coordinate(&mut self, x: f32, y: f32, is_kb_vertical: bool) {
        //let (x, y) = self.to_label_position((x, y));
        let position = if self.multiple_line {
            (x, y)
        } else {
            (x, 0.0)
        };
        let (row, col) = self.base.get_caret_at_offset_coordinate(position);
        let atom_offset = self.base.get_atom_offset_by_location((row, col));
        self.update_caret_value(atom_offset, is_kb_vertical);
    }

    fn update_caret_value(&mut self, new_caret: AtomOffset, is_kb_vertical: bool) {
        if !is_kb_vertical {
            self.vertical_caret_moving_coord_x = 0.0;
        }
        if new_caret != self.caret {
            self.caret = new_caret;
            if let Some(caret1) = &self.selecting_begin {
                let begin = AtomOffset::min(*caret1, new_caret);
                let end = AtomOffset::max(*caret1, new_caret);
                if begin != end {
                    self.base.select(begin, end);
                } else {
                    self.base.unselect();
                }
            }

            self.emit_caret_change();
            self.element.mark_dirty(false);
        }
    }

    fn emit_caret_change(&mut self) {
        let mut ele = self.element.clone();
        let origin_bounds = self.element.get_origin_bounds();
        let (border_top, _, _, border_left) = self.element.get_padding();

        let caret = self.caret;
        let (start, end) = self.base.get_caret_offset_coordinate(caret);
        // bounds relative to entry
        let bounds = Rect::new(start.0, start.1, 1.0, end.1 - start.1);
        let origin_bounds = bounds
            .translate(origin_bounds.x + border_left, origin_bounds.y + border_top);

        ele.emit_caret_change(CaretDetail::new(caret, origin_bounds, bounds));
    }

    fn caret_tick(caret_visible: Rc<Cell<bool>>, mut context: ElementRef) {
        let visible = caret_visible.get();
        caret_visible.set(!visible);
        context.mark_dirty(false);
    }

    fn to_label_position(&self, position: (f32, f32)) -> (f32, f32) {
        let ele = self.element.clone();
        let padding_left = ele.layout.get_layout_padding_left().de_nan(0.0);
        let padding_top = ele.layout.get_layout_padding_top().de_nan(0.0);
        return (position.0 - padding_left, position.1 - padding_top)
    }

    fn handle_blur(&mut self) {
        self.focusing = false;
        self.caret_timer_handle = None;
        self.caret_visible.set(false);
        self.element.mark_dirty(false);
    }

    fn begin_select(&mut self) {
        self.base.unselect();
        self.selecting_begin = Some(self.caret);
    }

    fn end_select(&mut self) {
        self.selecting_begin = None;
    }

    fn handle_mouse_event(&mut self, event: &MouseDetail) {
        match event.event_type {
            MouseEventType::MouseDown => {
                self.update_caret_by_offset_coordinate(event.offset_x, event.offset_y, false);
                self.begin_select();
            }
            MouseEventType::MouseMove => {
                if self.selecting_begin.is_some() {
                    self.update_caret_by_offset_coordinate(event.offset_x, event.offset_y, false);
                }
            }
            MouseEventType::MouseUp => {
                self.end_select();
            }
            _ => {},
        }
    }

    fn handle_key_down(&mut self, event: &KeyEventDetail) {
        if event.modifiers == 0 {
            if let Some(nk) = &event.named_key {
                match nk {
                    NamedKey::Backspace => {
                        if self.base.get_selection().is_none() {
                            self.base.select(self.caret - 1, self.caret);
                        }
                        self.handle_input("");
                    },
                    NamedKey::Enter => {
                        if self.multiple_line {
                            self.handle_input("\n");
                        }
                    },
                    NamedKey::ArrowLeft => {
                        self.move_caret(-1);
                    },
                    NamedKey::ArrowRight => {
                        self.move_caret(1);
                    },
                    NamedKey::ArrowUp => {
                        self.move_caret_vertical(true);
                    },
                    NamedKey::ArrowDown => {
                        self.move_caret_vertical(false);
                    }
                    NamedKey::Space => {
                        self.handle_input(" ");
                    },
                    NamedKey::Tab => {
                        //TODO use \t?
                        self.handle_input("   ");
                    },
                    _ => {}
                }
            } else if let Some(text) = &event.key_str {
                self.handle_input(&text);
            }
        } else if event.modifiers == KEY_MOD_SHIFT {
            if let Some(text) = &event.key_str {
                self.handle_input(&text);
            }
        } else if event.modifiers == KEY_MOD_CTRL {
            if let Some(text) = &event.key_str {
                match text.as_str() {
                    "c" | "x" => {
                        if let Some(sel) = self.base.get_selection_text() {
                            let sel=  sel.to_string();
                            if text == "x" {
                                self.handle_input("");
                            }
                            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                            ctx.set_contents(sel).unwrap();
                        }
                    },
                    "v" => {
                        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                        if let Ok(text) = ctx.get_contents() {
                            self.handle_input(&text);
                        }
                    }
                    "a" => {
                        self.base.set_selection((0, self.get_text().chars().count()))
                    }
                    "z" => {
                        self.undo();
                    }
                    _ => {}
                }
            }
        }
    }

    fn undo(&mut self) {
        if let Some(op) = &self.edit_history.undo() {
            match op.op {
                EditOpType::Insert => {
                    self.insert_text(op.content.as_str(), op.caret, false);
                }
                EditOpType::Delete => {
                    self.base.select(op.caret, op.caret + op.content.chars_count());
                    self.insert_text("", op.caret, false);
                }
            }
        }
    }

    fn handle_focus(&mut self) {
        self.focusing = true;
        // self.emit_caret_change();
        self.caret_visible.set(true);
        self.element.mark_dirty(false);
        self.caret_timer_handle = Some({
            let caret_visible = self.caret_visible.clone();
            let context = self.element.clone();
            timer::set_interval(move || {
                //println!("onInterval");
                Self::caret_tick(caret_visible.clone(), context.clone());
            }, 500)
        });
    }

    fn insert_text(&mut self, input: &str, caret: usize, record_history: bool) {
        if let Some((text, begin, end)) = self.base.get_selection_data() {
            if record_history {
                self.edit_history.record_delete(begin, &text);
            }
            self.base.delete_selected_text();
            self.update_caret_value(begin, false);
        }
        if !input.is_empty() {
            if record_history {
                self.edit_history.record_input(caret, input);
            }
            self.base.insert_text(caret, input);
            //TODO maybe update caret twice?
            self.update_caret_value(caret + input.chars_count(), false);
        }

        // emit text update
        let mut event = ElementEvent::new("textupdate", TextUpdateDetail {
            value: self.base.get_text().to_string()
        }, self.element.clone());
        self.element.emit_event("textupdate", &mut event);

        // emit text change
        let mut event = ElementEvent::new("textchange",TextChangeDetail {
            value: self.base.get_text().to_string(),
        }, self.element.clone());
        self.element.emit_event("textchange", &mut event);
    }

}

impl ElementBackend for Entry {

    fn create(mut ele: ElementRef) -> Self {
        let mut base = Label::create(ele.clone());
        base.set_text_wrap(false);
        ele.set_cursor(CursorIcon::Text);
        // Default style
        let caret_visible = Rc::new(Cell::new(false));
        Self {
            base,
            caret: 0,
            //paint_offset: 0f32,
            // text_changed_listener: Vec::new(),
            caret_visible,
            caret_timer_handle: None,
            selecting_begin: None,
            focusing: false,
            align: TextAlign::Left,
            multiple_line: false,
            element: ele,
            vertical_caret_moving_coord_x: 0.0,
            edit_history: EditHistory::new(),
        }
    }

    fn get_name(&self) -> &str {
        "Entry"
    }

    fn handle_style_changed(&mut self, key: &str) {
        self.base.handle_style_changed(key)
    }

    fn draw(&self, canvas: &Canvas) {
        //let paint = self.label.get_paint().clone();
        let mut paint = Paint::default();
        paint.set_color(self.base.get_color());

        let (start, end) = self.base.get_caret_offset_coordinate(self.caret);
        canvas.save();
        self.base.draw(canvas);
        if self.focusing && self.caret_visible.get() {
            paint.set_stroke_width(2.0);
            canvas.draw_line(start, end, &paint);
        }
        canvas.restore();
    }

    fn set_property(&mut self, p: &str, v: JsValue) {
        let mut label = &mut self.base;
        js_call!("text", String, self, set_text, p, v);
        js_call!("align", TextAlign, self, set_align, p, v);
        js_call!("fontsize", f32, label, set_font_size, p, v);
        js_call!("multipleline", bool, self, set_multiple_line, p, v);
        js_call!("selection", (usize, usize), label, set_selection, p, v);
        js_call!("caret", usize, self, set_caret, p, v);
    }

    fn get_property(&mut self, property_name: &str) -> Result<Option<JsValue>, Error> {
        self.base.get_property(property_name)
    }

    fn handle_input(&mut self, input: &str) {
        //println!("on input:{}", input);
        self.insert_text(input, self.caret, true);
    }

    fn handle_event_default_behavior(&mut self, event_type: &str, event: &mut ElementEvent) -> bool {
        KeyDownEvent::try_match(event_type, event, |d| {
            self.handle_key_down(d)
        })
            || self.base.handle_event_default_behavior(event_type, event)
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.base.handle_origin_bounds_change(bounds);
        //self.update_paint_offset(bounds.width, bounds.height);
    }

    fn handle_event(&mut self, event_type: &str, event: &mut ElementEvent) {
        match_event!(event_type, event, "focus", self, handle_focus);
        match_event!(event_type, event, "blur", self, handle_blur);
        match_event_type!(event, MouseDetail, self, handle_mouse_event);
    }

}

#[test]
pub fn test_edit_history() {
    let mut el = ElementRef::new(Entry::create);
    let entry = el.get_backend_mut_as::<Entry>();
    let text1 = "hello";
    let text2 = "world";
    let text_all = "helloworld";
    // input text1
    entry.handle_input(text1);
    assert_eq!(text1, entry.get_text());
    // input text2
    entry.handle_input(text2);
    assert_eq!(text_all, entry.get_text());
    // delete text2
    entry.base.select(text1.chars_count(), text1.chars_count() + text2.chars_count());
    entry.handle_input("");
    assert_eq!(text1, entry.get_text());
    // undo
    entry.undo();
    assert_eq!(text_all, entry.get_text());
    assert_eq!(text_all.chars_count(), entry.caret);
    entry.undo();
    assert_eq!("", entry.get_text());
    assert_eq!(0, entry.caret);
}
