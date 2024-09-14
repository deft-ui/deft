use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::str::FromStr;
use anyhow::{anyhow, Error};
use ordered_float::{Float};
use quick_js::{JsValue, ResourceValue};
use serde::{Deserialize, Serialize};
use skia_bindings::{SkPaint_Style, SkPathOp};
use skia_safe::{Canvas, Color, Paint, Path, Rect};
use winit::window::CursorIcon;
use yoga::{Direction, Edge, Node};
use crate::{base, inherit_color_prop, js_call, js_call_rust, js_event_bind, js_get_prop};
use crate::base::{CaretDetail, ElementEvent, ElementEventContext, ElementEventHandler, Event, EventRegistration, MouseDetail, ScrollEventDetail, TextChangeDetail};
use crate::border::{build_rect_with_radius, draw_border};
use crate::color::parse_hex_color;
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::entry::Entry;
use crate::element::image::Image;
use crate::element::label::Label;
use crate::element::scroll::Scroll;
use crate::element::textedit::TextEdit;
use crate::event::{DragOverEventDetail, DragStartEventDetail, DropEventDetail, KeyEventDetail, MouseWheelDetail};
use crate::ext::common::create_event_handler;
use crate::frame::{FrameRef, FrameWeak, WeakWindowHandle};
use crate::img_manager::IMG_MANAGER;
use crate::js::js_value_util::{FromJsValue2, ToJsValue2};
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;
use crate::style::{AllStylePropertyKey, ColorHelper, ColorPropValue, ComputedStyle, expand_mixed_style, parse_align, parse_color, parse_direction, parse_display, parse_flex_direction, parse_float, parse_justify, parse_length, parse_overflow, parse_position_type, parse_style, parse_wrap, Style, StyleNode, StylePropertyKey, StylePropertyValue};
use crate::ext::ext_frame::{VIEW_TYPE_BUTTON, VIEW_TYPE_CONTAINER, VIEW_TYPE_ENTRY, VIEW_TYPE_IMAGE, VIEW_TYPE_LABEL, VIEW_TYPE_SCROLL, VIEW_TYPE_TEXT_EDIT, ViewId};

pub mod container;
pub mod entry;
pub mod button;
pub mod scroll;
pub mod textedit;
mod scroll_bar;
pub mod image;
pub mod label;
mod edit_history;
pub mod text;

thread_local! {
    pub static NEXT_ELEMENT_ID: Cell<u32> = Cell::new(1);
}

#[derive(PartialEq)]
pub struct ElementRef {
    inner: Mrc<Element>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollByOption {
    x: f32,
    y: f32,
}

impl ElementRef {
    pub fn new<T: ElementBackend + 'static, F: FnOnce(ElementRef) -> T>(backend: F) -> Self {
        let empty_backend = EmptyElementBackend{};
        let inner =  Mrc::new(Element::new(empty_backend));
        let mut ele = Self {
            inner,
        };
        let ele_weak = ele.inner.as_weak();
        ele.inner.layout.on_changed = Some(Box::new(move |key| {
            if let Some(mut inner) = ele_weak.upgrade() {
                inner.backend.handle_style_changed(key);
            }
        }));
        let ele_cp = ele.clone();
        // let bk = backend(ele_cp);
        ele.backend = Box::new(backend(ele_cp));
        //ele.backend.bind(ele_cp);
        ele
    }

