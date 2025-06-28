use crate as deft;
use crate::app::AppEvent;
use crate::base::{Callback, EventContext, Rect};
use crate::canvas_util::CanvasHelper;
use crate::element::edit_history::{EditHistory, EditOpType};
use crate::element::util::is_form_event;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::{BlurEvent, BoundsChangeEvent, CaretChangeEvent, Event, FocusEvent, KeyDownEvent, KeyEventDetail, MouseDownEvent, MouseLeaveEvent, ScrollEvent, TextChangeEvent, TextInputEvent, TextUpdateEvent, KEY_MOD_CTRL, KEY_MOD_SHIFT};
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
use crate::{ok_or_return, some_or_return, timer};
use deft_macros::{element_backend, js_methods};
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use skia_safe::{Color, Paint};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
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

#[element_backend]
pub struct Editable {
    // base: Scroll,
    paragraph: TextBox,
    placeholder: TextBox,
    input_type: InputType,
    caret_visible: Rc<Cell<bool>>,
    caret_timer_handle: Option<TimerHandle>,
    focusing: bool,
    align: TextAlign,
    multiple_line: bool,
    element: ElementWeak,
    edit_history: EditHistory,
    rows: u32,
    disabled: bool,
    line_height: Option<f32>,
    auto_height: bool,
    layout_dirty: bool,
}

#[js_methods]
impl Editable {
    #[js_func]
    pub fn get_text(&self) -> String {
        self.paragraph.get_text()
    }

    #[js_func]
    pub fn set_text(&mut self, text: String) {
        let old_text = self.get_text();
        if text != old_text {
            self.paragraph.clear();
            let lines = text.split('\n').collect::<Vec<&str>>();
            for ln in lines {
                let ln = ln.trim_line_endings();
                self.paragraph.add_line(Self::build_line(ln.to_string()));
            }
            self.update_caret_value(TextCoord::new((0, 0)), false);
        }
        self.element.mark_dirty(true);
    }

    #[js_func]
    pub fn set_placeholder(&mut self, placeholder: String) {
        self.placeholder.clear();
        self.placeholder.add_line(Self::build_line(placeholder));
        self.element.mark_dirty(true);
    }

    #[js_func]
    pub fn get_placeholder(&self) -> String {
        self.placeholder.get_text()
    }

    #[js_func]
    pub fn set_placeholder_style(&mut self, _style: JsValue) {
        //TODO impl
        // self.placeholder_element.update_style(style, false);
    }

    #[js_func]
    pub fn set_multiple_line(&mut self, multiple_line: bool) {
        self.multiple_line = multiple_line;
        self.paragraph.set_text_wrap(multiple_line);
        self.element.mark_dirty(true);
    }

    #[js_func]
    pub fn set_rows(&mut self, rows: u32) {
        self.rows = rows;
        self.element.mark_dirty(true);
    }

    #[js_func]
    pub fn set_auto_height(&mut self, value: bool) {
        self.auto_height = value;
        self.element.mark_dirty(true);
    }

    #[js_func]
    pub fn set_selection_by_char_offset(&mut self, start: usize, end: usize) {
        if let Some(start_caret) = self.paragraph.get_text_coord_by_char_offset(start) {
            if let Some(end_caret) = self.paragraph.get_text_coord_by_char_offset(end) {
                self.paragraph.select(start_caret, end_caret);
            }
        }
    }

    #[js_func]
    pub fn set_caret_by_char_offset(&mut self, char_offset: usize) {
        if let Some(caret) = self.paragraph.get_text_coord_by_char_offset(char_offset) {
            self.update_caret_value(caret, false);
        }
    }

    #[js_func]
    pub fn set_type(&mut self, input_type: InputType) {
        match &input_type {
            InputType::Text => {
                self.paragraph.set_mask_char(None);
            }
            InputType::Password => {
                self.paragraph.set_mask_char(Some('*'));
            }
        }
        self.input_type = input_type;
    }

    #[js_func]
    pub fn get_type(&self) -> InputType {
        self.input_type.clone()
    }

    #[js_func]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    #[js_func]
    pub fn set_disabled(&mut self, disabled: bool) {
        let mut ele = ok_or_return!(self.element.upgrade());
        if disabled {
            ele.set_attribute("disabled".to_string(), "".to_string());
        } else {
            ele.remove_attribute("disabled".to_string());
        }
    }

