use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::{Deref, DerefMut};

use anyhow::Error;
use bitflags::bitflags;
use deft_macros::{js_methods, mrc_object};
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use skia_safe::Rect;
use winit::window::{Cursor, CursorIcon};
use yoga::{Direction, StyleUnit};

use crate::base::{
    BoxJsEventListenerFactory, EventContext, EventListener, EventRegistration, JsEvent,
};
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::image::Image;
use crate::element::scroll::{Scroll, ScrollBarStrategy};
use crate::event::{
    BlurEventListener, BoundsChangeEvent, BoundsChangeEventListener, ClickEventListener,
    ContextMenuEventListener, DragOverEventListener, DragStartEventListener, DropEventListener,
    DroppedFileEventListener, Event, FocusEventListener, FocusShiftEventListener,
    HoveredFileEventListener, KeyDownEventListener, KeyUpEventListener, MouseDownEvent,
    MouseDownEventListener, MouseEnterEvent, MouseEnterEventListener, MouseLeaveEvent,
    MouseLeaveEventListener, MouseMoveEventListener, MouseUpEventListener, MouseWheelEventListener,
    ScrollEvent, ScrollEventListener, TextChangeEventListener, TextUpdateEventListener,
    TouchCancelEventListener, TouchEndEventListener, TouchMoveEventListener, TouchStartEvent,
    TouchStartEventListener,
};
use crate::event_loop::create_event_loop_callback;
use crate::mrc::Mrc;
use crate::number::DeNan;
use crate::resource_table::ResourceTable;
use crate::style::{FixedStyleProp, ResolvedStyleProp, StyleNode, StylePropKey, StylePropVal};
use crate::window::{Window, WindowWeak};
use crate::{
    base, bind_js_event_listener, js_auto_upgrade, js_deserialize, js_serialize, js_value,
    ok_or_return,
};

pub mod body;
pub mod button;
pub mod checkbox;
pub mod common;
pub mod container;
mod edit_history;
pub mod entry;
mod font_manager;
pub mod image;
pub mod label;
pub mod paragraph;
pub mod radio;
pub mod richtext;
pub mod scroll;
pub mod select;
pub mod text;
pub mod textedit;
pub mod textinput;
pub mod util;

use crate as deft;
use crate::computed::ComputedValue;
use crate::element::body::Body;
use crate::element::checkbox::Checkbox;
use crate::element::common::scrollable::Scrollable;
use crate::element::entry::Entry;
use crate::element::label::Label;
use crate::element::paragraph::Paragraph;
use crate::element::radio::{Radio, RadioGroup};
use crate::element::richtext::RichText;
use crate::element::select::Select;
use crate::element::textedit::TextEdit;
use crate::element::textinput::TextInput;
use crate::element::util::is_form_event;
use crate::event::event_emitter::EventEmitter;
use crate::js::JsError;
use crate::paint::MatrixCalculator;
use crate::render::RenderFn;
use crate::style::border_path::BorderPath;
use crate::style::css_manager::CssManager;
use crate::style::length::LengthContext;
use crate::style::style_vars::StyleVars;
use crate::style::styles::Styles;
use crate::style_list::StyleList;

type BackendCreator = Box<dyn FnMut(&mut Element) -> Box<dyn ElementBackend + 'static>>;

thread_local! {
    pub static NEXT_ELEMENT_ID: Cell<u32> = Cell::new(1);
    pub static STYLE_VARS: ComputedValue<String> = ComputedValue::new();
    pub static CSS_MANAGER: RefCell<CssManager> = RefCell::new(CssManager::new());
    pub static ELEMENT_CREATORS: RefCell<HashMap<String, BackendCreator>> = RefCell::new(HashMap::new());
}