    pub fn create_shadow(&mut self) {
        self.layout = StyleNode::new_with_shadow();
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn set_draggable(&mut self, draggable: bool) {
        self.draggable = draggable;
    }

    pub fn is_draggable(&self) -> bool {
        self.draggable
    }

    pub fn set_property(&mut self, property_name: String, value: JsValue) -> Result<(), Error> {
        js_call!("scrollTop", f32, self, set_scroll_top, property_name, value, Ok(()));
        js_call!("scrollLeft", f32, self, set_scroll_left, property_name, value, Ok(()));
        js_call!("draggable", bool, self, set_draggable, property_name, value, Ok(()));
        js_call!("cursor", CursorIcon, self, set_cursor, property_name, value, Ok(()));
        js_call_rust!("scroll_by", ScrollByOption, self, scroll_by, property_name, value, Ok(()));
        self.get_backend_mut().set_property(&property_name, value);
        Ok(())
    }

    pub fn get_property(&mut self, property_name: String) -> Result<JsValue, Error> {
        js_get_prop!("size", self, get_size, property_name);
        let v = self.backend.get_property(&property_name)?;
        if let Some(s) = v {
            Ok(s)
        } else {
            Err(anyhow!("No property found:{}", property_name))
        }
    }

    pub fn add_child(&mut self, child: ElementRef, position: i32) -> Result<(), Error> {
        let position = if position < 0 { None } else { Some(position as u32) };
        self.backend.add_child_view(child, position);
        Ok(())
    }

    pub fn remove_child(&mut self, position: u32) -> Result<(), Error> {
        self.get_backend_mut().remove_child_view(position);
        Ok(())
    }

    pub fn bind_event(&mut self, e: String, callback: JsValue) -> Result<i32, Error> {
        let event_name = e.as_str();
        let handler = create_event_handler(event_name, callback);
        js_event_bind!(self, "blur", (), event_name, handler);
        js_event_bind!(self, "focus", (), event_name, handler);
        js_event_bind!(self, "click", MouseDetail, event_name, handler);
        js_event_bind!(self, "mousedown", MouseDetail, event_name, handler);
        js_event_bind!(self, "mouseup", MouseDetail, event_name, handler);
        js_event_bind!(self, "mouseenter", MouseDetail, event_name, handler);
        js_event_bind!(self, "mouseleave", MouseDetail, event_name, handler);
        js_event_bind!(self, "mousemove", MouseDetail, event_name, handler);

        js_event_bind!(self, "keydown", KeyEventDetail, event_name, handler);
        js_event_bind!(self, "keyup", KeyEventDetail, event_name, handler);
        js_event_bind!(self, "textchange", TextChangeDetail, event_name, handler);
        js_event_bind!(self, "caretchange", CaretDetail, event_name, handler);
        js_event_bind!(self, "scroll", ScrollEventDetail, event_name, handler);

        js_event_bind!(self, "dragstart", DragStartEventDetail, event_name, handler);
        js_event_bind!(self, "dragover", DragOverEventDetail, event_name, handler);
        js_event_bind!(self, "drop", DropEventDetail, event_name, handler);
        js_event_bind!(self, "mousewheel", MouseWheelDetail, event_name, handler);
        Ok(0)
    }

    pub fn set_cursor(&mut self, cursor: CursorIcon) {
        self.cursor = cursor;
        self.mark_dirty(false);
    }

    pub fn scroll_by(&mut self, option: ScrollByOption) {
        let mut el = self.backend.get_inner_element().unwrap_or(self.clone());
        if option.x != 0.0 {
            el.set_scroll_left(el.scroll_left + option.x);
        }
        if option.y != 0.0 {
            el.set_scroll_top(el.scroll_top + option.y);
        }
    }

    pub fn get_cursor(&self) -> CursorIcon {
        self.cursor
    }

    pub fn set_scroll_left(&mut self, mut value: f32) {
        if value.is_nan() {
            return
        }
        let content_bounds = self.get_content_bounds();
        let width = content_bounds.width;
        if width <= 0.0 {
            return;
        }
        let max_scroll_left = (self.get_real_content_size().0 - width).max(0.0);
        value = value.clamp(0.0, max_scroll_left);
        if self.scroll_left != value {
            self.scroll_left = value;
            self.emit_scroll_event();
            self.mark_dirty(false);
        }
    }

    pub fn get_scroll_left(&self) -> f32 {
        self.scroll_left
    }

    pub fn get_scroll_top(&self) -> f32 {
        self.scroll_top
    }

    pub fn set_scroll_top(&mut self, mut value: f32) {
        if value.is_nan() {
            return
        }
        let content_bounds = self.get_content_bounds();
        let height = content_bounds.height;
        if height <= 0.0 {
            return;
        }
        let max_scroll_top = (self.get_real_content_size().1 - height).max(0.0);
        value = value.clamp(0.0, max_scroll_top);
        if self.scroll_top != value {
            self.scroll_top = value;
            self.emit_scroll_event();
            self.mark_dirty(false);
        }
    }

    fn emit_scroll_event(&mut self) {
        let mut event = ElementEvent::new("scroll", ScrollEventDetail {
            scroll_top: self.scroll_top,
            scroll_left: self.scroll_left,
        }, self.clone());
        self.emit_event("scroll", &mut event);
    }

    pub fn get_backend_as<T>(&self) -> &T {
        unsafe {
            // &*(self as *const dyn Any as *const T)
            &*(self.backend.deref() as *const dyn ElementBackend as *const T)
        }
    }

    pub fn get_backend_mut_as<T>(&mut self) -> &mut T {
        unsafe {
            // &*(self as *const dyn Any as *const T)
            &mut *(self.backend.deref_mut() as *mut dyn ElementBackend as *mut T)
        }
    }

    pub fn with_backend_mut<T, F: FnOnce(&mut T)>(&mut self, callback:F) {
        let bk = self.get_backend_mut_as::<T>();
        callback(bk);
    }

    pub fn get_backend_mut(&mut self) -> &mut Box<dyn ElementBackend> {
        &mut self.backend
    }

    pub fn get_backend(&self) -> &Box<dyn ElementBackend> {
        &self.backend
    }

    pub fn set_parent(&mut self, parent: Option<ElementRef>) {
        self.parent = match parent {
            None => None,
            Some(p) => Some(p.inner.as_weak()),
        };
        self.layout.compute_color();
        self.layout.compute_background_color();
    }

    pub fn set_window(&mut self, window: Option<FrameWeak>) {
        self.window = window;
    }

    pub fn with_window<F: FnOnce(&mut FrameRef)>(&self, callback: F) {
        if let Some(p) = self.get_parent() {
            return p.with_window(callback);
        } else if let Some(ww) = &self.window {
            ww.upgrade_mut(|w| {
                callback(w);
            });
        }
    }

    // pub fn get_window(&self) -> Option<FrameRef> {
    //     if let Some(p) = self.get_parent() {
    //         return p.get_window()
    //     } else if let Some(ww) = &self.window {
    //         return ww.upgrade();
    //     }
    //     None
    // }

    pub fn get_parent(&self) -> Option<ElementRef> {
        let p = match &self.parent {
            None => return None,
            Some(p) => p,
        };
        let inner = match p.upgrade() {
            None => return None,
            Some(u) => u,
        };
        Some(ElementRef {
            inner,
        })
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let clip_path = self.build_clip_path();
        clip_path.contains((x, y))
    }

    pub fn draw_background(&self, canvas: &Canvas) {
        if let Some(img) = &self.layout.background_image {
            canvas.draw_image(img, (0.0, 0.0), Some(&Paint::default()));
        } else if !self.layout.computed_style.background_color.is_transparent() {
            let mut paint = Paint::default();
            let layout = &self.layout;
            let bd_top =  layout.get_style_border_top().de_nan(0.0);
            let bd_right =  layout.get_style_border_right().de_nan(0.0);
            let bd_bottom =  layout.get_style_border_bottom().de_nan(0.0);
            let bd_left =  layout.get_style_border_left().de_nan(0.0);
            let size_layout = layout.get_layout();
            let rect = Rect::new(bd_left, bd_top, size_layout.width() - bd_right, size_layout.height() - bd_bottom);

            paint.set_color(self.layout.computed_style.background_color);
            paint.set_style(SkPaint_Style::Fill);
            canvas.draw_rect(&rect, &paint);
        }
    }

    pub fn draw_border(&self, canvas: &Canvas) {
        let style = &self.layout;
        let paths = self.layout.get_border_paths();
        let color = style.border_color;
        for i in 0..4 {
            let p = &paths[i];
            if !p.is_empty() {
                let mut paint = Paint::default();
                paint.set_style(SkPaint_Style::Fill);
                paint.set_anti_alias(true);
                paint.set_color(color[i]);
                canvas.draw_path(&p, &paint);
            }
        }
    }

    pub fn get_size(&self) -> (f32, f32) {
        let layout = self.layout.get_layout();
        (layout.width().nan_to_zero(), layout.height().nan_to_zero())
    }


    /// bounds relative to parent
    pub fn get_bounds(&self) -> base::Rect {
        let ml = self.layout.get_layout();
        base::Rect::from_layout(&ml)
    }

    pub fn get_relative_bounds(&self, target: &Self) -> base::Rect {
        let my_origin_bounds = self.get_origin_bounds();
        let target_origin_bounds = target.get_origin_bounds();
        my_origin_bounds.translate(-target_origin_bounds.x, -target_origin_bounds.y)
    }

    pub fn get_real_content_size(&self) -> (f32, f32) {
        let mut content_width = 0.0;
        let mut content_height = 0.0;
        for c in self.get_children() {
            let cb = c.get_bounds();
            content_width = f32::max(content_width, cb.right());
            content_height = f32::max(content_height, cb.bottom());
        }
        (content_width, content_height)
    }

    /// content bounds relative to self(border box)
    pub fn get_content_bounds(&self) -> base::Rect {
        self.layout.get_content_bounds()
    }

    pub fn get_origin_content_bounds(&self) -> base::Rect {
        let (t, r, b, l) = self.get_padding();
        let bounds = self.get_origin_bounds();
        base::Rect::new(bounds.x + l, bounds.y + t, bounds.width - l - r, bounds.height - t - b)
    }

    /// bounds relative to root node
    pub fn get_origin_bounds(&self) -> base::Rect {
        let b = self.get_bounds();
        return if let Some(p) = self.get_parent() {
            let pob = p.get_origin_bounds();
            let offset_top = p.scroll_top;
            let offset_left = p.scroll_left;
            let x = pob.x + b.x - offset_left;
            let y = pob.y + b.y - offset_top;
            base::Rect::new(x, y, b.width, b.height)
        } else {
            b
        }
    }

    pub fn add_child_view(&mut self, mut child: ElementRef, position: Option<u32>) {
        self.backend.add_child_view(child, position);
    }

    pub fn remove_child_view(&mut self, position: u32) {
        self.backend.remove_child_view(position)
    }

    pub fn get_children(&self) -> Vec<ElementRef> {
        self.backend.get_children()
    }

    // pub fn get_layout(&self) -> Layout {
    //     let ml = self.layout.get_layout();
    //     return if let Some(p) = self.get_parent() {
    //         let pl = p.get_layout();
    //         let left = pl.left() + ml.left();
    //         let right = left + ml.width();
    //         let top = pl.top() + ml.top();
    //         let bottom = top + ml.height();
    //         Layout::new(left, right, top, bottom, ml.width(), ml.height())
    //     } else {
    //         ml
    //     }
    // }

    pub fn calculate_layout(&mut self, available_width: f32, available_height: f32) {
        // mark all children dirty so that custom measure function could be call
        self.mark_all_layout_dirty();
        self.layout.calculate_layout(available_width, available_height, Direction::LTR);
        self.on_layout_update();
    }

    pub fn set_border_width(&mut self, width: (f32, f32, f32, f32)) {
        self.layout.set_border(Edge::Top, width.0);
        self.layout.set_border(Edge::Right, width.1);
        self.layout.set_border(Edge::Bottom, width.2);
        self.layout.set_border(Edge::Left, width.3);
    }

    pub fn get_border_width(&self) -> (f32, f32, f32, f32) {
        (
            self.layout.get_style_border_top().de_nan(0.0),
            self.layout.get_style_border_right().de_nan(0.0),
            self.layout.get_style_border_bottom().de_nan(0.0),
            self.layout.get_style_border_left().de_nan(0.0),
        )
    }

    pub fn get_padding(&self) -> (f32, f32, f32, f32) {
        (
            self.layout.get_layout_padding_top().de_nan(0.0),
            self.layout.get_layout_padding_right().de_nan(0.0),
            self.layout.get_layout_padding_bottom().de_nan(0.0),
            self.layout.get_layout_padding_left().de_nan(0.0),
        )
    }

    pub fn set_border_color(&mut self, color: [Color; 4]) {
        self.layout.border_color = color;
    }


    pub fn set_background_image(&mut self, src: &str) {
        self.layout.background_image = IMG_MANAGER.with(|im| im.get_img(src));
        self.mark_dirty(true);
    }

    pub fn set_style(&mut self, style: JsValue) {
        self.set_style_props(parse_style(style))
    }

    pub fn set_style_props(&mut self, style: HashMap<AllStylePropertyKey, StylePropertyValue>) {
        self.style_props = style;
        self.apply_style();
    }

    pub fn set_hover_style(&mut self, style: JsValue) {
        self.hover_style_props = parse_style(style);
        if self.hover {
            self.apply_style();
        }
    }

    fn calculate_changed_style<'a>(
        old_style: &'a HashMap<StylePropertyKey, StylePropertyValue>,
        new_style: &'a HashMap<StylePropertyKey, StylePropertyValue>,
    ) -> HashMap<&'a StylePropertyKey, &'a StylePropertyValue> {
        let mut changed_style_props = HashMap::new();
        let mut keys = HashSet::new();
        for k in old_style.keys() {
            keys.insert(k);
        }
        for k in new_style.keys() {
            keys.insert(k);
        }
        for k in keys {
            let old_value = old_style.get(k);
            let new_value = new_style.get(k);
            if old_value != new_value {
                changed_style_props.insert(k, new_value.unwrap_or(&StylePropertyValue::Invalid));
            }
        }
        return changed_style_props;
    }

