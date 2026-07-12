use crate as deft;
use crate::app::AppEvent;
use crate::base::{Callback, Rect};
use crate::canvas_util::CanvasHelper;
use crate::element::edit_history::{EditDetail, EditHistory};
use crate::element::{Element, Widget, ElementDelegate, ElementWeak};
use crate::event::{BlurEventListener, BoundsChangeEventListener, CaretChangeEvent, FocusEventListener, KeyDownEventListener, KeyEventDetail, MouseDownEventListener, MouseLeaveEventListener, PreeditEventListener, ScrollEventListener, TextChangeEvent, TextInputEventListener, TextUpdateEvent, KEY_MOD_CTRL, KEY_MOD_SHIFT};
use crate::event_loop::create_event_loop_proxy;
use crate::js::{FromJsValue, ToJsValue};
use crate::number::DeNan;
use crate::render::RenderFn;
use crate::string::StringUtils;
use crate::style::{ResolvedStyleProp, StylePropKey};
use crate::text::textbox::{TextBox, TextCoord, TextElement, TextUnit};
use crate::text::TextAlign;
use crate::timer::TimerHandle;
use crate::winit::dpi::{LogicalPosition, LogicalSize, Size};
use crate::{ok_or_return, timer};
use deft_macros::{widget, js_methods, mrc_object};
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use skia_safe::{Color, Paint};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use winit::keyboard::NamedKey;
use winit::window::{Cursor, CursorIcon};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InputType {
    Text,
    Password,
}

impl ToJsValue for InputType {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        let str = match self {
            InputType::Text => "text",
            InputType::Password => "password",
        };
        Ok(JsValue::String(str.to_string()))
    }
}

impl FromJsValue for InputType {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        if let JsValue::String(value) = value {
            let value = value.to_lowercase();
            match value.as_str() {
                "text" => Ok(InputType::Text),
                "password" => Ok(InputType::Password),
                _ => Err(ValueError::UnexpectedType),
            }
        } else {
            Err(ValueError::UnexpectedType)
        }
    }
}

#[derive(Default)]
struct EditableState {
    focusing: bool,
    caret_visible: bool,
    caret_paint: Paint,
    caret_pos: Rect,
    has_text: bool,
}

#[mrc_object]
struct EditableVar {
    element: ElementWeak,
    line_height: Option<f32>,
    multiple_line: bool,
    paragraph: TextBox,
    placeholder: TextBox,
    layout_dirty: bool,
    input_type: InputType,
    caret_timer_handle: Option<TimerHandle>,
    align: TextAlign,
    edit_history: EditHistory,
    rows: u32,
    disabled: bool,
    auto_height: bool,
    state: Arc<Mutex<EditableState>>,
}

impl EditableVar {

    fn get_caret_pixels_position(&self) -> Option<Rect> {
        let el = self.element.upgrade().ok()?;
        let (scroll_left, scroll_top) = el.scrollable.scroll_offset();
        //TODO remove clone
        let caret_rect = self.clone().paragraph.get_caret_rect()?;
        let x = caret_rect.x - scroll_left;
        let y = caret_rect.y - scroll_top;
        Some(Rect::new(x, y, 1.0, caret_rect.height))
    }

    fn update_ime(&self) -> Option<()> {
        let el = self.element.upgrade().ok()?;
        let pos = self.get_caret_pixels_position()?;
        let win = el.get_window()?;
        let win = win.upgrade().ok()?;
        //TOOD use transformed position
        let el_offset = el.get_origin_bounds();
        let x = (el_offset.x + pos.x) as f64;
        let y = (el_offset.y + pos.bottom()) as f64;
        win.window.set_ime_cursor_area(
            crate::winit::dpi::Position::Logical(LogicalPosition { x, y }),
            Size::Logical(LogicalSize {
                width: 1.0,
                height: 1.0,
            }),
        );
        let mut state = self.state.lock().unwrap();
        state.caret_pos = pos;
        Some(())
    }

}

struct EditableDelegate {
    var: EditableVar,
    state: Arc<Mutex<EditableState>>,
}

impl EditableVar {