bitflags! {

    struct StyleDirtyFlags: u8 {
        const SelfDirty = 0b1;
        const ChildrenDirty = 0b10;
    }

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

pub fn register_component<T: ElementBackend>(tag: &str) {
    let tag = tag.to_string();
    let bc: BackendCreator = Box::new(move |ele| Box::new(T::create(ele)));
    ELEMENT_CREATORS.with_borrow_mut(move |map| {
        map.insert(tag, bc);
    })
}

pub fn init_base_components() {
    register_component::<Container>("container");
    register_component::<Scroll>("scroll");
    register_component::<Button>("button");
    register_component::<Checkbox>("checkbox");
    register_component::<Radio>("radio");
    register_component::<Image>("image");
    register_component::<Label>("label");
    register_component::<Paragraph>("paragraph");
    register_component::<Entry>("entry");
    register_component::<TextInput>("text-input");
    register_component::<TextEdit>("text-edit");
    register_component::<Body>("body");
    register_component::<RadioGroup>("radio-group");
    register_component::<RichText>("rich-text");
    register_component::<Select>("select");
    register_component::<Container>("dialog");
    register_component::<Container>("dialog-title");
}

#[js_methods]
impl Element {
    pub fn new(backend_creator: &mut BackendCreator) -> Self {
        let empty_backend = EmptyElementBackend {};
        let inner = Mrc::new(ElementData::new(empty_backend));
        let mut ele = Self { inner };
        let weak = ele.as_weak();
        ele.style.bind_element(weak);
        let ele_weak = ele.inner.as_weak();
        // let bk = backend(ele_cp);
        ele.backend = Mrc::new(backend_creator(&mut ele));
        ele.style.on_changed = Some(Box::new(move |key| {
            if let Ok(mut inner) = ele_weak.upgrade() {
                inner.backend.handle_style_changed(key);
            }
        }));

        {
            let el = ele.as_weak();
            ele.scrollable.horizontal_bar.set_scroll_callback(move |_| {
                let mut el = ok_or_return!(el.upgrade());
                el.mark_dirty(false);
                el.emit_scroll_event();
            });
        }
        {
            let el = ele.as_weak();
            ele.scrollable.vertical_bar.set_scroll_callback(move |_| {
                let mut el = ok_or_return!(el.upgrade());
                el.mark_dirty(false);
                el.emit_scroll_event();
            });
        }
        let weak = ele.as_weak();
        ele.style.bind_element(weak);
        //ele.backend.bind(ele_cp);
        ele
    }

    pub fn create<T: ElementBackend + 'static, F: 'static + FnMut(&mut Element) -> T>(
        mut backend: F,
    ) -> Self {
        let mut backend_creator: BackendCreator = Box::new(move |ele| Box::new(backend(ele)));
        Self::new(&mut backend_creator)
    }

    #[js_func]
    pub fn get_eid(&self) -> u32 {
        self.id
    }

    #[js_func]
    pub fn get_class(&self) -> String {
        let classes: Vec<String> = self.classes.iter().map(|it| it.to_string()).collect();
        classes.join(" ")
    }

    #[js_func]
    pub fn set_class(&mut self, class: String) {
        let old_classes = mem::take(&mut self.classes);
        for c in class.split(" ") {
            let c = c.trim();
            if !c.is_empty() {
                self.classes.insert(c.to_string());
            }
        }
        let need_update = CSS_MANAGER.with_borrow_mut(|cm| {
            old_classes
                .iter()
                .find(|it| cm.contains_class(it))
                .is_some()
                || self
                    .classes
                    .iter()
                    .find(|it| cm.contains_class(it))
                    .is_some()
        });
        if need_update {
            self.select_style_recurse();
        }
    }

    #[js_func]
    pub fn get_attribute(&self, key: String) -> Option<String> {
        self.attributes.get(&key).map(|it| it.to_string())
    }

    #[js_func]
    pub fn set_attribute(&mut self, key: String, value: String) {
        let need_update_style = CSS_MANAGER.with_borrow(|cm| cm.contains_attr(&key));
        let mut backend = self.backend.clone();
        let mut is_new = false;
        let v = self.attributes.entry(key.clone()).or_insert_with(|| {
            is_new = true;
            String::new()
        });
        if is_new || v != &value {
            *v = value;
            backend.on_attribute_changed(&key, Some(&v));
            if need_update_style {
                self.select_style_recurse();
            }
        }
    }

    #[js_func]
    pub fn remove_attribute(&mut self, key: String) {
        let need_update_style = CSS_MANAGER.with_borrow(|cm| cm.contains_attr(&key));
        self.attributes.remove(&key);
        self.backend.on_attribute_changed(&key, None);
        if need_update_style {
            self.select_style_recurse();
        }
    }

    #[js_func]
    pub fn is_disabled(&self) -> bool {
        self.is_form_element && self.attributes.contains_key("disabled")
    }

    #[js_func]
    pub fn set_disabled(&mut self, disabled: bool) {
        if !self.is_form_element {
            return;
        }
        if disabled {
            self.set_attribute("disabled".to_string(), "".to_string());
        } else {
            self.remove_attribute("disabled".to_string());
        }
    }

    #[js_func]
    pub fn set_draggable(&mut self, draggable: bool) {
        self.draggable = draggable;
    }

    #[js_func]
    pub fn set_scroll_y(&mut self, value: ScrollBarStrategy) {
        self.scrollable.vertical_bar.set_strategy(value);
        self.mark_dirty(true);
    }

    #[js_func]
    pub fn set_scroll_x(&mut self, value: ScrollBarStrategy) {
        self.scrollable.horizontal_bar.set_strategy(value);
        self.mark_dirty(true);
    }

    #[js_func]
    pub fn get_draggable(&mut self) -> bool {
        self.draggable
    }

    pub fn is_draggable(&self) -> bool {
        self.draggable
    }

    pub fn is_focused(&self) -> bool {
        if let Some(w) = &self.get_window() {
            let w = ok_or_return!(w.upgrade_mut(), false);
            w.is_focusing(self)
        } else {
            false
        }
    }

    #[js_func]
    pub fn create_by_tag(tag: String, context: JsValue) -> Result<Element, Error> {
        let tag = tag.to_lowercase();
        let mut view = ELEMENT_CREATORS.with_borrow_mut(|map| {
            if let Some(creator) = map.get_mut(&tag) {
                Element::new(creator)
            } else {
                Element::create(Container::create)
            }
        });
        view.resource_table.put(ElementJsContext { context });
        view.set_tag(tag);
        view.set_element_type(ElementType::Widget);
        Ok(view)
    }

    #[js_func]
    pub fn set_js_context(&mut self, context: JsValue) {
        self.resource_table.put(ElementJsContext { context });
    }

    #[js_func]
    pub fn get_js_context(&self) -> Result<JsValue, Error> {
        let e = self
            .resource_table
            .get::<ElementJsContext>()
            .map(|e| e.context.clone())
            .unwrap_or(JsValue::Undefined);
        Ok(e)
    }

    #[js_func]
    pub fn add_child(&mut self, child: Element, position: i32) -> Result<(), Error> {
        let position = if position < 0 {
            None
        } else {
            Some(position as u32)
        };
        self.add_child_view(child, position);
        Ok(())
    }

    #[js_func]
    pub fn remove_child(&mut self, position: u32) -> Result<(), Error> {
        self.remove_child_view(position);
        Ok(())
    }

    pub fn remove_all_child(&mut self) {
        while !self.children.is_empty() {
            let _ = self.remove_child(0);
        }
    }

    #[js_func]
    pub fn add_js_event_listener(
        &mut self,
        event_type: String,
        listener: JsValue,
    ) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener.clone();
            "click" => ClickEventListener,
            "contextmenu" => ContextMenuEventListener,
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
            "droppedfile" => DroppedFileEventListener,
            "hoveredfile" => HoveredFileEventListener,
        );
        if id.is_none() {
            if let Some(factory) = self.js_event_listener_factory.get_mut(&event_type) {
                if let Some((type_id, raw_listener)) = factory(listener.clone()) {
                    log::debug!("event listener added: name = {}, type_id = {:?}", &event_type, type_id);
                    return Ok(self
                        .event_registration
                        .register_raw_event_listener(type_id, raw_listener));
                }
            }
        }
        let id = id.ok_or_else(|| JsError::new(format!("unknown event_type:{}", event_type)))?;
        Ok(id)
    }

    #[js_func]
    pub fn focus(&mut self) {
        if let Some(mut window) = self.upgrade_window() {
            window.focus(self.clone());
        }
    }

    #[js_func]
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
        //TODO remove
        self.mark_dirty(false);
    }

    #[js_func]
    pub fn get_cursor(&self) -> Cursor {
        self.cursor.clone()
    }

    #[js_func]
    pub fn scroll_by(&mut self, option: ScrollByOption) {
        let (scroll_left, scroll_top) = self.scrollable.scroll_offset();
        if option.x != 0.0 {
            self.set_scroll_left(scroll_left + option.x);
        }
        if option.y != 0.0 {
            self.set_scroll_top(scroll_top + option.y);
        }
    }

    pub fn get_max_scroll_left(&self) -> f32 {
        let content_bounds = self.get_content_bounds();
        let width = content_bounds.width;
        (self.get_real_content_size().0 - width).max(0.0)
    }

    #[js_func]
    pub fn set_scroll_left(&mut self, value: f32) {
        self.scrollable.horizontal_bar.set_scroll_offset(value);
    }

    #[js_func]
    pub fn get_scroll_left(&self) -> f32 {
        self.scrollable.horizontal_bar.scroll_offset()
    }

    #[js_func]
    pub fn get_scroll_top(&self) -> f32 {
        self.scrollable.vertical_bar.scroll_offset()
    }

    pub fn get_max_scroll_top(&self) -> f32 {
        self.scrollable.vertical_bar.get_max_scroll_offset()
    }

    #[js_func]
    pub fn set_scroll_top(&mut self, value: f32) {
        self.scrollable.vertical_bar.set_scroll_offset(value);
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
        let (scroll_left, scroll_top) = self.scrollable.scroll_offset();
        self.emit(ScrollEvent {
            scroll_top,
            scroll_left,
        });
    }

    pub fn get_backend_as<T>(&self) -> &T {
        unsafe {
            // &*(self as *const dyn Any as *const T)
            &*(self.backend.deref().deref() as *const dyn ElementBackend as *const T)
        }
    }

    pub fn get_backend_mut_as<T>(&mut self) -> &mut T {
        unsafe {
            // &*(self as *const dyn Any as *const T)
            &mut *(self.backend.deref_mut().deref_mut() as *mut dyn ElementBackend as *mut T)
        }
    }

    pub fn get_backend_mut(&mut self) -> &mut Box<dyn ElementBackend> {
        &mut self.backend
    }

    pub fn get_backend(&self) -> &Box<dyn ElementBackend> {
        &self.backend
    }

    pub fn is_backend<T: 'static>(&self) -> bool {
        self.backend.backend_type_id() == TypeId::of::<T>()
    }

    fn set_parent_internal(&mut self, parent: ElementParent) {
        self.parent = parent;
        self.applied_style = Styles::new();
        self.select_style_recurse();
        self.mark_style_dirty();
    }

    pub fn set_parent(&mut self, parent: ElementParent) {
        self.parent = parent;
        self.process_auto_focus();
    }

    fn sync_style(&mut self) {
        self.mark_style_dirty();
    }

    pub fn with_window<F: FnOnce(&mut Window)>(&self, callback: F) {
        match &self.parent {
            ElementParent::None => {}
            ElementParent::Element(e) => {
                if let Ok(p) = e.upgrade() {
                    p.with_window(callback);
                }
            }
            ElementParent::Window(w) | ElementParent::Page(w) => {
                if let Ok(mut w) = w.upgrade() {
                    callback(&mut w);
                }
            }
        }
    }

    #[js_func]
    pub fn get_window(&self) -> Option<WindowWeak> {
        if let Some(p) = self.get_parent() {
            return p.get_window();
        } else if let ElementParent::Window(ww) = &self.parent {
            return Some(ww.clone());
        }
        None
    }

    pub fn upgrade_window(&self) -> Option<Window> {
        if let Some(f) = self.get_window() {
            f.upgrade().ok()
        } else {
            None
        }
    }

    #[js_func]
    pub fn get_parent(&self) -> Option<Element> {
        match &self.parent {
            ElementParent::Element(e) => Some(e.upgrade().ok()?),
            _ => None,
        }
    }

    pub fn get_root_element(&self) -> Element {
        if let Some(p) = self.get_parent() {
            p.get_root_element()
        } else {
            self.clone()
        }
    }

    #[js_func]
    pub fn get_size(&self) -> (f32, f32) {
        let layout = self.style.yoga_node._yn.get_layout();
        (layout.width().nan_to_zero(), layout.height().nan_to_zero())
    }

    #[js_func]
    pub fn set_auto_focus(&mut self, auto_focus: bool) {
        self.auto_focus = auto_focus;
    }

    #[js_func]
    pub fn get_auto_focus(&mut self) -> bool {
        self.auto_focus
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
        let ml = self.style.yoga_node._yn.get_layout();
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

    pub fn get_origin_content_bounds(&self) -> base::Rect {
        let (t, r, b, l) = self.get_padding();
        let bounds = self.get_origin_bounds();
        base::Rect::new(
            bounds.x + l,
            bounds.y + t,
            bounds.width - l - r,
            bounds.height - t - b,
        )
    }

    /// bounds relative to root node
    pub fn get_origin_bounds(&self) -> base::Rect {
        let b = self.get_bounds();
        return if let Some(p) = self.get_parent() {
            let pob = p.get_origin_bounds();
            let (offset_left, offset_top) = p.scrollable.scroll_offset();
            let x = pob.x + b.x - offset_left;
            let y = pob.y + b.y - offset_top;
            base::Rect::new(x, y, b.width, b.height)
        } else {
            b
        };
    }

    pub fn add_child_view(&mut self, mut child: Element, position: Option<u32>) {
        if let Some(p) = child.get_parent() {
            panic!(
                "child({}) has parent({}) already",
                child.get_eid(),
                p.get_eid()
            );
        }
        let pos = {
            let layout = &mut self.style;
            let pos = position.unwrap_or_else(|| layout.child_count());
            layout.insert_child(&mut child.style, pos);
            pos
        };
        self.mark_dirty(true);
        child.set_parent_internal(ElementParent::Element(self.as_weak()));
        self.children.insert(pos as usize, child.clone());
        child.process_auto_focus();
    }

    fn process_auto_focus(&self) {
        let focus_element = self.find_auto_focus_element();
        if let Some(mut fe) = focus_element {
            fe.focus();
        }
    }

    fn find_auto_focus_element(&self) -> Option<Element> {
        for c in self.children.iter().rev() {
            if let Some(fc) = c.find_auto_focus_element() {
                return Some(fc);
            }
        }
        if self.auto_focus {
            Some(self.clone())
        } else {
            None
        }
    }

    pub fn remove_child_view(&mut self, position: u32) {
        let mut c = self.children.remove(position as usize);
        c.set_parent_internal(ElementParent::None);
        let mut ele = self.clone();
        let layout = &mut ele.style;
        layout.remove_child(&mut c.style);
        ele.mark_dirty(true);
        if let Some(window) = self.get_window() {
            if let Ok(mut f) = window.upgrade_mut() {
                f.on_element_removed(&c);
            }
        }
    }

    pub fn get_children(&self) -> Vec<Element> {
        self.children.clone()
    }

    pub fn calculate_layout(&mut self, available_width: f32, available_height: f32) {
        // mark all children dirty so that custom measure function could be call
        // self.mark_all_layout_dirty();
        // debug!("calculate layout: {} {}", self.id, self.style.has_shadow());
        if self.style.has_shadow() {
            let mut me = self.clone();
            self.scrollable.update_layout(&mut me);
        } else {
            self.before_layout_recurse();
            self.style
                .calculate_layout(available_width, available_height, Direction::LTR);
            self.on_layout_update();
        }
    }

    pub fn get_border_width(&self) -> (f32, f32, f32, f32) {
        (
            self.style.yoga_node._yn.get_style_border_top().de_nan(0.0),
            self.style
                .yoga_node
                ._yn
                .get_style_border_right()
                .de_nan(0.0),
            self.style
                .yoga_node
                ._yn
                .get_style_border_bottom()
                .de_nan(0.0),
            self.style.yoga_node._yn.get_style_border_left().de_nan(0.0),
        )
    }

    /// Return the padding of element (order: Top, Right, Bottom, Left)
    pub fn get_padding(&self) -> (f32, f32, f32, f32) {
        self.style.get_padding()
    }

    #[js_func]
    pub fn set_style(&mut self, style: JsValue) {
        self.update_style(style, true);
    }

    #[js_func]
    pub fn get_style(&self) -> JsValue {
        let mut result = HashMap::new();
        for (_, v) in self.style_list.get_styles(self.hover) {
            result.insert(v.name().to_string(), JsValue::String(v.to_style_string()));
        }
        JsValue::Object(result)
    }

    pub fn update_style(&mut self, style: JsValue, full: bool) {
        if full {
            self.style_list.clear();
        }
        self.style_list.set_style_obj(style);
        self.sync_style();
    }

    pub fn set_style_props(&mut self, styles: Vec<FixedStyleProp>) {
        // self.style_props.clear();
        self.style_list.set_style_props(styles);
        self.sync_style();
    }

    #[js_func]
    pub fn set_hover_style(&mut self, style: JsValue) {
        self.style_list.set_hover_style(style);
        if self.hover {
            self.mark_style_dirty();
        }
    }

    pub fn set_hover_styles(&mut self, styles: Vec<FixedStyleProp>) {
        self.style_list.set_hover_styles(styles);
        if self.hover {
            self.mark_style_dirty();
        }
    }

    #[js_func]
    pub fn get_bounding_client_rect(&self) -> base::Rect {
        self.get_origin_bounds()
    }

    //TODO remove
    fn calculate_changed_style<'a>(
        old_style_map: &'a HashMap<StylePropKey, FixedStyleProp>,
        new_style_map: &'a HashMap<StylePropKey, FixedStyleProp>,
        parent_changed: &Vec<StylePropKey>,
    ) -> Vec<FixedStyleProp> {
        let mut changed_style_props = HashMap::new();
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
                None => old_value.unwrap().clone().unset(),
            };
            if old_value != Some(&new_value) {
                changed_style_props.insert(new_value.key(), new_value);
            }
        }
        for pc in parent_changed {
            if changed_style_props.contains_key(pc) {
                continue;
            }
            if let Some(v) = old_style_map.get(pc) {
                if v.is_inherited() {
                    changed_style_props.insert(v.key(), v.clone().clone());
                }
            }
        }
        changed_style_props.values().cloned().into_iter().collect()
    }

    pub(crate) fn mark_style_dirty(&mut self) {
        self.dirty_flags |= StyleDirtyFlags::SelfDirty;
        self.mark_dirty(false);
        if let Some(mut p) = self.get_parent() {
            p.mark_children_style_dirty();
        }
    }

    fn mark_children_style_dirty(&mut self) {
        if !self.dirty_flags.contains(StyleDirtyFlags::ChildrenDirty) {
            self.dirty_flags |= StyleDirtyFlags::ChildrenDirty;
            if let Some(mut p) = self.get_parent() {
                p.mark_children_style_dirty();
            }
        }
    }

    pub(crate) fn resolve_style_vars_recurse(&mut self, parent_vars: &StyleVars) {
        let new_vars = self.style_list.resolve_variables(&parent_vars);
        for mut c in self.get_children() {
            c.resolve_style_vars_recurse(&new_vars);
        }
    }

    pub(crate) fn compute_font_size_recurse(&mut self, ctx: &LengthContext) {
        let style = self.style_list.get_styles(self.hover);
        let px = if let Some(FixedStyleProp::FontSize(fs_prop)) = style.get(&StylePropKey::FontSize)
        {
            match fs_prop {
                StylePropVal::Custom(c) => c.to_px(&ctx),
                _ => ctx.font_size,
            }
        } else {
            ctx.font_size
        };
        if self.style.font_size != px {
            self.style.font_size = px;
            self.backend.handle_style_changed(StylePropKey::FontSize);
        }
        let mut ctx = ctx.clone();
        ctx.font_size = px;

        for mut c in self.get_children() {
            c.compute_font_size_recurse(&ctx);
        }
    }

    pub(crate) fn apply_style_update(&mut self, parent_changed: bool, length_ctx: &LengthContext) {
        let is_self_dirty = self.dirty_flags.contains(StyleDirtyFlags::SelfDirty);
        let is_children_dirty = self.dirty_flags.contains(StyleDirtyFlags::ChildrenDirty);
        let changed = if is_self_dirty || parent_changed {
            self.apply_owned_style(length_ctx)
        } else {
            false
        };
        if is_children_dirty || changed {
            let mut children = self.get_children();
            for c in &mut children {
                c.apply_style_update(changed, length_ctx);
            }
        }
        self.dirty_flags.remove(StyleDirtyFlags::ChildrenDirty);
        self.dirty_flags.remove(StyleDirtyFlags::SelfDirty);
    }

    fn compute_owned_style(&mut self) -> (Styles, HashMap<String, Styles>) {
        let mut style_props = self.style_list.get_styles(self.hover);
        for (k, v) in &self.animation_style_props {
            style_props.insert(k.clone(), v.clone());
        }
        let styles = self.resolve_style_props(style_props);
        let mut pseudo_element_styles = HashMap::new();

        for (k, v) in self.style_list.get_pseudo_element_style_props() {
            let pe_styles = self.resolve_style_props(v);
            pseudo_element_styles.insert(k, pe_styles);
        }
        (styles, pseudo_element_styles)
    }

    fn resolve_style_props(&self, style_props: HashMap<StylePropKey, FixedStyleProp>) -> Styles {
        let mut resolved = HashMap::new();
        for (k, prop) in style_props {
            let v = prop.resolve_value(
                |k| self.style.get_default_value(k),
                |k| {
                    if let Some(p) = self.get_parent() {
                        p.style.get_resolved_value(k)
                    } else {
                        self.style.get_default_value(k)
                    }
                },
            );
            resolved.insert(k, v);
        }
        Styles::from_map(resolved)
    }

    pub fn apply_owned_style(&mut self, length_ctx: &LengthContext) -> bool {
        let (styles, pseudo_element_styles) = self.compute_owned_style();

        let changed_styles =
            styles.compute_changed_style(&self.applied_style, |k| self.style.get_default_value(k));
        let mut changed = !changed_styles.is_empty();
        for sp in changed_styles {
            let (repaint, need_layout) = self.style.set_resolved_style_prop(sp, length_ctx);
            if need_layout || repaint {
                self.mark_dirty(need_layout);
            }
        }

        let mut pseudo_element_keys = Vec::new();
        for (k, _) in &pseudo_element_styles {
            pseudo_element_keys.push(k);
        }
        for (k, _) in &self.applied_pseudo_element_styles {
            pseudo_element_keys.push(k);
        }
        let empty_styles = Styles::default();
        let mut changed_pe_styles_map = HashMap::new();
        for k in pseudo_element_keys {
            let new_style = pseudo_element_styles.get(k).unwrap_or(&empty_styles);
            let old_style = self
                .applied_pseudo_element_styles
                .get(k)
                .unwrap_or(&empty_styles);
            let changed_pe_styles =
                new_style.compute_changed_style(&old_style, |k| self.style.get_default_value(k));
            if !changed_pe_styles.is_empty() {
                changed_pe_styles_map.insert(k.clone(), changed_pe_styles);
            }
        }
        if !changed_pe_styles_map.is_empty() {
            self.backend
                .accept_pseudo_element_styles(changed_pe_styles_map);
            changed = true;
        }

        // println!("changed list: {} {:?}", self.id, changed_list);
        self.applied_style = styles;
        self.applied_pseudo_element_styles = pseudo_element_styles;
        changed
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, ElementWeak> + 'static>(
        &mut self,
        listener: H,
    ) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    pub fn register_js_event<T: JsEvent<ElementWeak>>(&mut self, name: &str) {
        self.js_event_listener_factory
            .insert(name.to_string(), T::create_listener_factory());
    }

    #[js_func]
    pub fn remove_js_event_listener(&mut self, id: u32) {
        self.unregister_event_listener(id);
    }

    pub fn emit<T: ViewEvent + 'static>(&self, event: T) {
        let event_type_id = TypeId::of::<T>();
        self.emit_raw(event_type_id, Event::new(event));
    }

    pub fn emit_raw(&self, event_type_id: TypeId, mut event: Event) {
        // log::debug!("emitting {:?}", event_type_id);
        let mut me = self.clone();
        let callback = create_event_loop_callback(move || {
            let mut ctx = EventContext::new(me.as_weak());
            me.handle_event(event_type_id, &mut event, &mut ctx);
            if !ctx.prevent_default {
                me.handle_default_behavior(&mut event, &mut ctx);
            }
        });
        callback.call();
    }

    pub fn create_event_emitter(&mut self) -> EventEmitter {
        EventEmitter::new(&self)
    }

    fn handle_event(
        &mut self,
        event_type_id: TypeId,
        event: &mut Event,
        ctx: &mut EventContext<ElementWeak>,
    ) {
        if self.is_form_element && is_form_event(&event) && self.is_disabled() {
            ctx.propagation_cancelled = true;
            return;
        }
        if event_type_id == TypeId::of::<MouseEnterEvent>() {
            self.hover = true;
            //TODO optimize performance
            if !self.parent.is_element() {
                self.update_select_style_recurse();
            }
            if self.style_list.has_hover_style() {
                self.mark_style_dirty();
            }
        } else if event_type_id == TypeId::of::<MouseLeaveEvent>() {
            self.hover = false;
            //TODO optimize performance
            //FIXME style may not be updated if event stop propagates?
            if !self.parent.is_element() {
                self.update_select_style_recurse();
            }
            if self.style_list.has_hover_style() {
                self.mark_style_dirty();
            }
        }
        let me = self.clone();
        if !self.scrollable.on_event(&event, ctx, &me) {
            let backend = self.get_backend_mut();
            backend.on_event(event, ctx);
        }
        if !ctx.propagation_cancelled {
            self.event_registration.emit_raw(event_type_id, event, ctx);
            if ctx.allow_bubbles && !ctx.propagation_cancelled {
                if let Some(mut p) = self.get_parent() {
                    p.handle_event(event_type_id, event, ctx);
                }
            }
        }
    }

    fn handle_default_behavior(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
        if MouseDownEvent::is(event) || TouchStartEvent::is(event) {
            if self.as_weak() == ctx.target {
                if let Some(win) = self.get_window() {
                    if let Ok(mut win) = win.upgrade_mut() {
                        win.focus(self.clone());
                    }
                }
            }
        }
        if !self.backend.execute_default_behavior(event, ctx) {
            if let Some(mut p) = self.get_parent() {
                p.handle_default_behavior(event, ctx);
            }
        }
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: u32) {
        self.event_registration
            .remove_event_listener(&event_type, id)
    }

    pub fn mark_dirty(&mut self, layout_dirty: bool) {
        if layout_dirty {
            if let Some(mut p) = self.get_parent() {
                if p.style.has_shadow() {
                    self.with_window(|win| {
                        win.invalid_layout(p);
                    });
                } else {
                    p.mark_dirty(layout_dirty);
                }
            } else {
                self.with_window(|win| {
                    win.invalid_layout(self.clone());
                });
            }
        } else {
            let el = self.clone();
            self.request_invalid(&el);
        }
    }

    fn request_invalid(&mut self, element: &Element) {
        if let Some(mut p) = self.get_parent() {
            p.request_invalid(element);
        } else {
            self.with_window(|w| {
                let root = element.get_root_element();
                if let Some(tree) = w.render_tree.get_mut(&root) {
                    tree.invalid_element(element);
                }
                w.notify_update();
            });
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

    pub fn before_layout_recurse(&mut self) {
        self.backend.before_layout();
        for c in &mut self.children {
            c.before_layout_recurse();
        }
    }
    pub fn before_render_recurse(&mut self) {
        self.scrollable.execute_auto_scroll_callback();
        for c in &mut self.children {
            c.before_render_recurse();
        }
    }

    pub fn on_layout_update(&mut self) {
        //TODO emit size change
        let origin_bounds = self.get_origin_bounds();
        if origin_bounds != self.rect {
            self.rect = origin_bounds.clone();
            // Disable bubble
            let mut ctx = EventContext::new(self.as_weak());
            ctx.propagation_cancelled = true;
            let event = BoundsChangeEvent {
                origin_bounds: origin_bounds.clone(),
            };
            self.event_registration.emit(event, &mut ctx);
        }
        //TODO performance: maybe not changed?
        //TODO change is_visible?
        if !origin_bounds.is_empty() {
            self.backend.handle_origin_bounds_change(&origin_bounds);
            if self.style.has_shadow() {
                self.calculate_layout(origin_bounds.width, origin_bounds.height);
            } else {
                for child in &mut self.get_children() {
                    child.on_layout_update();
                }
            }
        }
    }

    pub fn get_border_path_mut(&mut self) -> &mut BorderPath {
        let bounds = self.get_bounds();
        let border_widths = self.get_border_width();
        let border_widths = [
            border_widths.0,
            border_widths.1,
            border_widths.2,
            border_widths.3,
        ];
        let bp = BorderPath::new(
            bounds.width,
            bounds.height,
            self.style.border_radius,
            border_widths,
        );
        if !self.border_path.is_same(&bp) {
            self.border_path = bp;
        }
        &mut self.border_path
    }

    #[js_func]
    pub fn set_focusable(&mut self, focusable: bool) {
        self.focusable = focusable;
    }

    #[js_func]
    pub fn is_focusable(&self) -> bool {
        if self.is_form_element && self.is_disabled() {
            return false;
        }
        self.focusable
    }

    pub(crate) fn select_style(&mut self) {
        if self.element_type == ElementType::Widget {
            let (style, pseudo_styles) = CSS_MANAGER.with_borrow(|cm| cm.match_styles(&self));
            let selector_style_changed = self.style_list.set_selector_style(style);
            let pseudo_element_style_changed =
                self.style_list.set_pseudo_element_style(pseudo_styles);
            if selector_style_changed || pseudo_element_style_changed {
                self.mark_style_dirty();
            }
        }
    }

    pub fn set_element_type(&mut self, element_type: ElementType) {
        if self.element_type != element_type {
            self.element_type = element_type;
            if element_type == ElementType::Widget {
                self.select_style();
            }
        }
    }

    pub fn update_select_style_recurse(&mut self) {
        self.select_style_recurse();
    }

    pub fn set_tag(&mut self, tag: String) {
        self.tag = tag;
    }

    fn select_style_recurse(&mut self) {
        self.select_style();
        for mut child in self.get_children() {
            child.select_style_recurse();
        }
    }
}

impl ElementWeak {
    pub fn emit<T: ViewEvent + 'static>(&self, event: T) {
        if let Ok(el) = self.upgrade() {
            el.emit(event);
        }
    }
    pub fn mark_dirty(&mut self, layout_dirty: bool) {
        let mut ele = ok_or_return!(self.upgrade_mut());
        ele.mark_dirty(layout_dirty);
    }
}