    fn apply_style(&mut self) {
        let mut style_props = self.style_props.clone();
        if self.hover {
            for (k, v) in &self.hover_style_props {
                style_props.insert(k.clone(), v.clone());
            }
        }
        let new_style = expand_mixed_style(style_props);

        let old_style = self.applied_style.clone();
        let mut changed_style_props = Self::calculate_changed_style(&old_style, &new_style);

        changed_style_props.iter().for_each(|(k, value)| {
            let (repaint, need_layout) = self.layout.set_style(k, value);
            if need_layout || repaint {
                self.mark_dirty(need_layout);
            }
        });
        self.applied_style = new_style;
    }

    pub fn set_style_property(&mut self, name: &str, value: &str) {
        //FIXME remove
        /*
        let mut repaint = true;
        let mut need_layout = true;
        match name.to_lowercase().as_str() {
            "color" => {
                self.layout.color = parse_color(value);
                self.compute_color();
                need_layout = false;
            },
            "background" | "backgroundcolor" => {
                self.layout.background_color = parse_color(value);
                self.compute_background_color();
                need_layout = false;
            }
            "bordertop" => {self.set_border(value, &vec![0])},
            "borderright" => {self.set_border(value, &vec![1])},
            "borderbottom" => {self.set_border(value, &vec![2])},
            "borderleft" => {self.set_border(value, &vec![3])},
            "border" => {self.set_border(value, &vec![0, 1, 2, 3])}
            "display" => {self.layout.set_display(parse_display(value))}
            "width" => self.layout.set_width(parse_length(value)),
            "height" => self.layout.set_height(parse_length(value)),
            "maxwidth" => self.layout.set_max_width(parse_length(value)),
            "maxheight" => self.layout.set_max_height(parse_length(value)),
            "minwidth" => self.layout.set_min_width(parse_length(value)),
            "minheight" => self.layout.set_min_height(parse_length(value)),
            "margintop" => self.layout.set_margin(Edge::Top, parse_length(value)),
            "marginright" => self.layout.set_margin(Edge::Right, parse_length(value)),
            "marginbottom" => self.layout.set_margin(Edge::Bottom, parse_length(value)),
            "marginleft" => self.layout.set_margin(Edge::Left, parse_length(value)),
            "margin" => {
                self.layout.set_margin(Edge::Top, parse_length(value));
                self.layout.set_margin(Edge::Right, parse_length(value));
                self.layout.set_margin(Edge::Bottom, parse_length(value));
                self.layout.set_margin(Edge::Left, parse_length(value));
            },
            "paddingtop" => self.layout.set_padding(Edge::Top, parse_length(value)),
            "paddingright" => self.layout.set_padding(Edge::Right, parse_length(value)),
            "paddingbottom" => self.layout.set_padding(Edge::Bottom, parse_length(value)),
            "paddingleft" => self.layout.set_padding(Edge::Left, parse_length(value)),
            "padding" => {
                self.layout.set_padding(Edge::Top, parse_length(value));
                self.layout.set_padding(Edge::Right, parse_length(value));
                self.layout.set_padding(Edge::Bottom, parse_length(value));
                self.layout.set_padding(Edge::Left, parse_length(value));
            },
            "flex" => self.layout.set_flex(parse_float(value)),
            "flexbasis" => self.layout.set_flex_basis(parse_length(value)),
            "flexgrow" => self.layout.set_flex_grow(parse_float(value)),
            "flexshrink" => self.layout.set_flex_shrink(parse_float(value)),
            "alignself" => self.layout.set_align_self(parse_align(value)),
            "direction" => self.layout.set_direction(parse_direction(value)),
            "position" => self.layout.set_position_type(parse_position_type(value)),
            "overflow" => self.layout.set_overflow(parse_overflow(value)),
            "borderradius" => {
                let value = parse_float(value);
                self.layout.border_radius = [value, value, value, value];
            }
            "bordertopleftradius" => {
                self.layout.border_radius[0] = parse_float(value);
                println!("{:?}", self.layout.border_radius);
            },
            "bordertoprightradius" => self.layout.border_radius[1] = parse_float(value),
            "borderbottomrightradius" => self.layout.border_radius[2] = parse_float(value),
            "borderbottomleftradius" => self.layout.border_radius[3] = parse_float(value),


            "justifycontent" => self.inner_ele_or_self().layout.set_justify_content(parse_justify(value)),
            "flexdirection" => self.inner_ele_or_self().layout.set_flex_direction(parse_flex_direction(value)),
            "aligncontent" => self.inner_ele_or_self().layout.set_align_content(parse_align(value)),
            "alignitems" => self.inner_ele_or_self().layout.set_align_items(parse_align(value)),
            "flexwrap" => self.inner_ele_or_self().layout.set_flex_wrap(parse_wrap(value)),
            "columngap" => self.inner_ele_or_self().layout.set_column_gap(parse_float(value)),
            "rowgap" => self.inner_ele_or_self().layout.set_row_gap(parse_float(value)),
            "gap" => {
                self.inner_ele_or_self().layout.set_column_gap(parse_float(value));
                self.inner_ele_or_self().layout.set_row_gap(parse_float(value));
            },
            //TODO aspectratio , backgroundcolor
            // right
            // top
            // bottom
            // left
            _ => repaint = false,
        }
        if need_layout || repaint {
            self.mark_dirty(need_layout);
        }*/
    }