    fn layout(&mut self, bounds: &Rect) {
        let element = ok_or_return!(self.element.upgrade());
        let padding = element.get_padding();
        let border = element.get_border_width();
        let mut line_height = self.line_height;
        let padding_box_width = bounds.width.de_nan(f32::INFINITY) - border.1 - border.3;
        let padding_box_height = bounds.height.de_nan(f32::INFINITY) - border.0 - border.2;

        let mut layout_width = padding_box_width;
        if !self.multiple_line {
            let content_height = padding_box_height;
            line_height = Some(content_height);
            layout_width = f32::NAN;
        }

        self.placeholder.set_line_height(line_height);
        self.paragraph.set_line_height(line_height);

        self.placeholder.set_padding(padding);
        self.placeholder.set_layout_width(layout_width);
        self.placeholder.layout();

        self.paragraph.set_padding(padding);
        self.paragraph.set_layout_width(layout_width);
        self.paragraph.layout();
        self.layout_dirty = false;
    }

    fn handle_focus(&mut self) {
        let _ = self.update_ime();
        {
            let mut state = self.state.lock().unwrap();
            state.focusing = true;
            // self.emit_caret_change();
            state.caret_visible = true;
        }
        self.caret_timer_handle = Some({
            let state = self.state.clone();
            let context = self.element.clone();
            timer::set_interval(
                move || {
                    //debug!("onInterval");
                    Editable::caret_tick(state.clone(), context.clone());
                },
                500,
            )
        });
        self.element.mark_dirty(false);
        let el = ok_or_return!(self.element.upgrade());
        if let Some(window) = el.get_window() {
            if let Ok(f) = window.upgrade() {
                let elp = create_event_loop_proxy();
                elp.send_event(AppEvent::ShowSoftInput(f.get_id())).unwrap();
            }
        }
    }

    fn handle_blur(&mut self) {
        self.caret_timer_handle = None;
        {
            let mut state = self.state.lock().unwrap();
            state.focusing = false;
            state.caret_visible = false;
            let mut el = ok_or_return!(self.element.upgrade());
            if let Some(window) = el.get_window() {
                if let Ok(f) = window.upgrade() {
                    let elp = create_event_loop_proxy();
                    elp.send_event(AppEvent::HideSoftInput(f.get_id())).unwrap();
                }
            }
            el.mark_dirty(false);
        }
    }

    fn insert_text(&mut self, input: &str, mut caret: TextCoord, record_history: bool) {
        let mut delete_detail = None;
        let mut insert_detail = None;
        let selection = self.paragraph.get_selection();
        if !selection.is_empty() {
            let (start, end) = selection.normalize();
            let selected_text = self.paragraph.get_selection_text().unwrap_or(String::new());
            if start.0 == end.0 {
                let line_text = self.paragraph.get_line_text(start.0).unwrap();
                let left = line_text.substring(0, start.1);
                let right = line_text.substring(end.1, line_text.chars_count());
                let new_text = format!("{}{}", left, right);
                self.paragraph
                    .update_line(caret.0, Editable::build_line(new_text));
            } else {
                let first_line = self.paragraph.get_line_text(start.0).unwrap();
                let left = first_line.substring(0, start.1).to_string();
                let last_line = self.paragraph.get_line_text(end.0).unwrap();
                let right = last_line
                    .substring(end.1, last_line.chars_count())
                    .to_string();
                self.paragraph
                    .update_line(start.0, Editable::build_line(format!("{}{}", left, right)));
                if end.0 > start.0 {
                    for _ in start.0..end.0 {
                        self.paragraph.delete_line(start.0 + 1);
                    }
                }
            }
            self.paragraph.unselect();
            self.update_caret_value(start, false);
            caret = start;
            if start != end {
                delete_detail = Some(EditDetail {
                    content: selected_text,
                    end,
                });
            }
        }
        let start_caret = caret;
        if !input.is_empty() {
            let line_text = self.paragraph.get_line_text(caret.0).unwrap();
            let left_str = line_text.substring(0, caret.1);
            let right_str = line_text.substring(caret.1, line_text.len() - caret.1);
            let input_lines = input.split('\n').collect::<Vec<&str>>();
            let new_caret = if input_lines.len() == 1 {
                let new_text = format!("{}{}{}", left_str, input, right_str);
                self.paragraph
                    .update_line(caret.0, Editable::build_line(new_text));
                TextCoord(caret.0, caret.1 + input.chars_count())
            } else {
                let first_line = format!("{}{}", left_str, unsafe { input_lines.get_unchecked(0) });
                self.paragraph
                    .insert_line(caret.0, Editable::build_line(first_line));
                if input_lines.len() > 2 {
                    for i in 1..input_lines.len() - 1 {
                        let line = unsafe { input_lines.get_unchecked(i).to_string() };
                        self.paragraph
                            .insert_line(caret.0 + i, Editable::build_line(line));
                    }
                }
                let last_line = format!(
                    "{}{}",
                    unsafe { input_lines.get_unchecked(input_lines.len() - 1) },
                    right_str
                );
                self.paragraph
                    .update_line(caret.0 + input_lines.len() - 1, Editable::build_line(last_line));
                TextCoord(
                    caret.0 + input_lines.len() - 1,
                    input_lines.last().unwrap().chars_count(),
                )
            };
            //TODO maybe update caret twice?
            self.update_caret_value(new_caret, false);
            insert_detail = Some(EditDetail {
                content: input.to_string(),
                end: self.paragraph.get_caret(),
            });
        }

        if record_history {
            self.edit_history.record_input(start_caret, delete_detail, insert_detail);
        }

        // emit text update
        let text = self.paragraph.get_text().to_string();
        self.element.emit(TextUpdateEvent {
            value: text.clone(),
        });

        // emit text change
        self.element.emit(TextChangeEvent { value: text });

        self.element.mark_dirty(true);
    }