impl Debug for Element {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("#")?;
        self.id.fmt(f)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum ElementType {
    Widget,
    Inner,
}

#[derive(PartialEq, Clone)]
pub enum ElementParent {
    None,
    Element(ElementWeak),
    Window(WindowWeak),
    Page(WindowWeak),
}

impl ElementParent {
    pub fn is_element(&self) -> bool {
        match self {
            ElementParent::None => false,
            ElementParent::Element(_) => true,
            ElementParent::Window(_) => false,
            ElementParent::Page(_) => false,
        }
    }
}

#[mrc_object]
pub struct Element {
    id: u32,
    backend: Mrc<Box<dyn ElementBackend>>,
    pub(crate) parent: ElementParent,
    children: Vec<Element>,
    event_registration: EventRegistration<ElementWeak>,
    pub style: StyleNode,
    pub(crate) animation_style_props: HashMap<StylePropKey, FixedStyleProp>,
    pub(crate) hover: bool,
    auto_focus: bool,
    dirty_flags: StyleDirtyFlags,
    element_type: ElementType,

    applied_style: Styles,
    applied_pseudo_element_styles: HashMap<String, Styles>,
    // animation_instance: Option<AnimationInstance>,
    draggable: bool,
    cursor: Cursor,
    rect: base::Rect,
    resource_table: ResourceTable,
    children_decoration: (f32, f32, f32, f32),