    fn inner_ele_or_self(&self) -> ElementRef {
        if let Some(e) = self.backend.get_inner_element() {
            e
        } else {
            self.clone()
        }
    }

    pub fn add_event_listener(&mut self, event_type: &str, handler: Box<ElementEventHandler>) -> u32 {
        self.event_registration.add_event_listener(event_type, handler)
    }

    pub fn bind_event_listener<T: 'static, F: FnMut(&mut ElementEventContext, &mut T) + 'static>(&mut self, event_type: &str, handler: F) -> u32 {
        self.event_registration.bind_event_listener(event_type, handler)
    }

    pub fn remove_event_listener(&mut self, event_type: String, id: u32) {
        self.event_registration.remove_event_listener(&event_type, id)
    }

    pub fn emit_event(&mut self, event_type: &str, event: &mut ElementEvent) {
        self.emit_event_internal(event_type, event);
        if !event.context.prevent_default {
            self.handle_event_default_behavior(event_type, event);
        }
    }

    fn emit_event_internal(&mut self, event_type: &str, event: &mut ElementEvent) {
        if event_type == "mouseenter" {
            self.hover = true;
            if !self.hover_style_props.is_empty() {
                self.apply_style();
            }
        } else if event_type == "mouseleave" {
            self.hover = false;
            if !self.hover_style_props.is_empty() {
                self.apply_style();
            }
        }
        let backend = self.get_backend_mut();
        backend.handle_event(event_type, event);
        self.event_registration.emit_event(event_type, event);
        //TODO check bubble-supported
        if !event.context.propagation_cancelled {
            if let Some(mut parent) = self.get_parent() {
                parent.emit_event_internal(event_type, event);
            }
        }
    }