    fn update_caret_value(&mut self, new_caret: TextCoord, is_kb_vertical: bool) {
        let old_caret = self.paragraph.get_caret();
        self.paragraph.update_caret_value(new_caret, is_kb_vertical);
        if new_caret != old_caret {
            //TODO remove?
            // if let Some(caret1) = &self.selecting_begin {
            //     let begin = TextCoord::min(*caret1, new_caret);
            //     let end = TextCoord::max(*caret1, new_caret);
            //     if begin != end {
            //         self.base.select(begin, end);
            //     } else {
            //         self.base.unselect();
            //     }
            // }
            self.element.mark_dirty(false);
            //TODO do not use loop callback?
            // Note: here use loop callback because of paragraph has not been layout when receive caret change event
            let mut me = self.clone();
            let var_weak = self.as_weak();
            let callback = Callback::new(move || {
                if let Ok(var) = var_weak.upgrade() {
                    var.update_ime();
                }
                me.emit_caret_change();
            });
            let el = ok_or_return!(self.element.upgrade());
            el.with_window(|mut w| {
                w.request_next_paint_callback(callback);
            });
        }
    }

    fn emit_caret_change(&mut self) {
        let element = ok_or_return!(self.element.upgrade());
        let origin_bounds = element.get_origin_bounds();
        let (border_top, _, _, border_left) = element.get_padding();
        let (scroll_left, scroll_top) = element.scrollable.scroll_offset();

        let caret = self.paragraph.get_caret();
        let bounds = match self.paragraph.get_char_rect(caret) {
            None => return,
            Some(rect) => rect.translate(-scroll_left, -scroll_top),
        };
        // bounds relative to entry
        let origin_bounds =
            bounds.translate(origin_bounds.x + border_left, origin_bounds.y + border_top);

        self.element.emit(CaretChangeEvent {
            row: caret.0,
            col: caret.1,
            origin_bounds,
            bounds,
        });
    }

    fn handle_input(&mut self, input: &str) {
        //debug!("on input:{}", input);
        let caret = self.paragraph.get_caret();
        self.insert_text(input, caret, true);
    }

    fn get_text_for_copy(&self) -> String {
        if self.input_type == InputType::Text {
            self.paragraph
                .get_selection_text()
                .unwrap_or_else(String::new)
        } else {
            String::new()
        }
    }

    #[cfg(feature = "clipboard")]
    fn copy(&self) {
        use clipboard::{ClipboardContext, ClipboardProvider};
        let text_for_copy = self.get_text_for_copy();
        if !text_for_copy.is_empty() {
            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
            ctx.set_contents(text_for_copy).unwrap();
        }
    }

