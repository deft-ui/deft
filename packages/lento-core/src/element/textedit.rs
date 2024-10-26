use std::cell::RefCell;
use std::rc::Rc;
use anyhow::Error;
use quick_js::JsValue;
use skia_safe::{Canvas, Color, Font, Paint};
use winit::event::KeyEvent;
use crate::base::{ElementEvent, MouseDetail, PropertyValue, Rect, TextAlign, TextChangeDetail, TextUpdateDetail, VerticalAlign};
use crate::canvas_util::CanvasHelper;
use crate::color::parse_hex_color;
use crate::element::container::Container;
use crate::element::{ElementBackend, ElementRef};
use crate::element::entry::Entry;
use crate::event::{TextUpdateEventBind};
use crate::{create_element, js_call_rust, set_style, tree};
use crate::element::scroll::Scroll;

pub struct TextEdit {
    element: ElementRef,
    container_element: ElementRef,
    entry_element: ElementRef,
    base: Scroll,
    line_number_font: Rc<RefCell<Font>>,
}

impl TextEdit {

    fn get_entry_mut(&mut self) -> &mut Entry {
        self.entry_element.get_backend_mut_as::<Entry>()
    }

    fn get_entry(&self) -> &Entry {
        self.entry_element.get_backend_as::<Entry>()
    }

}

impl ElementBackend for TextEdit {
    fn create(mut element: ElementRef) -> Self {
        let mut base = Scroll::create(element.clone());
        let mut container_ele = create_element!(Container, {
            minHeight   =>  "100%",
            paddingLeft =>  "20",
        });

        let mut entry_element = create_element!(Entry, {
            minHeight => "100%",
            borderLeft => "1 #4A4B4E",
            paddingLeft => "2",
        });

        let entry = entry_element.get_backend_mut_as::<Entry>();
        entry.set_multiple_line(true);
        //TODO use width-fixed font?
        let line_number_font = Rc::new(RefCell::new(entry.get_font().clone()));

        tree!(container_ele, [
            entry_element,
        ]);
        base.add_child_view(container_ele.clone(), None);

        let mut update_line_number_width = {
            let mut container_ele = container_ele.clone();
            let line_number_font = line_number_font.clone();
            move |lines: i32| {
                let line_number_width = line_number_font.borrow().size() * lines.to_string().len() as f32;
                set_style!(container_ele, {
                    paddingLeft => &line_number_width.to_string(),
                });
            }
        };
        update_line_number_width(1);

        element.bind_text_update(move |ctx, detail| {
            let mut lines = 1;
            detail.value.chars().for_each(|c| {
                if c == char::from_u32(10).unwrap() {
                    lines += 1;
                }
            });
            update_line_number_width(lines);
        });

        Self {
            entry_element,
            base,
            element,
            line_number_font,
            container_element: container_ele,
        }
    }

    fn get_name(&self) -> &str {
        "TextEdit"
    }

    fn handle_style_changed(&mut self, key: &str) {
        self.get_entry_mut().handle_style_changed(key);
    }

    fn draw(&self, canvas: &Canvas) {
        self.base.draw(canvas);
        canvas.save();
        // canvas.translate((0.0, -self.element.get_scroll_top()));
        let mut paint = Paint::default();
        paint.set_color(parse_hex_color("4A4B4E").unwrap());
        let entry = self.get_entry();
        let font = self.line_number_font.clone();
        entry.with_paragraph(|lines| {
            let line_number_width = self.container_element.get_padding().3;
            let padding_top = self.entry_element.get_relative_bounds(&self.element).y + self.entry_element.get_padding().0;
            let mut line_start = padding_top;
            let height = self.element.get_size().1;
            let mut line_number = 0;
            for p in lines {
                let p_height = p.paragraph.height();
                let y_start = line_start;
                let y_end = y_start + p_height;
                line_start += p_height;
                line_number += 1;

                if y_end < 0.0 {
                    continue;
                } else if y_start > height {
                    break;
                }
                let rect = Rect::new(0.0, y_start, line_number_width, p.get_soft_line_height(0));
                canvas.draw_text(&rect, &line_number.to_string(), &font.borrow(), &paint, TextAlign::Right, VerticalAlign::Middle);
            }
        });
        canvas.restore();
    }

    fn set_property(&mut self, property_name: &str, property_value: JsValue) {
        let base = &mut self.base;
        js_call_rust!("scroll_to_top", f32, base, scroll_to_top, property_name, property_value);
        self.get_entry_mut().set_property(property_name, property_value);
    }

    fn get_property(&mut self, property_name: &str) -> Result<Option<JsValue>, Error> {
        self.get_entry_mut().get_property(property_name)
    }

    fn handle_input(&mut self, _input: &str) {
        self.base.handle_input(_input)
    }

    fn handle_origin_bounds_change(&mut self, _bounds: &Rect) {
        self.base.handle_origin_bounds_change(_bounds);
    }

    fn handle_event(&mut self, _event_type: &str, _event: &mut ElementEvent) {
        self.base.handle_event(_event_type, _event)
    }

    fn add_child_view(&mut self, child: ElementRef, position: Option<u32>) {
        self.base.add_child_view(child, position);
    }

    fn get_children(&self) -> Vec<ElementRef> {
        self.base.get_children()
    }

    fn remove_child_view(&mut self, position: u32) {
        self.base.remove_child_view(position)
    }

    fn handle_event_default_behavior(&mut self,event_type: &str, event: &mut ElementEvent) -> bool {
        self.base.handle_event_default_behavior(event_type, event)
    }
}