    fn handle_event_default_behavior(&mut self,event_type: &str, event: &mut ElementEvent) {
        let default_behavior = self.backend.handle_event_default_behavior(event_type, event);
        if !default_behavior {
            if let Some(mut parent) = self.get_parent() {
                parent.handle_event_default_behavior(event_type, event);
            }
        }
    }

    pub fn handle_input(&mut self, input: &str) {
        self.backend.handle_input(input);
    }

    pub fn mark_dirty(&mut self, layout_dirty: bool) {
        if layout_dirty && self.layout.get_own_context_mut().is_some() {
            self.layout.mark_dirty();
        }

        self.with_window(|win| {
            win.mark_dirty(layout_dirty);
        });

    }

    pub fn mark_all_layout_dirty(&mut self) {
        self.mark_dirty(true);
        for mut c in self.get_children() {
            c.mark_all_layout_dirty();
        }
    }


    pub fn on_layout_update(&mut self) {
        //TODO emit size change
        let origin_bounds = self.get_origin_bounds();
        //TODO performance: maybe not changed?
        //TODO change is_visible?
        if !origin_bounds.is_empty() {
            self.backend.handle_origin_bounds_change(&origin_bounds);
            for child in &mut self.get_children() {
                child.on_layout_update();
            }
        }
    }