    #[cfg(feature = "clipboard")]
    fn cut(&mut self) {
        self.copy();
        self.handle_input("");
    }

    #[cfg(feature = "clipboard")]
    fn paste(&mut self) {
        use clipboard::{ClipboardContext, ClipboardProvider};
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Ok(text) = ctx.get_contents() {
            self.handle_input(&text);
        }
    }

    fn setup_auto_scroll_callback(&mut self) {
        let el = ok_or_return!(self.element.upgrade());
        if let Some(mut p) = el.get_parent() {
            let me = self.as_weak();
            p.scrollable.set_autoscroll_callback(move || {
                let me = me.upgrade().ok()?;
                me.get_caret_pixels_position()
            });
        }
    }

    fn show_menu(&self, x: f32, y: f32) {
        use crate::menu::{Menu, MenuItem, StandardMenuItem};
        let mut menu = Menu::create();
        #[cfg(feature = "clipboard")]
        {
            let (cut_menu, copy_menu) = {
                let text_for_copy = self.get_text_for_copy();
                let is_empty = text_for_copy.is_empty();
                let me_weak = self.as_weak();
                let mut copy_item = StandardMenuItem::new("Copy", move || {
                    if let Ok(me) = me_weak.upgrade() {
                        me.copy();
                    }
                });
                copy_item.set_disabled(is_empty);

                let me_weak = self.as_weak();
                let mut cut_item = StandardMenuItem::new("Cut", move || {
                    if let Ok(mut me) = me_weak.upgrade() {
                        me.cut();
                    }
                });
                cut_item.set_disabled(is_empty);
                (cut_item, copy_item)
            };
            let paste_menu = {
                let content = crate::ext::ext_clipboard::Clipboard::read_text()
                    .ok()
                    .unwrap_or_else(String::new);
                let has_content = !content.is_empty();
                let me_weak = self.as_weak();
                let mut item = StandardMenuItem::new("Paste", move || {
                    if let Ok(mut me) = me_weak.upgrade() {
                        me.paste();
                    }
                });
                item.set_disabled(!has_content);
                item
            };
            menu.add_item(MenuItem::Standard(cut_menu));
            menu.add_item(MenuItem::Standard(copy_menu));
            menu.add_item(MenuItem::Standard(paste_menu));
            menu.add_item(MenuItem::Separator);
        }
        let select_all_menu = {
            let me_weak = self.as_weak();
            let content = self.paragraph.get_text();
            let allow_select_all =
                !content.is_empty() && Some(content) != self.paragraph.get_selection_text();
            let mut item = StandardMenuItem::new("Select All", move || {
                if let Ok(mut me) = me_weak.upgrade() {
                    me.paragraph.select_all();
                }
            });
            item.set_disabled(!allow_select_all);
            item
        };
        menu.add_item(MenuItem::Standard(select_all_menu));

        let el = ok_or_return!(self.element.upgrade());
        if let Some(w) = el.get_window() {
            if let Ok(w) = w.upgrade() {
                w.popup_menu(menu, x, y);
            }
        }

    }

