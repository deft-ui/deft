use crate as deft;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use std::string::ToString;
use anyhow::Error;
use clipboard::{ClipboardContext, ClipboardProvider};
use ordered_float::OrderedFloat;
use quick_js::JsValue;
use skia_safe::{Canvas, Color, Font, Paint};
use skia_safe::textlayout::{TextAlign};
use skia_safe::wrapper::NativeTransmutableWrapper;
use winit::keyboard::NamedKey;
use winit::window::CursorIcon;
use yoga::{PositionType, StyleUnit};
use deft_macros::{element_backend, js_methods};
use crate::base::{Callback, CaretDetail, ElementEvent, EventContext, MouseDetail, MouseEventType, Rect, TextChangeDetail, TextUpdateDetail};
use crate::element::{ElementBackend, Element, ElementWeak};
use crate::element::text::{AtomOffset, Text as Label};
use crate::number::DeNan;
use crate::{js_call, match_event, match_event_type, ok_or_return, timer};
use crate::app::AppEvent;
use crate::element::edit_history::{EditHistory, EditOpType};
use crate::element::paragraph::{Paragraph, ParagraphUnit, TextCoord, TextUnit};
use crate::element::scroll::{Scroll, ScrollBarStrategy};
use crate::element::text::text_paragraph::Line;
use crate::event::{KEY_MOD_CTRL, KEY_MOD_SHIFT, KeyEventDetail, MouseDownEvent, MouseUpEvent, MouseMoveEvent, KeyDownEvent, CaretChangeEvent, TextUpdateEvent, TextChangeEvent, FocusEvent, BlurEvent, SelectStartEvent, SelectEndEvent, SelectMoveEvent, TextInputEvent, ClickEvent};
use crate::event_loop::{create_event_loop_callback, create_event_loop_proxy};
use crate::frame::Frame;
use crate::render::RenderFn;
use crate::string::StringUtils;
use crate::style::{StyleProp, StylePropKey, StylePropVal};
use crate::style::StyleProp::{BackgroundColor, Left, MinWidth, Position, Top};
use crate::style::StylePropKey::Height;
use crate::timer::TimerHandle;

const COPY_KEY: &str = "\x03";
const PASTE_KEY: &str = "\x16";
const KEY_BACKSPACE: &str = "\x08";
const KEY_ENTER: &str = "\x0D";

#[element_backend]
pub struct Entry {
    base: Scroll,
    paragraph: Paragraph,
    paragraph_element: Element,
    /// (row_offset, column_offset)
    caret: TextCoord,
    // paint_offset: f32,
    // text_changed_listener: Vec<Box<TextChangeHandler>>,
    caret_visible: Rc<Cell<bool>>,
    caret_timer_handle: Option<TimerHandle>,
    focusing: bool,
    align: TextAlign,
    multiple_line: bool,
    element: ElementWeak,
    vertical_caret_moving_coord_x: f32,
    edit_history: EditHistory,
    rows: u32,
}

pub type TextChangeHandler = dyn FnMut(&str);

#[js_methods]
impl Entry {

    #[js_func]
    pub fn get_text(&self) -> String {
        self.paragraph.get_text()
    }

    #[js_func]
    pub fn set_text(&mut self, text: String) {
        let old_text = self.paragraph.get_text();
        if text != old_text {
            self.paragraph.clear();
            let lines = text.split('\n').collect::<Vec<&str>>();
            for ln in lines {
                let ln = ln.trim_line_endings();
                self.paragraph.add_line(Self::build_line(ln.to_string()));
            }
            self.update_caret_value(TextCoord::new((0, 0)), false);
        }
    }

    pub fn set_align(&mut self, align: TextAlign) {
        self.align = align;
        //self.update_paint_offset(self.context.layout.get_layout_width(), self.context.layout.get_layout_height());
        self.element.clone().mark_dirty(false);
    }