    pub fn get_border_box_path(&self) -> Path {
        let bounds = self.get_bounds();
        build_rect_with_radius(self.layout.border_radius, bounds.width, bounds.height)
    }

    pub fn get_content_box_path(&self) -> Path {
        let mut path = Path::new();
        let bounds = self.get_content_bounds();
        path.add_rect(&bounds.to_skia_rect(), None);
        return path;
    }

    pub fn build_clip_path(&self) -> Path {
        let origin_bounds = self.get_origin_bounds();
        let mut clip_path = build_rect_with_radius(self.layout.border_radius, origin_bounds.width, origin_bounds.height);
        if let Some(p) = self.get_parent() {
            let outer_bounds = p.get_origin_content_bounds();
            let clip_bounds = outer_bounds.intersect(&origin_bounds).translate(-origin_bounds.x, -origin_bounds.y);
            //TODO  why none?
            if let Some(cp) = clip_path.op(&clip_bounds.to_path(), SkPathOp::Intersect) {
                clip_path = cp;
            } else {
                clip_path = Path::new();
            }
        }
        clip_path
    }

}

impl Deref for ElementRef {
    type Target = Element;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ElementRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Clone for ElementRef {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}


pub struct Element {
    id: u32,
    backend: Box<dyn ElementBackend>,
    parent: Option<MrcWeak<Element>>,
    window: Option<FrameWeak>,
    event_registration: EventRegistration<ElementRef>,
    pub layout: StyleNode,
    pub style_props: HashMap<AllStylePropertyKey, StylePropertyValue>,
    pub hover_style_props: HashMap<AllStylePropertyKey, StylePropertyValue>,
    hover: bool,