    fn handle_key_down(&mut self, event: &KeyEventDetail) {
        #[cfg(target_os = "macos")]
        let shortcut_modifier = crate::event::KEY_MOD_META;
        #[cfg(not(target_os = "macos"))]
        let shortcut_modifier = crate::event::KEY_MOD_CTRL;
        if event.modifiers == 0 {
            if let Some(nk) = &event.named_key {
                match nk {
                    NamedKey::Backspace => {
                        let end = self.paragraph.get_caret();
                        if self.paragraph.get_selection().is_empty() {
                            if end.0 > 0 || end.1 > 0 {
                                self.move_caret(-1);
                                let start = self.paragraph.get_caret();
                                self.paragraph.select(start, end);
                            }
                        }
                        self.handle_input("");
                    }
                    NamedKey::Enter => {
                        if self.multiple_line {
                            self.handle_input("\n");
                        }
                    }
                    NamedKey::ArrowLeft => {
                        self.move_caret(-1);
                    }
                    NamedKey::ArrowRight => {
                        self.move_caret(1);
                    }
                    NamedKey::ArrowUp => {
                        self.move_caret_vertical(true);
                    }
                    NamedKey::ArrowDown => {
                        self.move_caret_vertical(false);
                    }
                    NamedKey::Space => {
                        self.handle_input(" ");
                    }
                    NamedKey::Tab => {
                        //TODO use \t?
                        self.handle_input("   ");
                    }
                    _ => {}
                }
            } else if let Some(text) = &event.key_str {
                self.handle_input(&text);
            }
        } else if event.modifiers == KEY_MOD_SHIFT {
            if let Some(text) = &event.key_str {
                self.handle_input(&text);
            }
        } else if event.modifiers == shortcut_modifier {
            if let Some(text) = &event.key_str {
                match text.as_str() {
                    #[cfg(feature = "clipboard")]
                    "c" => self.copy(),
                    #[cfg(feature = "clipboard")]
                    "x" => self.cut(),
                    #[cfg(feature = "clipboard")]
                    "v" => self.paste(),
                    "a" => self.select_all(),
                    "z" => {
                        self.undo();
                    }
                    _ => {}
                }
            }
        } else if event.modifiers == KEY_MOD_CTRL | KEY_MOD_SHIFT {
            if let Some(text) = &event.key_str {
                match text.to_lowercase().as_str() {
                    "z" => {
                        self.redo();
                    }
                    _ => {}
                }
            }
        }
    }

    fn select_all(&mut self) {
        self.paragraph.select_all();
    }

    fn move_caret(&mut self, delta: isize) {
        self.paragraph.move_caret(delta);
    }

    fn move_caret_vertical(&mut self, is_up: bool) {
        self.paragraph.move_caret_vertical(is_up);
    }

    fn undo(&mut self) {
        if let Some(op) = self.edit_history.undo() {
            if let Some(insert) = op.insert {
                self.paragraph.select(op.caret, insert.end);
            }
            let delete_content = op.delete.map(|it| it.content).unwrap_or_else(String::new);
            self.insert_text(&delete_content, op.caret, false);
        }
    }

    fn redo(&mut self) {
        if let Some(op) = self.edit_history.redo() {
            if let Some(delete) = op.delete {
                self.paragraph.select(op.caret, delete.end);
            }
            let insert_content = op.insert.map(|it| it.content).unwrap_or_else(String::new);
            self.insert_text(&insert_content, op.caret, false);
        }
    }

}


#[widget]
pub struct Editable {
    // base: Scroll,
    var: EditableVar,
}

#[js_methods]
impl Editable {
    #[js_func]
    pub fn get_text(&self) -> String {
        self.var.paragraph.get_text()
    }

    #[js_func]
    pub fn set_text(&mut self, text: String) {
        let old_text = self.get_text();
        if text != old_text {
            self.var.paragraph.clear();
            let lines = text.split('\n').collect::<Vec<&str>>();
            for ln in lines {
                let ln = ln.trim_line_endings();
                self.var.paragraph.add_line(Self::build_line(ln.to_string()));
            }
            self.var.update_caret_value(TextCoord::new((0, 0)), false);
        }
        self.var.state.lock().unwrap().has_text = !text.is_empty();
        self.el.mark_dirty(true);
    }

    #[js_func]
    pub fn set_placeholder(&mut self, placeholder: String) {
        self.var.placeholder.clear();
        self.var.placeholder.add_line(Self::build_line(placeholder));
        self.el.mark_dirty(true);
    }

    #[js_func]
    pub fn get_placeholder(&self) -> String {
        self.var.placeholder.get_text()
    }

    #[js_func]
    pub fn set_placeholder_style(&mut self, _style: JsValue) {
        //TODO impl
        // self.var.placeholder_element.update_style(style, false);
    }

    #[js_func]
    pub fn set_multiple_line(&mut self, multiple_line: bool) {
        self.var.multiple_line = multiple_line;
        self.var.paragraph.set_text_wrap(multiple_line);
        self.el.mark_dirty(true);
    }

    #[js_func]
    pub fn set_rows(&mut self, rows: u32) {
        self.var.rows = rows;
        self.el.mark_dirty(true);
    }

    #[js_func]
    pub fn set_auto_height(&mut self, value: bool) {
        self.var.auto_height = value;
        self.el.mark_dirty(true);
    }

