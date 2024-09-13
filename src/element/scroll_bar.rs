use crate::backend_as_api;
use crate::base::{ElementEvent, MouseDetail, ScrollEventDetail};
use crate::element::container::Container;
use crate::element::{ElementBackend, ElementRef};

const BACKGROUND_COLOR: &str = "#1E1F22";

const INDICATOR_COLOR: &str = "#444446";

backend_as_api!(ScrollBarBackend, ScrollBar, as_scroll_bar, as_scroll_bar_mut);

pub struct ScrollBar {
    base: Container,
    element: ElementRef,
    ///(frame position, scroll_position)
    start_scroll_info: Option<(f32, f32)>,
    indicator_ele: ElementRef,
    // content_length: f32,
    content_container: Option<ElementRef>,
    vertical: bool,
    scroll_bar_width: f32,
    indicator_length: i32,
    indicator_offset: i32,
}

impl ElementBackend for ScrollBar {
    fn create(mut element: ElementRef) -> Self {
        let mut base = Container::create(element.clone());
        element.set_style_property("background", BACKGROUND_COLOR);
        let mut indicator_ele = ElementRef::new(Container::create);
        indicator_ele.set_style_property("background", INDICATOR_COLOR);
        indicator_ele.set_style_property("borderradius", "4");
        base.add_child_view(indicator_ele.clone(), None);
        let mut inst = Self {
            base,
            element,
            start_scroll_info: None,
            indicator_ele,
            // content_length: 0.0,
            content_container: None,
            vertical: true,
            scroll_bar_width: 14.0,
            indicator_length: 0,
            indicator_offset: 0,
        };
        inst.set_vertical(true);
        inst
    }

    fn get_name(&self) -> &str {
        "ScrollBar"
    }

    fn handle_event(&mut self, event_type: &str, event: &mut ElementEvent) {
        match event_type {
            "mousedown" | "mouseup" | "mousemove" | "click" => {
                if let Some(mouse_event) = event.detail.downcast_ref::<MouseDetail>() {
                    match event_type {
                        //TODO support click
                        //"click" => self.handle_click(mouse_event),
                        "mousedown" => self.handle_mouse_down(mouse_event),
                        "mousemove" => self.handle_mouse_move(mouse_event),
                        "mouseup" => self.handle_mouse_up(mouse_event),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn get_children(&self) -> Vec<ElementRef> {
        self.base.get_children()
    }

}

impl ScrollBar {

    pub fn set_content_container(&mut self, content_container: Option<ElementRef>) {
        self.content_container = content_container;
    }

    pub fn set_visible(&mut self, visible: bool) {
        let bar_width = self.scroll_bar_width.to_string();
        let (len, padding) = if visible  {
            (bar_width.as_str(), "2")
        } else {
            ("0", "0")
        };
        if self.vertical {
            self.element.set_style_property("width", len);
            self.element.set_style_property("paddingLeft", padding);
            self.element.set_style_property("paddingRight", padding);
        } else {
            self.element.set_style_property("height", len);
            self.element.set_style_property("paddingTop", padding);
            self.element.set_style_property("paddingBottom", padding);
        }
    }

    pub fn is_visible(&self) -> bool {
        if self.vertical {
            self.element.get_size().0 > 0.0
        } else {
            self.element.get_size().1 > 0.0
        }
    }


    pub fn set_vertical(&mut self, vertical: bool) {
        self.vertical = vertical;
        if vertical {
            self.element.set_style_property("width", "0");
            self.element.set_style_property("height", "100%");
            self.indicator_ele.set_style_property("width", "100%");
        } else {
            self.element.set_style_property("width", "100%");
            self.element.set_style_property("height", "0");
            self.indicator_ele.set_style_property("height", "100%");
        }
    }

    fn update_indicator_style(&mut self, value: f32, offset: f32) {
        // TODO maybe low performant
        if self.vertical {
            self.indicator_ele.set_style_property("height", &value.to_string());
            self.element.set_style_property("paddingTop", &offset.to_string());
        } else {
            self.indicator_ele.set_style_property("width", &value.to_string());
            self.element.set_style_property("paddingLeft", &offset.to_string());
        }
    }

    pub fn update(&mut self, real_content_length: f32, offset: f32) {
        let bar_length = if self.vertical {
            self.element.get_size().1
        } else {
            self.element.get_size().0
        };
        let indicator_length = bar_length / real_content_length * bar_length;
        let indicator_offset = offset / (real_content_length - bar_length) * (bar_length - indicator_length);
        if indicator_length as i32 != self.indicator_length || indicator_offset as i32 != self.indicator_offset {
            self.update_indicator_style(indicator_length, indicator_offset);
            self.indicator_length = indicator_length as i32;
            self.indicator_offset = indicator_offset as i32;
        }
    }
    // }

    fn handle_mouse_down(&mut self, e: &MouseDetail) {
        println!("mouse down");
        let context = self.content_container.clone().unwrap();
        if self.vertical {
            self.start_scroll_info = Some((e.frame_y, context.get_scroll_top()));
        } else {
            self.start_scroll_info = Some((e.frame_x, context.get_scroll_left()));
        }

    }

    fn handle_mouse_move(&mut self, e: &MouseDetail) {
        //println!("mouse move");
        let mut context = self.element.clone();
        let indicator_ele = self.indicator_ele.clone();
        if let Some((start_frame_pos, start_scroll_pos)) = self.start_scroll_info {
            let mouse_move_distance = if self.vertical {
                e.frame_y - start_frame_pos
            } else {
                e.frame_x - start_frame_pos
            };
            // println!("move distance:{}", mouse_move_distance);
            let (width, height) = context.get_size();
            let (indicator_width, indicator_height) = indicator_ele.get_size();
            let (content_width, content_height) = match &self.content_container {
                None => (0.0, 0.0),
                Some(cc) => cc.get_real_content_size()
            };

            let scroll_move_distance = if self.vertical {
                mouse_move_distance / (height - indicator_height) * (content_height - height)
            } else {
                mouse_move_distance / (width - indicator_width) * (content_width - width)
            };
            println!("move distance:{}, {}", scroll_move_distance, start_scroll_pos + scroll_move_distance);
            if let Some(cc) = &mut self.content_container {
                if self.vertical {
                    cc.set_scroll_top(start_scroll_pos + scroll_move_distance);
                    let new_scroll_top = cc.get_scroll_top();
                    let indicator_top = new_scroll_top / content_height * height;
                    self.element.set_style_property("paddingTop", &indicator_top.to_string());
                } else {
                    cc.set_scroll_left(start_scroll_pos + scroll_move_distance);
                    let new_scroll_left = cc.get_scroll_left();
                    let indicator_left = new_scroll_left / content_width * width;
                    self.element.set_style_property("paddingLeft", &indicator_left.to_string());
                }
            }
            context.mark_dirty(false);
        }
    }

    fn handle_mouse_up(&mut self, _e: &MouseDetail) {
        self.start_scroll_info = None;
    }

}