    applied_style: HashMap<StylePropertyKey, StylePropertyValue>,


    scroll_top: f32,
    scroll_left: f32,
    draggable: bool,
    cursor: CursorIcon,
}


impl Element {

    pub fn new<T: ElementBackend + 'static>(backend: T) -> Self {
        let id = NEXT_ELEMENT_ID.get();
        NEXT_ELEMENT_ID.set(id + 1);
        Self {
            id,
            backend: Box::new(backend),
            parent: None,
            window: None,
            event_registration: EventRegistration::new(),
            layout: StyleNode::new(),
            style_props: HashMap::new(),
            hover_style_props: HashMap::new(),
            applied_style: HashMap::new(),
            hover: false,

            scroll_top: 0.0,
            scroll_left: 0.0,
            draggable: false,
            cursor: CursorIcon::Default,
        }
    }

}

pub struct EmptyElementBackend {

}

impl ElementBackend for EmptyElementBackend {
    fn create(_ele: ElementRef) -> Self {
        Self {}
    }

    fn get_name(&self) -> &str {
        "Empty"
    }

}

pub trait ElementBackend {

    fn create(element: ElementRef) -> Self where Self: Sized;

    fn get_name(&self) -> &str;

    fn handle_style_changed(&mut self, key: &str) {}

    fn draw(&self, _canvas: &Canvas) {

    }