    #[js_func]
    pub fn set_selection_by_char_offset(&mut self, start: usize, end: usize) {
        if let Some(start_caret) = self.var.paragraph.get_text_coord_by_char_offset(start) {
            if let Some(end_caret) = self.var.paragraph.get_text_coord_by_char_offset(end) {
                self.var.paragraph.select(start_caret, end_caret);
            }
        }
    }

    #[js_func]
    pub fn set_caret_by_char_offset(&mut self, char_offset: usize) {
        if let Some(caret) = self.var.paragraph.get_text_coord_by_char_offset(char_offset) {
            self.var.update_caret_value(caret, false);
        }
    }

    #[js_func]
    pub fn set_type(&mut self, input_type: InputType) {
        match &input_type {
            InputType::Text => {
                self.var.paragraph.set_mask_char(None);
            }
            InputType::Password => {
                self.var.paragraph.set_mask_char(Some('*'));
            }
        }
        self.var.input_type = input_type;
    }

    #[js_func]
    pub fn get_type(&self) -> InputType {
        self.var.input_type.clone()
    }

    #[js_func]
    pub fn is_disabled(&self) -> bool {
        self.var.disabled
    }

    #[js_func]
    pub fn set_disabled(&mut self, disabled: bool) {
        if disabled {
            self.el.set_attribute("disabled".to_string(), "".to_string());
        } else {
            self.el.remove_attribute("disabled".to_string());
        }
    }
    
    pub fn set_max_history(&mut self, max_history: usize) {
        self.var.edit_history.set_max_history(max_history);
    }
    
    pub fn get_max_history(&self) -> usize {
        self.var.edit_history.get_max_history()
    }

    fn caret_tick(state: Arc<Mutex<EditableState>>, mut context: ElementWeak) {
        let mut state = state.lock().unwrap();
        state.caret_visible = !state.caret_visible;
        context.mark_dirty(false);
    }

    fn handle_input(&mut self, input: &str) {
        self.var.handle_input(input)
    }

    pub fn build_line(text: String) -> Vec<TextElement> {
        let unit = TextElement::Text(TextUnit {
            text,
            font_families: None,
            font_size: None,
            color: None,
            text_decoration_line: None,
            weight: None,
            background_color: None,
            style: None,
        });
        vec![unit]
    }

    fn bind_events(&mut self) {
        //TODO do not handle event when disabled
        let me = self.var.as_weak();
        self.el.register_event_listener(FocusEventListener::new(move |_e, _ctx| {
            let mut me = ok_or_return!(me.upgrade());
            me.handle_focus();
        }));

        let me = self.var.as_weak();
        self.el.register_event_listener(BlurEventListener::new(move |_e, _ctx| {
            let mut me = ok_or_return!(me.upgrade());
            me.handle_blur();
        }));

        let me = self.var.as_weak();
        self.el.register_event_listener(TextInputEventListener::new(move |e, _ctx| {
            let mut me = ok_or_return!(me.upgrade());
            let caret = me.paragraph.get_caret();
            me.insert_text(e.0.as_str(), caret, true);
        }));

        let me = self.var.as_weak();
        self.el.register_event_listener(ScrollEventListener::new(move |_e, _ctx| {
            let me = ok_or_return!(me.upgrade());
            me.update_ime();
        }));

        let me = self.var.as_weak();
        self.el.register_event_listener(BoundsChangeEventListener::new(move |_e, _ctx| {
            let me = ok_or_return!(me.upgrade());
            me.update_ime();
        }));

        let me = self.el.as_weak();
        self.el.register_event_listener(MouseLeaveEventListener::new(move |_e, _ctx| {
            let mut me = ok_or_return!(me.upgrade());
            me.set_cursor(Cursor::Icon(CursorIcon::Default));
        }));

        let me = self.var.as_weak();
        self.el.event_registration.set_default_behavior_handler(MouseDownEventListener::new(move |e, ctx| {
            if e.0.button == 2 {
                let me = ok_or_return!(me.upgrade());
                me.show_menu(e.0.window_x, e.0.window_y);
                ctx.propagation_cancelled = true;
            }
        }));

        let me = self.var.as_weak();
        self.el.event_registration.set_default_behavior_handler(KeyDownEventListener::new(move |e, _ctx| {
            let mut me = ok_or_return!(me.upgrade());
            me.handle_key_down(&e.0);
        }));

        let me = self.var.as_weak();
        self.el.event_registration.set_default_behavior_handler(PreeditEventListener::new(move |e, _ctx| {
            let mut var = ok_or_return!(me.upgrade());
            var.handle_input(&e.content);
            if !e.content.is_empty() {
                let end_caret = var.paragraph.get_caret();
                let content_chars_count = e.content.chars_count() as isize;
                if let Some(start_caret) = var.paragraph.calculate_caret(-content_chars_count) {
                    var.paragraph.select(start_caret, end_caret);
                    if let Some(offset) = e.offset {
                        let char_offset = if offset == 0 {
                            0
                        } else {
                            e.content[0..offset].chars_count()
                        } as isize;
                        if char_offset < content_chars_count {
                            var.paragraph.move_caret( char_offset - content_chars_count)
                        }
                    }
                }
            }
        }));
    }