    #[js_func]
    pub fn set_multiple_line(&mut self, multiple_line: bool) {
        self.multiple_line = multiple_line;
        self.paragraph.set_text_wrap(multiple_line);
        // self.base.content_auto_width = !multiple_line;
        // self.base.content_auto_height = multiple_line;
        if multiple_line {
            self.base.set_scroll_y(ScrollBarStrategy::Auto);
        } else {
            self.base.set_scroll_y(ScrollBarStrategy::Never);
        }
        self.update_default_size();
        //self.base.set_text_wrap(multiple_line);
        //self.update_paint_offset(self.context.layout.get_layout_width(), self.context.layout.get_layout_height());
        self.element.clone().mark_dirty(true);
    }

    #[js_func]
    pub fn set_rows(&mut self, rows: u32) {
        self.rows = rows;
        self.update_default_size();
    }

    #[js_func]
    pub fn set_auto_height(&mut self, value: bool) {
        self.base.set_auto_height(value);
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

    // pub fn get_font(&self) -> &Font {
    //     &self.base.get_font()
    // }

    // pub fn get_line_height(&self) -> Option<f32> {
    //     self.base.get_line_height()
    // }

    // pub fn get_computed_line_height(&self) -> f32 {
    //     self.base.get_computed_line_height()
    // }

    // pub fn set_caret(&mut self, atom_offset: usize) {
    //     self.update_caret_value(atom_offset, false);
    // }

    fn update_default_size(&mut self) {
        // if self.multiple_line {
        //     let (_, line_height) = self.paragraph.measure_line(Self::build_line("a".to_string()));
        //     let expected_height = (self.rows as f32 * line_height);
        //     self.base.set_default_height(Some(expected_height));
        // } else {
        //     self.base.set_default_height(None);
        // }
    }

    fn move_caret(&mut self, mut delta: isize) {
        let mut row = self.caret.0;
        let mut col = self.caret.1 as isize;
        loop {
            let lines = self.paragraph.get_lines();
            let line = match lines.get(row) {
                None => return,
                Some(ln) => ln
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

    fn move_caret_vertical(&mut self, is_up: bool) {
        let caret = self.caret;
        let (current_row, current_col) = (self.caret.0, self.caret.1);
        let line_height = match self.paragraph.get_soft_line_height(current_row, current_col) {
            None => return,
            Some(height) => {height}
        };
        let caret_coord = match self.paragraph.get_char_rect(caret) {
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

    fn update_caret_by_offset_coordinate(&mut self, x: f32, y: f32, is_kb_vertical: bool) {
        let text_coord = self.paragraph.get_text_coord_by_pixel_coord((x, y));
        self.update_caret_value(text_coord, is_kb_vertical);
    }

    fn update_caret_value(&mut self, new_caret: TextCoord, is_kb_vertical: bool) {
        if !is_kb_vertical {
            self.vertical_caret_moving_coord_x = 0.0;
        }
        if new_caret != self.caret {
            self.caret = new_caret;
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
            let mut element = ok_or_return!(self.element.upgrade_mut());
            let callback = Callback::new(move || {
                me.emit_caret_change();
            });
            element.with_window(|w| {
                w.request_next_paint_callback(callback);
            });
        }
    }

    fn emit_caret_change(&mut self) {
        let mut element = ok_or_return!(self.element.upgrade_mut());
        let origin_bounds = element.get_origin_bounds();
        let (border_top, _, _, border_left) = element.get_padding();
        let scroll_left = element.get_scroll_left();
        let scroll_top = element.get_scroll_top();

        let caret = self.caret;
        let bounds = match self.paragraph.get_char_rect(caret) {
            None => return,
            Some(rect) => rect.translate(-scroll_left, -scroll_top),
        };
        // bounds relative to entry
        let origin_bounds = bounds
            .translate(origin_bounds.x + border_left, origin_bounds.y + border_top);

        let mut element = ok_or_return!(self.element.upgrade_mut());
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

    fn to_label_position(&self, position: (f32, f32)) -> (f32, f32) {
        let ele = ok_or_return!(self.element.upgrade_mut(), (0.0, 0.0));
        let padding_left = ele.style.yoga_node.get_layout_padding_left().de_nan(0.0);
        let padding_top = ele.style.yoga_node.get_layout_padding_top().de_nan(0.0);
        (position.0 - padding_left, position.1 - padding_top)
    }

    fn handle_blur(&mut self) {
        self.focusing = false;
        self.caret_timer_handle = None;
        self.caret_visible.set(false);
        let mut element = ok_or_return!(self.element.upgrade_mut());
        element.mark_dirty(false);
        if let Some(frame) = element.get_frame() {
            if let Ok(f) = frame.upgrade_mut() {
                let elp = create_event_loop_proxy();
                elp.send_event(AppEvent::HideSoftInput(f.get_id())).unwrap();
            }
        }
    }

    fn handle_key_down(&mut self, event: &KeyEventDetail) {
        if event.modifiers == 0 {
            if let Some(nk) = &event.named_key {
                match nk {
                    NamedKey::Backspace => {
                        let caret = self.caret;
                        if self.paragraph.get_selection().is_none() {
                            if caret.0 > 0 || caret.1 > 0 {
                                self.move_caret(-1);
                                let start = self.caret;
                                self.paragraph.select(start, caret);
                            }
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
                        if let Some(sel) = self.paragraph.get_selection_text() {
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
                        //TODO self.base.set_selection((0, self.get_text().chars().count()))
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
        self.focusing = true;
        // self.emit_caret_change();
        self.caret_visible.set(true);
        self.caret_timer_handle = Some({
            let caret_visible = self.caret_visible.clone();
            let context = self.element.clone();
            timer::set_interval(move || {
                //println!("onInterval");
                Self::caret_tick(caret_visible.clone(), context.clone());
            }, 500)
        });
        let mut element = ok_or_return!(self.element.upgrade_mut());
        element.mark_dirty(false);
        if let Some(frame) = element.get_frame() {
            if let Ok(f) = frame.upgrade_mut() {
                let elp = create_event_loop_proxy();
                elp.send_event(AppEvent::ShowSoftInput(f.get_id())).unwrap();
            }
        }
    }

    fn insert_text(&mut self, input: &str, mut caret: TextCoord, record_history: bool) {
        if let Some((start, end)) = self.paragraph.get_selection() {
            if record_history {
                let text= self.paragraph.get_selection_text().unwrap();
                //TODO self.edit_history.record_delete(begin, &text);
            }

            if start.0 == end.0 {
                let line_text = self.paragraph.get_line_text(start.0).unwrap();
                let left = line_text.substring(0, start.1);
                let right = line_text.substring(end.1, line_text.chars_count());
                let new_text = format!("{}{}", left, right);
                self.paragraph.update_line(caret.0, Self::build_line(new_text));
            } else {
                let first_line = self.paragraph.get_line_text(start.0).unwrap();
                let left = first_line.substring(0, start.1).to_string();
                let last_line = self.paragraph.get_line_text(end.0).unwrap();
                let right = last_line.substring(end.1, last_line.chars_count()).to_string();
                self.paragraph.update_line(start.0, Self::build_line(format!("{}{}", left, right)));
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
                let new_text = format!("{}{}{}", left_str, input , right_str);
                self.paragraph.update_line(caret.0, Self::build_line(new_text));
                TextCoord(caret.0, caret.1 + input.chars_count())
            } else {
                let first_line = format!("{}{}",left_str, unsafe { input_lines.get_unchecked(0) });
                self.paragraph.insert_line(caret.0, Self::build_line(first_line));
                if input_lines.len() > 2 {
                    for i in 1..input_lines.len() - 1 {
                        let line = unsafe { input_lines.get_unchecked(i).to_string() };
                        self.paragraph.insert_line(caret.0 + i, Self::build_line(line));
                    }
                }
                let last_line = format!("{}{}", unsafe { input_lines.get_unchecked(input_lines.len() - 1) }, right_str);
                self.paragraph.update_line(caret.0 + input_lines.len() - 1, Self::build_line(last_line));
                TextCoord(caret.0 + input_lines.len() - 1, input_lines.last().unwrap().chars_count())
            };
            //TODO maybe update caret twice?
            self.update_caret_value(new_caret, false);
        }

        // emit text update
        let text = self.paragraph.get_text().to_string();
        self.element.emit(TextUpdateEvent {
            value: text.clone()
        });

        // emit text change
        self.element.emit(TextChangeEvent {
            value: text,
        });
    }

    fn handle_input(&mut self, input: &str) {
        //println!("on input:{}", input);
        self.insert_text(input, self.caret, true);
    }

    fn build_line(text: String) -> Vec<ParagraphUnit> {
        let unit = ParagraphUnit::Text(TextUnit {
            text,
            font_families: None,
            font_size: None,
            color: None,
            text_decoration_line: None,
            weight: None,
            background_color: None,
        });
        vec![unit]
    }

}

impl ElementBackend for Entry {

    fn create(mut ele: &mut Element) -> Self {
        let mut base = Scroll::create(ele);
        let mut paragraph_element = Element::create(Paragraph::create);
        paragraph_element.set_style_props(vec![
            Position(StylePropVal::Custom(PositionType::Absolute)),
            Left(StylePropVal::Custom(StyleUnit::Point(OrderedFloat(0.0)))),
            Top(StylePropVal::Custom(StyleUnit::Point(OrderedFloat(0.0)))),
            MinWidth(StylePropVal::Custom(StyleUnit::Percent(OrderedFloat(100.0)))),
            // BackgroundColor(StylePropVal::Custom(Color::from_argb(80,80, 80, 80))),
        ]);
        let mut paragraph = paragraph_element.get_backend_as::<Paragraph>().clone();
        paragraph.set_text_wrap(false);
        paragraph.add_line(Self::build_line("".to_string()));
        ele.add_child_view(paragraph_element.clone(), None);
        // base.set_text_wrap(false);
        ele.set_cursor(CursorIcon::Text);
        base.set_scroll_x(ScrollBarStrategy::Never);
        base.set_scroll_y(ScrollBarStrategy::Never);
        //TODO not working
        // let border = "1 #F9F9F9";
        // ele.set_style_prop(StylePropKey::BorderLeft, border);
        // ele.set_style_prop(StylePropKey::BorderRight, border);
        // ele.set_style_prop(StylePropKey::BorderTop, border);
        // ele.set_style_prop(StylePropKey::BorderBottom, border);

        // Default style
        let caret_visible = Rc::new(Cell::new(false));
        let mut inst = EntryData {
            base,
            paragraph,
            paragraph_element,
            caret: TextCoord::new((0, 0)),
            //paint_offset: 0f32,
            // text_changed_listener: Vec::new(),
            caret_visible,
            caret_timer_handle: None,
            focusing: false,
            align: TextAlign::Left,
            multiple_line: false,
            element: ele.as_weak(),
            vertical_caret_moving_coord_x: 0.0,
            edit_history: EditHistory::new(),
            rows: 5,
        }.to_ref();
        inst.set_multiple_line(false);
        inst
    }

    fn get_name(&self) -> &str {
        "Entry"
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        self.base.handle_style_changed(key)
    }

    fn render(&mut self) -> RenderFn {
        let mut element = ok_or_return!(self.element.upgrade_mut(), RenderFn::empty());
        let children = element.get_children();
        //let paint = self.label.get_paint().clone();
        let mut paint = Paint::default();
        paint.set_color(element.style.computed_style.color);
        let scroll_left = element.get_scroll_left();
        let scroll_top = element.get_scroll_top();
        let paragraph_padding = self.paragraph_element.get_padding();


        let mut me = self.clone();
        let caret_rect = match me.paragraph.get_char_rect(self.caret) {
            None => return RenderFn::empty(),
            Some(r) => r,
        };
        let mut base_render_fn = self.base.render();
        let focusing = self.focusing;
        let caret_visible = self.caret_visible.get();
        let x = caret_rect.x - scroll_left + paragraph_padding.1;
        let y = caret_rect.y - scroll_top + paragraph_padding.0;

        RenderFn::new(move |canvas| {
            canvas.save();
            base_render_fn.run(canvas);
            if focusing && caret_visible {
                paint.set_stroke_width(2.0);
                let start = (x, y);
                let end = (x, y + caret_rect.height);
                canvas.draw_line(start, end, &paint);
            }
            canvas.restore();
        })
    }

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        if let Some(e) = event.downcast_ref::<FocusEvent>() {
            self.handle_focus();
        } else if let Some(e) = event.downcast_ref::<BlurEvent>() {
            self.handle_blur();
        } else if let Some(e) = event.downcast_ref::<SelectStartEvent>() {
            self.update_caret_value(TextCoord(e.row, e.col), false);
        } else if let Some(e) = event.downcast_ref::<SelectMoveEvent>() {
            self.update_caret_value(TextCoord(e.row, e.col), false);
        } else if let Some(e) = event.downcast_ref::<TextInputEvent>() {
            self.insert_text(e.0.as_str(), self.caret, true);
        } else if let Some(e) = event.downcast_ref::<ClickEvent>() {
            if !self.paragraph.is_selecting() {
                self.update_caret_by_offset_coordinate(e.0.offset_x, e.0.offset_y, false);
            }
        }
        self.base.on_event(event, ctx)
    }

    fn execute_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
        if let Some(e) = event.downcast_ref::<KeyDownEvent>() {
            self.handle_key_down(&e.0);
            true
        } else {
            self.base.execute_default_behavior(event, ctx)
        }
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.base.handle_origin_bounds_change(bounds);
        //self.update_paint_offset(bounds.width, bounds.height);
    }

    fn accept_style(&mut self, style: &StyleProp) -> bool {
        let key = style.key();
        match key {
            StylePropKey::PaddingTop
            | StylePropKey::PaddingRight
            | StylePropKey::PaddingBottom
            | StylePropKey::PaddingLeft
            => {
                self.paragraph_element.set_style_prop_internal(style.clone());
                return false;
            },
            _ => {

            }
        }
        self.base.accept_style(style)
    }

}

#[test]
fn test_caret() {
    let mut el = Element::create(Entry::create);
    let entry = el.get_backend_mut_as::<Entry>();
    entry.set_text("1\n12\n123\n1234".to_string());
    entry.caret = TextCoord::new((0, 0));
    let expected_carets = vec![
        TextCoord(0, 1),
        TextCoord(1, 0), TextCoord(1, 1), TextCoord(1, 2),
        TextCoord(2, 0), TextCoord(2, 1), TextCoord(2, 2), TextCoord(2, 3),
        TextCoord(3, 0), TextCoord(3, 1), TextCoord(3, 2), TextCoord(3, 3), TextCoord(3, 4),
    ];
    for c in expected_carets {
        entry.move_caret(1);
        assert_eq!(entry.caret, c);
    }
}

#[test]
pub fn test_edit_history() {
    let mut el = Element::create(Entry::create);
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
    entry.paragraph.select(TextCoord(0, text1.chars_count()), TextCoord(0, text1.chars_count() + text2.chars_count()));
    entry.handle_input("");
    assert_eq!(text1, entry.get_text());
    // undo
    entry.undo();
    assert_eq!(text_all, entry.get_text());
    assert_eq!(text_all.chars_count(), entry.caret.1);
    entry.undo();
    assert_eq!("", entry.get_text());
    assert_eq!(0, entry.caret.1);
}