    //TODO rename
    pub need_snapshot: bool,
    pub render_object_idx: Option<usize>,
    border_path: BorderPath,
    style_list: StyleList,
    focusable: bool,
    pub(crate) classes: HashSet<String>,
    pub(crate) attributes: HashMap<String, String>,
    pub scrollable: Scrollable,
    pub tag: String,
    is_form_element: bool,
    pub allow_ime: bool,
    js_event_listener_factory: HashMap<String, BoxJsEventListenerFactory<ElementWeak>>,
}

// js_weak_value!(Element, ElementWeak);
js_value!(Element);
js_auto_upgrade!(ElementWeak, Element);

impl Hash for Element {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Element {}

impl ElementData {
    pub fn new<T: ElementBackend + 'static>(backend: T) -> Self {
        let id = NEXT_ELEMENT_ID.get();
        NEXT_ELEMENT_ID.set(id + 1);
        let scrollable = Scrollable::new();
        Self {
            id,
            backend: Mrc::new(Box::new(backend)),
            parent: ElementParent::None,
            event_registration: EventRegistration::new(),
            style: StyleNode::new(),
            animation_style_props: HashMap::new(),
            applied_style: Styles::new(),
            hover: false,
            element_type: ElementType::Inner,

            draggable: false,
            cursor: Cursor::Icon(CursorIcon::Default),
            rect: base::Rect::empty(),
            resource_table: ResourceTable::new(),
            children_decoration: (0.0, 0.0, 0.0, 0.0),
            children: Vec::new(),
            need_snapshot: false,
            render_object_idx: None,
            border_path: BorderPath::new(0.0, 0.0, [0.0; 4], [0.0; 4]),
            style_list: StyleList::new(),
            auto_focus: false,
            focusable: false,
            dirty_flags: StyleDirtyFlags::empty(),
            classes: HashSet::new(),
            attributes: HashMap::new(),
            applied_pseudo_element_styles: HashMap::new(),
            scrollable,
            tag: "".to_string(),
            is_form_element: false,
            allow_ime: false,
            js_event_listener_factory: HashMap::new(),
        }
    }
}

pub struct EmptyElementBackend {}

impl ElementBackend for EmptyElementBackend {
    fn create(_ele: &mut Element) -> Self {
        Self {}
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }
}

pub trait ElementBackend: 'static {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized;

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        if let Some(base) = self.get_base_mut() {
            base.handle_style_changed(key);
        }
    }

    fn render(&mut self) -> RenderFn {
        if let Some(base) = self.get_base_mut() {
            base.render()
        } else {
            RenderFn::new(|_c| {})
        }
    }

    fn on_event(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
        if let Some(base) = self.get_base_mut() {
            base.on_event(event, ctx);
        }
    }

    fn execute_default_behavior(
        &mut self,
        event: &mut Event,
        ctx: &mut EventContext<ElementWeak>,
    ) -> bool {
        if let Some(base) = self.get_base_mut() {
            base.execute_default_behavior(event, ctx)
        } else {
            false
        }
    }

    fn before_layout(&mut self) {
        if let Some(base) = self.get_base_mut() {
            base.before_layout();
        }
    }

    fn handle_origin_bounds_change(&mut self, bounds: &base::Rect) {
        if let Some(base) = self.get_base_mut() {
            base.handle_origin_bounds_change(bounds);
        }
    }

    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(base) = self.get_base_mut() {
            base.accept_pseudo_element_styles(styles);
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        if let Some(base) = self.get_base_mut() {
            base.on_attribute_changed(key, value);
        }
    }

    fn backend_type_id(&self) -> TypeId {
        self.type_id()
    }
}

#[test]
fn test_backend_type_id() {
    let el = Element::create(Container::create);
    assert_eq!(true, el.is_backend::<Container>());
}