    fn get_caret_pixels_position(&self) -> Option<Rect> {
        let element = self.element.upgrade_mut().ok()?;
        let (scroll_left, scroll_top) = element.scrollable.scroll_offset();

        let mut me = self.clone();
        let caret_rect = me.paragraph.get_caret_rect()?;
        let x = caret_rect.x - scroll_left;
        let y = caret_rect.y - scroll_top;
        Some(Rect::new(x, y, 1.0, caret_rect.height))
    }

    fn update_ime(&self) -> Option<()> {
        let pos = self.get_caret_pixels_position()?;
        let el = self.element.upgrade_mut().ok()?;
        let win = el.get_window()?;
        let win = win.upgrade_mut().ok()?;
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
        Some(())
    }

    fn move_caret(&mut self, delta: isize) {
        self.paragraph.move_caret(delta);
    }

    fn move_caret_vertical(&mut self, is_up: bool) {
        self.paragraph.move_caret_vertical(is_up);
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
            let element = ok_or_return!(self.element.upgrade_mut());
            let callback = Callback::new(move || {
                me.update_ime();
                me.emit_caret_change();
            });
            element.with_window(|mut w| {
                w.request_next_paint_callback(callback);
            });
        }
    }

    fn setup_auto_scroll_callback(&mut self) {
        let element = ok_or_return!(self.element.upgrade());
        if let Some(mut p) = element.get_parent() {
            let me = self.as_weak();
            p.scrollable.set_autoscroll_callback(move || {
                let me = me.upgrade().ok()?;
                me.get_caret_pixels_position()
            });
        }
    }

    fn emit_caret_change(&mut self) {
        let element = ok_or_return!(self.element.upgrade_mut());
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

        let element = ok_or_return!(self.element.upgrade_mut());
        element.emit(CaretChangeEvent {
            row: caret.0,
            col: caret.1,
            origin_bounds,
            bounds,
        });
    }

    fn caret_tick(caret_visible: Rc<Cell<bool>>, mut context: ElementWeak) {
        let visible = caret_visible.get();
        caret_visible.set(!visible);
        context.mark_dirty(false);
    }

    fn handle_blur(&mut self) {
        self.focusing = false;
        self.caret_timer_handle = None;
        self.caret_visible.set(false);
        let mut element = ok_or_return!(self.element.upgrade_mut());
        element.mark_dirty(false);
        if let Some(window) = element.get_window() {
            if let Ok(f) = window.upgrade_mut() {
                let elp = create_event_loop_proxy();
                elp.send_event(AppEvent::HideSoftInput(f.get_id())).unwrap();
            }
        }
    }

    fn get_text_for_copy(&self) -> String {
        if self.input_type == InputType::Text {
            self.paragraph.get_selection_text().unwrap_or_else(String::new)
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

    fn select_all(&mut self) {
        self.paragraph.select_all();
    }

    fn handle_key_down(&mut self, event: &KeyEventDetail) {
        if event.modifiers == 0 {
            if let Some(nk) = &event.named_key {
                match nk {
                    NamedKey::Backspace => {
                        let end = self.paragraph.get_caret();
                        if self.paragraph.get_selection().is_none() {
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
        } else if event.modifiers == KEY_MOD_CTRL {
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
        }
    }

    fn undo(&mut self) {
        if let Some(op) = &self.edit_history.undo() {
            match op.op {
                EditOpType::Insert => {
                    //TODO self.insert_text(op.content.as_str(), op.caret, false);
                }
                EditOpType::Delete => {
                    //TODO self.base.select(op.caret, op.caret + op.content.chars_count());
                    //TODO self.insert_text("", op.caret, false);
                }
            }
        }
    }

    fn handle_focus(&mut self) {
        let _ = self.update_ime();
        self.focusing = true;
        // self.emit_caret_change();
        self.caret_visible.set(true);
        self.caret_timer_handle = Some({
            let caret_visible = self.caret_visible.clone();
            let context = self.element.clone();
            timer::set_interval(
                move || {
                    //debug!("onInterval");
                    Self::caret_tick(caret_visible.clone(), context.clone());
                },
                500,
            )
        });
        let mut element = ok_or_return!(self.element.upgrade_mut());
        element.mark_dirty(false);
        if let Some(window) = element.get_window() {
            if let Ok(f) = window.upgrade_mut() {
                let elp = create_event_loop_proxy();
                elp.send_event(AppEvent::ShowSoftInput(f.get_id())).unwrap();
            }
        }
    }

    fn insert_text(&mut self, input: &str, mut caret: TextCoord, record_history: bool) {
        if let Some((start, end)) = self.paragraph.get_selection() {
            if record_history {
                // let text= self.paragraph.get_selection_text().unwrap();
                //TODO self.edit_history.record_delete(begin, &text);
            }

            if start.0 == end.0 {
                let line_text = self.paragraph.get_line_text(start.0).unwrap();
                let left = line_text.substring(0, start.1);
                let right = line_text.substring(end.1, line_text.chars_count());
                let new_text = format!("{}{}", left, right);
                self.paragraph
                    .update_line(caret.0, Self::build_line(new_text));
            } else {
                let first_line = self.paragraph.get_line_text(start.0).unwrap();
                let left = first_line.substring(0, start.1).to_string();
                let last_line = self.paragraph.get_line_text(end.0).unwrap();
                let right = last_line
                    .substring(end.1, last_line.chars_count())
                    .to_string();
                self.paragraph
                    .update_line(start.0, Self::build_line(format!("{}{}", left, right)));
                if end.0 > start.0 {
                    for _ in start.0..end.0 {
                        self.paragraph.delete_line(start.0 + 1);
                    }
                }
            }
            self.paragraph.unselect();
            self.update_caret_value(start, false);
            caret = start;
        }
        if !input.is_empty() {
            if record_history {
                //TODO self.edit_history.record_input(caret, input);
            }
            let line_text = self.paragraph.get_line_text(caret.0).unwrap();
            let left_str = line_text.substring(0, caret.1);
            let right_str = line_text.substring(caret.1, line_text.len() - caret.1);
            let input_lines = input.split('\n').collect::<Vec<&str>>();
            let new_caret = if input_lines.len() == 1 {
                let new_text = format!("{}{}{}", left_str, input, right_str);
                self.paragraph
                    .update_line(caret.0, Self::build_line(new_text));
                TextCoord(caret.0, caret.1 + input.chars_count())
            } else {
                let first_line = format!("{}{}", left_str, unsafe { input_lines.get_unchecked(0) });
                self.paragraph
                    .insert_line(caret.0, Self::build_line(first_line));
                if input_lines.len() > 2 {
                    for i in 1..input_lines.len() - 1 {
                        let line = unsafe { input_lines.get_unchecked(i).to_string() };
                        self.paragraph
                            .insert_line(caret.0 + i, Self::build_line(line));
                    }
                }
                let last_line = format!(
                    "{}{}",
                    unsafe { input_lines.get_unchecked(input_lines.len() - 1) },
                    right_str
                );
                self.paragraph
                    .update_line(caret.0 + input_lines.len() - 1, Self::build_line(last_line));
                TextCoord(
                    caret.0 + input_lines.len() - 1,
                    input_lines.last().unwrap().chars_count(),
                )
            };
            //TODO maybe update caret twice?
            self.update_caret_value(new_caret, false);
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

    fn handle_input(&mut self, input: &str) {
        //debug!("on input:{}", input);
        self.insert_text(input, self.paragraph.get_caret(), true);
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

    pub fn handle_event(
        &mut self,
        event: &mut Event,
        ctx: &mut EventContext<ElementWeak>,
        scroll_offset: (f32, f32),
    ) {
        if self.disabled && is_form_event(&event) {
            ctx.propagation_cancelled = true;
            return;
        }

        let (offset_x, offset_y) = scroll_offset;
        if self.paragraph.on_event(&event, ctx, offset_x, offset_y) {
            return;
        }
        if let Some(_e) = FocusEvent::cast(event) {
            self.handle_focus();
        } else if let Some(_e) = BlurEvent::cast(event) {
            self.handle_blur();
        } else if let Some(e) = TextInputEvent::cast(event) {
            self.insert_text(e.0.as_str(), self.paragraph.get_caret(), true);
        } else if let Some(_e) = ScrollEvent::cast(event) {
            //TODO update later?
            let _ = self.update_ime();
        } else if let Some(_e) = BoundsChangeEvent::cast(event) {
            //TODO update later?
            let _ = self.update_ime();
        } else if let Some(_e) = MouseLeaveEvent::cast(event) {
            let mut el = ok_or_return!(self.element.upgrade());
            el.set_cursor(Cursor::Icon(CursorIcon::Default));
        }
    }

    pub(crate) fn on_execute_default_behavior(&mut self, event: &mut Event) -> bool {
        #[cfg(feature = "clipboard")]
        if let Some(_e) = MouseDownEvent::cast(event) {
            if _e.0.button == 2 {
                self.show_menu(_e.0.window_x, _e.0.window_y);
                return true;
            }
        }
        if let Some(e) = KeyDownEvent::cast(event) {
            self.handle_key_down(&e.0);
        }
        false
    }

    #[cfg(feature = "clipboard")]
    fn show_menu(&self, x: f32, y: f32) {
        use crate::menu::{Menu, MenuItem, StandardMenuItem};
        let mut menu = Menu::new();
        let (cut_menu, copy_menu) = {
            let text_for_copy = self.get_text_for_copy();
            let is_empty = text_for_copy.is_empty();
            let me_weak = self.as_weak();
            let mut copy_item = StandardMenuItem::new("Copy", move || {
                if let Ok(me) = me_weak.upgrade_mut() {
                    me.copy();
                }
            });
            copy_item.set_disabled(is_empty);

            let me_weak = self.as_weak();
            let mut cut_item = StandardMenuItem::new("Cut", move || {
                if let Ok(mut me) = me_weak.upgrade_mut() {
                    me.cut();
                }
            });
            cut_item.set_disabled(is_empty);
            (cut_item, copy_item)
        };
        let paste_menu = {
            let content = crate::ext::ext_clipboard::Clipboard::read_text().ok().unwrap_or_else(String::new);
            let has_content = !content.is_empty();
            let me_weak = self.as_weak();
            let mut item = StandardMenuItem::new("Paste", move || {
                if let Ok(mut me) = me_weak.upgrade_mut() {
                    me.paste();
                }
            });
            item.set_disabled(!has_content);
            item
        };
        let select_all_menu = {
            let me_weak = self.as_weak();
            let content = self.paragraph.get_text();
            let allow_select_all = !content.is_empty() && Some(content) != self.paragraph.get_selection_text();
            let mut item = StandardMenuItem::new("Select All", move || {
                if let Ok(mut me) = me_weak.upgrade_mut() {
                    me.paragraph.select_all();
                }
            });
            item.set_disabled(!allow_select_all);
            item
        };
        menu.add_item(MenuItem::Standard(cut_menu));
        menu.add_item(MenuItem::Standard(copy_menu));
        menu.add_item(MenuItem::Standard(paste_menu));
        menu.add_item(MenuItem::Separator);
        menu.add_item(MenuItem::Standard(select_all_menu));

        if let Ok(e) = self.element.upgrade_mut() {
            if let Some(w) = e.get_window() {
                if let Ok(w) = w.upgrade_mut() {
                    w.popup_menu(menu, x, y);
                }
            }
        }
    }
}

impl ElementBackend for Editable {
    fn create(ele: &mut Element) -> Self {
        //TODO register and emit in parent element?
        ele.register_js_event::<CaretChangeEvent>("caretchange");
        ele.set_focusable(true);
        // let mut base = Scroll::create(ele);
        let mut paragraph = TextBox::new();
        let mut placeholder = TextBox::new();
        paragraph.set_text_wrap(false);
        placeholder.set_text_wrap(false);

        //TODO support custom style
        placeholder.set_color(Color::from_rgb(80, 80, 80));

        // ele.set_cursor(CursorIcon::Text);
        // base.set_scroll_x(ScrollBarStrategy::Never);
        // base.set_scroll_y(ScrollBarStrategy::Never);

        paragraph.add_line(Self::build_line("".to_string()));
        {
            let mut element_weak = ele.as_weak();
            paragraph.set_layout_callback(move || {
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
        let caret_visible = Rc::new(Cell::new(false));

        let mut inst = EditableData {
            // base,
            paragraph,
            placeholder,
            input_type: InputType::Text,
            //paint_offset: 0f32,
            // text_changed_listener: Vec::new(),
            caret_visible,
            caret_timer_handle: None,
            focusing: false,
            align: TextAlign::Left,
            multiple_line: false,
            element: ele.as_weak(),
            edit_history: EditHistory::new(),
            rows: 5,
            disabled: false,
            line_height: None,
            auto_height: true,
            layout_dirty: true,
        }
        .to_ref();
        inst.set_multiple_line(false);
        {
            let weak = inst.as_weak();
            inst.paragraph.set_caret_change_callback(move || {
                let mut entry = ok_or_return!(weak.upgrade());
                entry.setup_auto_scroll_callback();
                entry.emit_caret_change();
                entry.element.mark_dirty(false);
            });
        }
        ele.style
            .yoga_node
            .set_measure_func(inst.as_weak(), |entry, params| {
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
        inst
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let element = ok_or_return!(self.element.upgrade());
        match key {
            StylePropKey::FontStyle => {
                self.paragraph.set_font_style(element.style.font_style);
            }
            StylePropKey::FontSize => {
                self.paragraph.set_font_size(element.style.font_size);
            }
            StylePropKey::LineHeight => {
                self.line_height = element.style.line_height;
            }
            StylePropKey::Color => {
                self.paragraph.set_color(element.style.color);
            }
            StylePropKey::FontWeight => {
                self.paragraph.set_font_weight(element.style.font_weight);
            }
            StylePropKey::FontFamily => {
                self.paragraph
                    .set_font_families(element.style.font_family.clone());
            }
            _ => {}
        }
    }

    fn render(&mut self) -> RenderFn {
        let element = ok_or_return!(self.element.upgrade(), RenderFn::empty());
        let mut paint = Paint::default();
        paint.set_color(element.style.color);

        let focusing = self.focusing;
        let caret_visible = self.caret_visible.get();

        let caret_pos = some_or_return!(self.get_caret_pixels_position(), RenderFn::empty());
        let text_render = if self.get_text().is_empty() {
            self.placeholder.render()
        } else {
            self.paragraph.render()
        };

        RenderFn::new(move |painter| {
            let canvas = painter.canvas;
            canvas.session(|_| {
                text_render.run(painter);
            });
            canvas.session(|_| {
                if focusing && caret_visible {
                    paint.set_stroke_width(2.0);
                    let start = (caret_pos.x, caret_pos.y);
                    let end = (caret_pos.x, caret_pos.bottom());
                    canvas.draw_line(start, end, &paint);
                }
            });
        })
    }

    fn on_event(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
        self.handle_event(event, ctx, (0.0, 0.0));
    }

    fn execute_default_behavior(&mut self, event: &mut Event, _ctx: &mut EventContext<ElementWeak>) -> bool {
        self.on_execute_default_behavior(event)
    }

    fn before_layout(&mut self) {
        self.layout_dirty = true;
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        if self.layout_dirty {
            self.layout(bounds);
        }
    }

    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(placeholder_styles) = styles.get("placeholder") {
            for style in placeholder_styles {
                match style {
                    ResolvedStyleProp::Color(color) => {
                        self.placeholder.set_color(*color);
                    }
                    _ => {}
                }
            }
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        match key {
            "disabled" => self.disabled = value.is_some(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::element::common::editable::Editable;
    use crate::element::{Element, ElementBackend};
    use crate::string::StringUtils;
    use crate::text::textbox::TextCoord;
    use measure_time::print_time;

    #[test]
    fn test_performance() {
        let mut entry_el = Element::create(Editable::create);
        let mut entry_el2 = entry_el.clone();
        let entry = entry_el.get_backend_mut_as::<Editable>();
        entry.set_text(include_str!("../../../Cargo.lock").to_string().repeat(10));
        {
            print_time!("layout time");
            entry_el2.calculate_layout(1000.0, 100.0);
        }

        print_time!("render paragraph");
        // entry.paragraph.render();
    }

    #[test]
    fn test_caret() {
        let mut el = Element::create(Editable::create);
        let entry = el.get_backend_mut_as::<Editable>();
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
            entry.move_caret(1);
            assert_eq!(entry.paragraph.get_caret(), c);
        }
    }

    //TODO error because of missing event loop
    // #[test]
    pub fn test_edit_history() {
        let mut el = Element::create(Editable::create);
        let entry = el.get_backend_mut_as::<Editable>();
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
        entry.undo();
        assert_eq!(text_all, entry.get_text());
        assert_eq!(text_all.chars_count(), entry.paragraph.get_caret().1);
        entry.undo();
        assert_eq!("", entry.get_text());
        assert_eq!(0, entry.paragraph.get_caret().1);
    }
}
