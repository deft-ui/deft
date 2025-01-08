use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use anyhow::{anyhow, Error};
use lento_macros::{js_func, js_methods, mrc_object};
use ordered_float::Float;
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use skia_bindings::{SkPaint_Style, SkPathOp};
use skia_safe::{Canvas, Color, Matrix, Paint, Path, Rect};
use winit::window::CursorIcon;
use yoga::{Direction, Edge, StyleUnit};

use crate::base::{ElementEvent, ElementEventContext, ElementEventHandler, EventContext, EventListener, EventRegistration, ScrollEventDetail};
use crate::border::build_rect_with_radius;
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::entry::Entry;
use crate::element::image::Image;
use crate::element::scroll::Scroll;
use crate::element::text::Text;
use crate::element::textedit::TextEdit;
use crate::event::{DragOverEventListener, BlurEventListener, BoundsChangeEventListener, CaretChangeEventListener, ClickEventListener, DragStartEventListener, DropEventListener, FocusEventListener, FocusShiftEventListener, KeyDownEventListener, KeyUpEventListener, MouseDownEventListener, MouseEnterEvent, MouseEnterEventListener, MouseLeaveEvent, MouseLeaveEventListener, MouseMoveEventListener, MouseUpEventListener, MouseWheelEventListener, ScrollEvent, ScrollEventListener, TextChangeEventListener, TextUpdateEventListener, TouchCancelEventListener, TouchEndEventListener, TouchMoveEventListener, TouchStartEventListener, BoundsChangeEvent, ContextMenuEventListener};
use crate::event_loop::{create_event_loop_callback};
use crate::ext::ext_frame::{VIEW_TYPE_BUTTON, VIEW_TYPE_CONTAINER, VIEW_TYPE_ENTRY, VIEW_TYPE_IMAGE, VIEW_TYPE_LABEL, VIEW_TYPE_SCROLL, VIEW_TYPE_TEXT_EDIT};
use crate::frame::{Frame, FrameWeak, InvalidMode};
use crate::img_manager::IMG_MANAGER;
use crate::js::js_serde::JsValueSerializer;
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;
use crate::resource_table::ResourceTable;
use crate::style::{parse_style_obj, ColorHelper, StyleNode, StyleProp, StylePropKey, StyleTransform};
use crate::{base, bind_js_event_listener, compute_style, js_call, js_call_rust, js_deserialize, js_get_prop, js_serialize, js_weak_value};

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
pub mod paragraph;

use crate as lento;
use crate::js::JsError;
use crate::layout::LayoutRoot;
use crate::paint::{MatrixCalculator, Painter, UniqueRect};
use crate::render::RenderFn;
use crate::style::border_path::BorderPath;

thread_local! {
    pub static NEXT_ELEMENT_ID: Cell<u32> = Cell::new(1);
}

struct ElementJsContext {
    context: JsValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollByOption {
    x: f32,
    y: f32,
}
js_serialize!(ScrollByOption);
js_deserialize!(ScrollByOption);

//TODO rename
pub trait ViewEvent {
    fn allow_bubbles(&self) -> bool;
}

#[js_methods]
impl Element {
    pub fn create<T: ElementBackend + 'static, F: FnOnce(Element) -> T>(backend: F) -> Self {
        let empty_backend = EmptyElementBackend{};
        let inner =  Mrc::new(ElementData::new(empty_backend));
        let mut ele = Self {
            inner,
        };
        let ele_weak = ele.inner.as_weak();
        let weak = ele.as_weak();
        ele.style.bind_element(weak);
        ele.inner.style.on_changed = Some(Box::new(move |key| {
            if let Ok(mut inner) = ele_weak.upgrade() {
                inner.backend.handle_style_changed(key);
            }
        }));
        let ele_weak = ele.inner.as_weak();
        ele.inner.style.animation_renderer = Some(Mrc::new(Box::new(move |styles| {
            if let Ok(inner) = ele_weak.upgrade() {
                let mut el = Element::from_inner(inner);
                el.animation_style_props = styles;
                el.apply_style();
            }
        })));
        let ele_cp = ele.clone();
        // let bk = backend(ele_cp);
        ele.backend = Box::new(backend(ele_cp));
        //ele.backend.bind(ele_cp);
        ele
    }