    pub fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(placeholder_styles) = styles.get("placeholder") {
            for style in placeholder_styles {
                match style {
                    ResolvedStyleProp::Color(color) => {
                        self.var.placeholder.set_color(*color);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Editable {
    pub fn new() -> Self {
        let mut ele = Element::new_untagged();
        //TODO register and emit in parent element?
        ele.register_js_event::<CaretChangeEvent>("caretchange");
        ele.set_focusable(true);
        // TODO move to outer element
        ele.allow_ime = true;
        // let mut base = Scroll::create(ele);
        let mut paragraph = TextBox::new();
        paragraph.bind_event(&mut ele);
        let mut placeholder = TextBox::new();
        paragraph.set_text_wrap(false);
        placeholder.set_text_wrap(false);
        let state = Arc::new(Mutex::new(EditableState::default()));

        //TODO support custom style
        placeholder.set_color(Color::from_rgb(80, 80, 80));

        // ele.set_cursor(CursorIcon::Text);
        // base.set_scroll_x(ScrollBarStrategy::Never);
        // base.set_scroll_y(ScrollBarStrategy::Never);

        paragraph.add_line(Self::build_line("".to_string()));
        {
            let state = state.clone();
            let mut element_weak = ele.as_weak();
            paragraph.set_layout_callback(move |has_text| {
                let mut state = state.lock().unwrap();
                state.has_text = has_text;
                element_weak.mark_dirty(true);
            });
        }
        {
            let mut element_weak = ele.as_weak();
            paragraph.set_repaint_callback(move || {
                element_weak.mark_dirty(false);
            });
        }

        // Default style
        let mut editable_state = EditableState::default();
        editable_state.caret_paint.set_stroke_width(2.0);

        let var = EditableVarData {
            paragraph,
            placeholder,
            multiple_line: false,
            line_height: None,
            layout_dirty: true,
            element: ele.as_weak(),
            input_type: InputType::Text,
            //paint_offset: 0f32,
            // text_changed_listener: Vec::new(),
            caret_timer_handle: None,
            align: TextAlign::Left,
            edit_history: EditHistory::new(10),
            rows: 5,
            disabled: false,
            auto_height: true,
            state: state.clone(),
        }.to_ref();
        ele.set_delegate(EditableDelegate {
            state: state.clone(),
            var: var.clone(),
        });
        let mut inst = Editable {
            // base,
            var,
            el: ele,
        };
        inst.set_multiple_line(false);
        {
            let var_weak = inst.var.as_weak();
            inst.var.paragraph.set_caret_change_callback(move || {
                let mut var = ok_or_return!(var_weak.upgrade());
                var.setup_auto_scroll_callback();
                var.emit_caret_change();
                var.update_ime();
                var.element.mark_dirty(false);
            });
        }

        let inst_weak = inst.var.as_weak();
        inst.el.style
            .yoga_node
            .set_measure_func(inst_weak, |entry, params| {
                let default_size = yoga::Size {
                    width: 0.0,
                    height: 0.0,
                };
                if let Ok(mut e) = entry.upgrade() {
                    let width = if e.multiple_line {
                        params.width
                    } else {
                        f32::NAN
                    };
                    let height = params.height;
                    let bounds = Rect::new(0.0, 0.0, width, height);
                    e.layout(&bounds);
                    let (width, height) = e.paragraph.get_size_without_padding();
                    return yoga::Size { width, height };
                }
                default_size
            });

        inst.bind_events();

        inst
    }

    // fn on_event(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
    //     self.handle_event(event, ctx, (0.0, 0.0));
    // }

    // fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
    //     match key {
    //         "disabled" => self.disabled = value.is_some(),
    //         _ => {}
    //     }
    // }
}

impl Widget for Editable {}

impl ElementDelegate for EditableDelegate {
    fn handle_style_changed(&mut self, key: StylePropKey) {
        let element = self.var.element.clone();
        let element = ok_or_return!(element.upgrade());
        match key {
            StylePropKey::FontStyle => {
                self.var.paragraph.set_font_style(element.style.font_style);
            }
            StylePropKey::FontSize => {
                self.var.paragraph.set_font_size(element.style.font_size);
            }
            StylePropKey::LineHeight => {
                self.var.line_height = element.style.line_height;
            }
            StylePropKey::Color => {
                self.var.paragraph.set_color(element.style.color);
                let mut state = self.var.state.lock().unwrap();
                state.caret_paint.set_color(element.style.color);
            }
            StylePropKey::FontWeight => {
                self.var.paragraph.set_font_weight(element.style.font_weight);
            }
            StylePropKey::FontFamily => {
                self.var.paragraph
                    .set_font_families(element.style.font_family.clone());
            }
            _ => {}
        }
    }

    fn render(&mut self) -> RenderFn {
        let state = self.var.state.clone();
        let mut placeholder_renderer = self.var.placeholder.render();
        let mut paragraph_renderer = self.var.paragraph.render();
        RenderFn::new(move |painter| {
            let canvas = painter.canvas;
            let state = state.lock().unwrap();
            let text_render = if state.has_text {
                &mut paragraph_renderer
            } else {
                &mut placeholder_renderer
            };
            canvas.session(move |_| {
                text_render.run(painter);
            });
            canvas.session(move |_| {
                if state.focusing && state.caret_visible {
                    let caret_pos = state.caret_pos;
                    let start = (caret_pos.x, caret_pos.y);
                    let end = (caret_pos.x, caret_pos.bottom());
                    canvas.draw_line(start, end, &state.caret_paint);
                }
            });
        })
    }

    fn before_layout(&mut self) {
        self.var.layout_dirty = true;
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        if self.var.layout_dirty {
            self.var.layout(&bounds);
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::element::common::editable::Editable;
    use crate::string::StringUtils;
    use crate::text::textbox::TextCoord;
    
    #[test]
    fn test_caret() {
        let mut entry = Editable::new();
        entry.set_text("1\n12\n123\n1234".to_string());
        // entry.caret = TextCoord::new((0, 0));
        let expected_carets = vec![
            TextCoord(0, 1),
            TextCoord(1, 0),
            TextCoord(1, 1),
            TextCoord(1, 2),
            TextCoord(2, 0),
            TextCoord(2, 1),
            TextCoord(2, 2),
            TextCoord(2, 3),
            TextCoord(3, 0),
            TextCoord(3, 1),
            TextCoord(3, 2),
            TextCoord(3, 3),
            TextCoord(3, 4),
        ];
        for c in expected_carets {
            entry.var.move_caret(1);
            assert_eq!(entry.var.paragraph.get_caret(), c);
        }
    }

    //TODO error because of missing event loop
    // #[test]
    pub fn test_edit_history() {
        let mut entry = Editable::new();
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
        // entry.paragraph.select(TextCoord(0, text1.chars_count()), TextCoord(0, text1.chars_count() + text2.chars_count()));
        entry.handle_input("");
        assert_eq!(text1, entry.get_text());
        // undo
        entry.var.undo();
        assert_eq!(text_all, entry.get_text());
        assert_eq!(text_all.chars_count(), entry.var.paragraph.get_caret().1);
        entry.var.undo();
        assert_eq!("", entry.get_text());
        assert_eq!(0, entry.var.paragraph.get_caret().1);
    }
}