    fn get_inner_element(&self) -> Option<ElementRef> {
        None
    }

    fn set_property(&mut self, _property_name: &str, _property_value: JsValue) {

    }

    fn get_property(&mut self, _property_name: &str) -> Result<Option<JsValue>, Error> {
        Ok(None)
    }

    fn handle_input(&mut self, _input: &str) {}

    fn handle_event_default_behavior(&mut self, event_type: &str, event: &mut ElementEvent) -> bool {
        (event_type, event);
        false
    }

    fn handle_origin_bounds_change(&mut self, _bounds: &base::Rect) {}

    fn handle_event(&mut self, _event_type: &str, _event: &mut ElementEvent) {}

    fn add_child_view(&mut self, child: ElementRef, position: Option<u32>) {
        //panic!("unsupported")
    }
    fn remove_child_view(&mut self, position: u32) {
        //panic!("unsupported")
    }
    fn get_children(&self) -> Vec<ElementRef> {
        Vec::new()
    }
}

// js api
impl ToJsValue2 for ElementRef {
    fn to_js_value(self) -> Result<JsValue, Error> {
        Ok(JsValue::Resource(ResourceValue { resource: Rc::new(RefCell::new(self)) }))
    }
}
impl FromJsValue2 for ElementRef {
    fn from_js_value(value: JsValue) -> Result<Self, Error> {
        if let Some(r) = value.as_resource(|r:&mut ElementRef| r.clone()) {
            Ok(r.clone())
        } else {
            Err(anyhow!("invalid value"))
        }
    }
}

pub fn element_create(view_type: i32) -> Result<ElementRef, Error> {
    let view = match view_type {
        VIEW_TYPE_CONTAINER => ElementRef::new(Container::create),
        VIEW_TYPE_SCROLL => ElementRef::new(Scroll::create),
        VIEW_TYPE_LABEL => ElementRef::new(Label::create),
        VIEW_TYPE_ENTRY => ElementRef::new(Entry::create),
        VIEW_TYPE_BUTTON => ElementRef::new(Button::create),
        VIEW_TYPE_TEXT_EDIT => ElementRef::new(TextEdit::create),
        VIEW_TYPE_IMAGE => ElementRef::new(Image::create),
        _ => return Err(anyhow!("invalid view_type")),
    };
    Ok(view)
}