    pub fn create_shadow(&mut self) {
        self.style = StyleNode::new_with_shadow();
        let weak = self.as_weak();
        self.style.bind_element(weak);
    }

    #[js_func]
    pub fn get_id(&self) -> u32 {
        self.id
    }

    #[js_func]
    pub fn set_draggable(&mut self, draggable: bool) {
        self.draggable = draggable;
    }

    pub fn is_draggable(&self) -> bool {
        self.draggable
    }

    #[js_func]
    pub fn create_by_type(view_type: i32, context: JsValue) -> Result<Element, Error> {
        let mut view = match view_type {
            VIEW_TYPE_CONTAINER => Element::create(Container::create),
            VIEW_TYPE_SCROLL => Element::create(Scroll::create),
            VIEW_TYPE_LABEL => Element::create(Text::create),
            VIEW_TYPE_ENTRY => Element::create(Entry::create),
            VIEW_TYPE_BUTTON => Element::create(Button::create),
            VIEW_TYPE_TEXT_EDIT => Element::create(TextEdit::create),
            VIEW_TYPE_IMAGE => Element::create(Image::create),
            _ => return Err(anyhow!("invalid view_type")),
        };
        view.resource_table.put(ElementJsContext { context });

        Ok(view)
    }

    #[js_func]
    pub fn set_js_context(&mut self, context: JsValue) {
        self.resource_table.put(ElementJsContext { context });
    }


    #[js_func]
    pub fn get_js_context(&self) -> Result<JsValue, Error> {
        let e = self.resource_table.get::<ElementJsContext>()
            .map(|e| e.context.clone())
            .unwrap_or(JsValue::Undefined);
        Ok(e)
    }

    #[js_func]
    pub fn add_child(&mut self, mut child: Element, position: i32) -> Result<(), Error> {
        let position = if position < 0 { None } else { Some(position as u32) };
        self.add_child_view(child, position);
        Ok(())
    }

    #[js_func]
    pub fn remove_child(&mut self, position: u32) -> Result<(), Error> {
        self.remove_child_view(position);
        Ok(())
    }

    #[js_func]
    pub fn add_js_event_listener(&mut self, event_type: String, listener: JsValue) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "click" => ClickEventListener,
            "contextmenu" => ContextMenuEventListener,
            "caretchange" => CaretChangeEventListener,
            "mousedown" => MouseDownEventListener,
            "mousemove" => MouseMoveEventListener,
            "mouseup" => MouseUpEventListener,
            "mouseenter" => MouseEnterEventListener,
            "mouseleave" => MouseLeaveEventListener,
            "keydown" => KeyDownEventListener,
            "keyup" => KeyUpEventListener,
            "mousewheel" => MouseWheelEventListener,
            "textupdate" => TextUpdateEventListener,
            "touchstart" => TouchStartEventListener,
            "touchmove" => TouchMoveEventListener,
            "touchend" => TouchEndEventListener,
            "touchcancel" => TouchCancelEventListener,
            "focus" => FocusEventListener,
            "blur" => BlurEventListener,
            "focusshift" => FocusShiftEventListener,
            "textchange" => TextChangeEventListener,
            "scroll" => ScrollEventListener,
            "dragstart" => DragStartEventListener,
            "dragover" => DragOverEventListener,
            "drop" => DropEventListener,
            "boundschange" => BoundsChangeEventListener,
        );
        Ok(id)
    }

    #[js_func]
    pub fn focus(&mut self) {
        if let Some(mut frame) = self.upgrade_frame() {
            frame.focus(self.clone());
        }
    }

    #[js_func]
    pub fn set_cursor(&mut self, cursor: CursorIcon) {
        self.cursor = cursor;
        self.mark_dirty(false);
    }

    #[js_func]
    pub fn scroll_by(&mut self, option: ScrollByOption) {
        let mut el = self.clone();
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

    #[js_func]
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
        if value != self.scroll_left {
            self.scroll_layer((value - self.scroll_left, 0.0));
            self.mark_dirty(false);
            self.scroll_left = value;
            //TODO emit on layout updated?
            self.emit_scroll_event();
        }
    }

    #[js_func]
    pub fn get_scroll_left(&self) -> f32 {
        self.scroll_left
    }

    #[js_func]
    pub fn get_scroll_top(&self) -> f32 {
        self.scroll_top
    }

    #[js_func]
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
        if value != self.scroll_top {
            self.scroll_layer((0.0, value - self.scroll_top));
            self.mark_dirty(false);
            self.scroll_top = value;
            //TODO emit on layout updated?
            self.emit_scroll_event();
        }
    }

    #[js_func]
    pub fn get_scroll_height(&self) -> f32 {
        self.get_real_content_size().1
    }

    #[js_func]
    pub fn get_scroll_width(&self) -> f32 {
        self.get_real_content_size().0
    }

    fn emit_scroll_event(&mut self) {
        self.emit(ScrollEvent {
            scroll_top: self.scroll_top,
            scroll_left: self.scroll_left,
        });
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

    pub fn set_parent(&mut self, parent: Option<Element>) {
        self.parent = match parent {
            None => None,
            Some(p) => Some(p.as_weak()),
        };
        self.applied_style.clear();
        self.apply_style();

        self.style.update_computed_style(None);
    }

    pub fn set_window(&mut self, window: Option<FrameWeak>) {
        self.window = window;
    }

    pub fn with_window<F: FnOnce(&mut Frame)>(&self, callback: F) {
        if let Some(p) = self.get_parent() {
            return p.with_window(callback);
        } else if let Some(ww) = &self.window {
            ww.upgrade_mut(|w| {
                callback(w);
            });
        }
    }

    pub fn get_frame(&self) -> Option<FrameWeak> {
        if let Some(p) = self.get_parent() {
            return p.get_frame()
        } else if let Some(ww) = &self.window {
            return Some(ww.clone())
        }
        None
    }

    pub fn upgrade_frame(&self) -> Option<Frame> {
        if let Some(f) = self.get_frame() {
            f.upgrade().ok()
        } else {
            None
        }
    }

    pub fn get_parent(&self) -> Option<Element> {
        let p = match &self.parent {
            None => return None,
            Some(p) => p,
        };
        let p = match p.upgrade() {
            Err(_e) => return None,
            Ok(u) => u,
        };
        Some(p)
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let clip_path = self.build_clip_path();
        clip_path.contains((x, y))
    }

    #[js_func]
    pub fn get_size(&self) -> (f32, f32) {
        let layout = self.style.get_layout();
        (layout.width().nan_to_zero(), layout.height().nan_to_zero())
    }

    fn compute_length(&self, length: StyleUnit, parent_length: Option<f32>) -> Option<f32> {
        if let StyleUnit::Point(p) = length {
            Some(p.0)
        } else if let StyleUnit::Percent(p) = length {
            if let Some(parent_size) = parent_length {
                Some(parent_size * p.0)
            } else {
                Some(0.0)
            }
        } else {
            None
        }
    }


    /// bounds relative to parent
    pub fn get_bounds(&self) -> base::Rect {
        let ml = self.style.get_layout();
        base::Rect::from_layout(&ml)
    }

    pub fn apply_transform(&self, mc: &mut MatrixCalculator) {
        if let Some(tf) = &self.style.transform {
            let bounds = self.get_bounds();
            mc.translate((bounds.width / 2.0, bounds.height / 2.0));
            tf.apply(bounds.width, bounds.height, mc);
            mc.translate((-bounds.width / 2.0, -bounds.height / 2.0));
        }
    }

    pub fn get_relative_bounds(&self, target: &Self) -> base::Rect {
        let my_origin_bounds = self.get_origin_bounds();
        let target_origin_bounds = target.get_origin_bounds();
        my_origin_bounds.translate(-target_origin_bounds.x, -target_origin_bounds.y)
    }

    #[js_func]
    pub fn get_real_content_size(&self) -> (f32, f32) {
        let mut content_width = 0.0;
        let mut content_height = 0.0;
        for c in self.get_children() {
            let cb = c.get_bounds();
            content_width = f32::max(content_width, cb.right());
            content_height = f32::max(content_height, cb.bottom());
        }
        let padding = self.style.get_padding();
        (content_width + padding.1, content_height + padding.2)
    }

    /// content bounds relative to self(border box)
    pub fn get_content_bounds(&self) -> base::Rect {
        self.style.get_content_bounds()
    }

    pub fn get_origin_padding_bounds(&self) -> base::Rect {
        let (t, r, b, l) = self.get_border_width();
        let bounds = self.get_origin_bounds();
        base::Rect::new(bounds.x + l, bounds.y + t, bounds.width - l - r, bounds.height - t - b)
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

    pub fn add_child_view(&mut self, mut child: Element, position: Option<u32>) {
        if let Some(p) = child.get_parent() {
            panic!("child({}) has parent({}) already", child.get_id(), p.get_id());
        }
        let pos = {
            let layout = &mut self.style;
            let pos = position.unwrap_or_else(|| layout.child_count());
            layout.insert_child(&mut child.style, pos);
            pos
        };
        self.mark_dirty(true);
        child.set_parent(Some(self.clone()));
        child.set_dirty_state_recurse(true);
        self.children.insert(pos as usize, child);
    }

    pub fn remove_child_view(&mut self, position: u32) {
        let mut c = self.children.remove(position as usize);
        c.set_parent(None);
        let mut ele = self.clone();
        let layout = &mut ele.style;
        layout.remove_child(&mut c.style);
        ele.mark_dirty(true);
    }

    pub fn get_children(&self) -> Vec<Element> {
        self.children.clone()
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
        // self.mark_all_layout_dirty();
        self.on_before_layout_update();
        self.style.calculate_layout(available_width, available_height, Direction::LTR);
        self.on_layout_update();
        self.set_dirty_state_recurse(false);
    }

    pub fn set_border_width(&mut self, width: (f32, f32, f32, f32)) {
        self.style.set_border(Edge::Top, width.0);
        self.style.set_border(Edge::Right, width.1);
        self.style.set_border(Edge::Bottom, width.2);
        self.style.set_border(Edge::Left, width.3);
    }

    pub fn get_border_width(&self) -> (f32, f32, f32, f32) {
        (
            self.style.get_style_border_top().de_nan(0.0),
            self.style.get_style_border_right().de_nan(0.0),
            self.style.get_style_border_bottom().de_nan(0.0),
            self.style.get_style_border_left().de_nan(0.0),
        )
    }

    /// Return the padding of element (order: Top, Right, Bottom, Left)
    pub fn get_padding(&self) -> (f32, f32, f32, f32) {
        self.style.get_padding()
    }

    pub fn set_border_color(&mut self, color: [Color; 4]) {
        self.style.border_color = color;
    }


    pub fn set_background_image(&mut self, src: &str) {
        self.style.background_image = IMG_MANAGER.with(|im| im.get_img(src));
        self.mark_dirty(true);
    }

    #[js_func]
    pub fn set_style(&mut self, style: JsValue) {
        self.set_style_props(parse_style_obj(style))
    }

    pub fn set_style_props(&mut self, styles: Vec<StyleProp>) {
        self.style_props = styles;
        self.apply_style();
    }

    #[js_func]
    pub fn set_hover_style(&mut self, style: JsValue) {
        self.hover_style_props = parse_style_obj(style);
        if self.hover {
            self.apply_style();
        }
    }

    #[js_func]
    pub fn get_bounding_client_rect(&self) -> base::Rect {
        self.get_origin_bounds()
    }

    fn calculate_changed_style<'a>(
        old_style: &'a Vec<StyleProp>,
        new_style: &'a Vec<StyleProp>,
    ) -> Vec<StyleProp> {
        let mut changed_style_props = Vec::new();
        let mut old_style_map = HashMap::new();
        let mut new_style_map = HashMap::new();
        for e in old_style {
            old_style_map.insert(e.name(), e);
        }
        for e in new_style {
            new_style_map.insert(e.name(), e);
        }
        let mut keys = HashSet::new();
        for k in old_style_map.keys() {
            keys.insert(k);
        }
        for k in new_style_map.keys() {
            keys.insert(k);
        }
        for k in keys {
            let old_value = old_style_map.get(k);
            #[allow(suspicious_double_ref_op)]
            let new_value = match new_style_map.get(k) {
                Some(t) => t.clone().clone(),
                None => old_value.unwrap().clone().clone().unset(),
            };
            if old_value != Some(&&new_value) {
                changed_style_props.push(new_value)
            }
        }
        changed_style_props
    }

    fn apply_style(&mut self) {
        let mut style_props = self.style_props.clone();
        if self.hover {
            for v in &self.hover_style_props {
                style_props.push(v.clone());
            }
        }
        for v in &self.animation_style_props {
            style_props.push(v.clone());
        }
        let new_style = style_props;

        let old_style = self.applied_style.clone();
        let mut changed_style_props = Self::calculate_changed_style(&old_style, &new_style);
        // println!("changed style props:{:?}", changed_style_props);

        changed_style_props.iter().for_each(| e | {
            let (repaint, need_layout) = self.style.set_style(e);
            if need_layout || repaint {
                self.mark_dirty(need_layout);
            }
        });
        self.applied_style = new_style;
    }

    pub fn set_style_prop(&mut self, key: StylePropKey, value: &str) {
        if let Some(prop) = StyleProp::parse_value(key, value) {
            self.set_style_props(vec![prop]);
        }
    }

    pub fn set_style_property(&mut self, key: &str, value: &str) {
        if let Some(prop) = StyleProp::parse(key, value) {
            self.set_style_props(vec![prop]);
        }
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

    pub fn register_event_listener<T: 'static, H: EventListener<T, ElementWeak> + 'static>(&mut self, mut listener: H) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    #[js_func]
    pub fn remove_js_event_listener(&mut self, id: u32) {
        self.unregister_event_listener(id);
    }

    pub fn emit<T: ViewEvent + 'static>(&mut self, mut event: T) {
        let mut me = self.clone();
        let callback = create_event_loop_callback(move || {
            let mut ctx = EventContext {
                target: me.as_weak(),
                propagation_cancelled: false,
                prevent_default: false,
            };
            me.handle_event(&mut event, &mut ctx);
            if !ctx.prevent_default {
                let mut e: Box<dyn Any> = Box::new(event);
                me.handle_default_behavior(&mut e, &mut ctx);
            }
        });
        callback.call();
    }

    fn handle_event<T: ViewEvent + 'static>(&mut self, event: &mut T, ctx: &mut EventContext<ElementWeak>) {
        if TypeId::of::<T>() == TypeId::of::<MouseEnterEvent>() {
            self.hover = true;
            if !self.hover_style_props.is_empty() {
                self.apply_style();
            }
        } else if TypeId::of::<T>() == TypeId::of::<MouseLeaveEvent>() {
            self.hover = false;
            if !self.hover_style_props.is_empty() {
                self.apply_style();
            }
        }
        let backend = self.get_backend_mut();
        let mut e: Box<&mut dyn Any> = Box::new(event);
        backend.on_event(e, ctx);
        self.event_registration.emit(event, ctx);
        if event.allow_bubbles() && !ctx.propagation_cancelled {
            if let Some(mut p) = self.get_parent() {
                p.handle_event(event, ctx);
            }
        }
    }

    fn handle_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        if !self.backend.execute_default_behavior(event, ctx) {
            if let Some(mut p) = self.get_parent() {
                p.handle_default_behavior(event, ctx);
            }
        }
    }

    pub fn add_event_listener(&mut self, event_type: &str, handler: Box<ElementEventHandler>) -> u32 {
        self.event_registration.add_event_listener(event_type, handler)
    }

    pub fn bind_event_listener<T: 'static, F: FnMut(&mut ElementEventContext, &mut T) + 'static>(&mut self, event_type: &str, handler: F) -> u32 {
        self.event_registration.bind_event_listener(event_type, handler)
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: u32) {
        self.event_registration.remove_event_listener(&event_type, id)
    }

    fn scroll_layer(&mut self, delta: (f32, f32)) {
        self.with_window(|win| {
            win.scroll_layer(self.id, delta)
        });
    }

    pub fn set_as_layout_root(&mut self, layout_root: Option<Box<dyn LayoutRoot>>) {
        self.layout_root = layout_root;
    }

    pub fn mark_dirty(&mut self, layout_dirty: bool) {
        if layout_dirty && self.style.get_own_context_mut().is_some() {
            self.style.mark_dirty();
        }

        if layout_dirty {
            if let Some(layout_root) = &mut self.layout_root {
                layout_root.mark_layout_dirty();
                self.set_dirty_state_recurse(true);
                return;
            }
            if let Some(mut parent) = self.get_parent() {
                parent.mark_dirty(true);
                return;
            }
            self.set_dirty_state_recurse(true);
            self.with_window(|win| {
                win.invalid_layout();
            });
        } else {
            self.with_window(|win| {
                win.invalid_element(self.id);
            });
        }
    }

    fn set_dirty_state_recurse(&mut self, dirty: bool) {
        if self.layout_dirty != dirty {
            self.layout_dirty = dirty;
            for mut c in self.get_children() {
                if c.layout_root.is_none() {
                    c.set_dirty_state_recurse(dirty);
                }
            }
        }
    }

    pub fn mark_all_layout_dirty(&mut self) {
        self.mark_dirty(true);
        for mut c in self.get_children() {
            c.mark_all_layout_dirty();
        }
    }

    pub fn set_child_decoration(&mut self, decoration: (f32, f32, f32, f32)) {
        self.children_decoration = decoration;
        self.mark_dirty(false);
    }

    pub fn get_children_viewport(&self) -> Option<Rect> {
        //TODO support overflow:visible
        let border = self.get_border_width();
        let children_decoration = self.children_decoration;
        let bounds = self.get_bounds();
        let x = border.3 + children_decoration.3;
        let y = border.0 + children_decoration.0;
        let right = bounds.width - border.1 - children_decoration.1;
        let bottom = bounds.height - border.2 - children_decoration.2;
        Some(Rect::new(x, y, right, bottom))
    }

    //TODO remove?
    pub fn on_before_layout_update(&mut self) {
        for c in &mut self.get_children() {
            c.on_before_layout_update();
        }
    }


    pub fn on_layout_update(&mut self) {
        let origin_bounds = self.get_origin_bounds();
        if let Some(layout_root) = &mut self.layout_root {
            layout_root.on_root_bounds_updated(origin_bounds.to_skia_rect());
        }
        //TODO emit size change
        let origin_bounds = self.get_origin_bounds();
        if origin_bounds != self.rect {
            // Disable bubble
            let mut ctx = EventContext {
                target: self.as_weak(),
                propagation_cancelled: true,
                prevent_default: false,
            };
            let mut event = BoundsChangeEvent {
                origin_bounds: origin_bounds.clone(),
            };
            self.event_registration.emit(&mut event, &mut ctx);
        }
        //TODO performance: maybe not changed?
        //TODO change is_visible?
        if !origin_bounds.is_empty() {
            // self.backend.handle_origin_bounds_change(&origin_bounds);
            for child in &mut self.get_children() {
                child.on_layout_update();
            }
        }
    }

    pub fn create_border_path(&self) -> BorderPath {
        let bounds = self.get_bounds();
        let border_widths = self.get_border_width();
        let border_widths = [border_widths.0, border_widths.1, border_widths.2, border_widths.3];
        BorderPath::new(bounds.width, bounds.height, self.style.border_radius, border_widths)
    }

    pub fn get_origin_border_bounds(&self) -> base::Rect {
        let origin_bounds = self.get_origin_bounds();
        let border = self.get_border_width();
        base::Rect::from_skia_rect(
            Rect::new(
                origin_bounds.x + border.3,
                origin_bounds.y + border.0,
                origin_bounds.right() - border.1,
                origin_bounds.bottom() - border.2,
            )
        )
    }

    pub fn get_content_box_path(&self) -> Path {
        let mut path = Path::new();
        let bounds = self.get_content_bounds();
        path.add_rect(&bounds.to_skia_rect(), None);
        return path;
    }

    pub fn build_clip_path(&self) -> Path {
        let origin_bounds = self.get_origin_bounds();
        let mut clip_path = build_rect_with_radius(self.style.border_radius, origin_bounds.width, origin_bounds.height);
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

impl ElementWeak {
    pub fn emit<T: ViewEvent + 'static>(&self, event: T) {
        if let Ok(mut el) = self.upgrade() {
            el.emit(event);
        }
    }
}

#[mrc_object]
pub struct Element {
    id: u32,
    backend: Box<dyn ElementBackend>,
    parent: Option<ElementWeak>,
    children: Vec<Element>,
    window: Option<FrameWeak>,
    event_registration: EventRegistration<ElementWeak>,
    pub style: StyleNode,
    style_props: Vec<StyleProp>,
    hover_style_props: Vec<StyleProp>,
    animation_style_props: Vec<StyleProp>,
    hover: bool,

    applied_style: Vec<StyleProp>,
    // animation_instance: Option<AnimationInstance>,


    pub scroll_top: f32,
    pub scroll_left: f32,
    pub last_paint_info: Option<PaintInfo>,
    pub invalid_unique_rect: Option<UniqueRect>,
    draggable: bool,
    cursor: CursorIcon,
    rect: base::Rect,
    resource_table: ResourceTable,
    children_decoration: (f32, f32, f32, f32),

    layout_root: Option<Box<dyn LayoutRoot>>,
    layout_dirty: bool,
    //TODO rename
    pub need_snapshot: bool,
}

pub struct PaintInfo {
    pub scroll_left: f32,
    pub scroll_top: f32,
}

js_weak_value!(Element, ElementWeak);


impl ElementData {

    pub fn new<T: ElementBackend + 'static>(backend: T) -> Self {
        let id = NEXT_ELEMENT_ID.get();
        NEXT_ELEMENT_ID.set(id + 1);
        Self {
            id,
            backend: Box::new(backend),
            parent: None,
            window: None,
            event_registration: EventRegistration::new(),
            style: StyleNode::new(),
            style_props: Vec::new(),
            hover_style_props: Vec::new(),
            animation_style_props: Vec::new(),
            applied_style: Vec::new(),
            hover: false,

            invalid_unique_rect: None,
            scroll_top: 0.0,
            scroll_left: 0.0,
            last_paint_info: None,
            draggable: false,
            cursor: CursorIcon::Default,
            rect: base::Rect::empty(),
            resource_table: ResourceTable::new(),
            children_decoration: (0.0, 0.0, 0.0, 0.0),
            children: Vec::new(),
            layout_root: None,
            layout_dirty: true,
            need_snapshot: false,
        }
    }

}

pub struct EmptyElementBackend {

}

impl ElementBackend for EmptyElementBackend {
    fn create(_ele: Element) -> Self {
        Self {}
    }

    fn get_name(&self) -> &str {
        "Empty"
    }

}

pub trait ElementBackend {

    fn create(element: Element) -> Self where Self: Sized;

    fn get_name(&self) -> &str;

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let _ = key;
    }

    fn render(&mut self) -> RenderFn {
        RenderFn::new(|_c| {})
    }

    fn on_event(&mut self, mut event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        let _ = (event, ctx);
    }

    fn execute_default_behavior(&mut self, mut event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
        let _ = (event, ctx);
        false
    }

    fn handle_origin_bounds_change(&mut self, _bounds: &base::Rect) {}

}

