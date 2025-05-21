use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::mem;
use std::ops::{Deref, DerefMut};

use anyhow::{anyhow, Error};
use bitflags::{bitflags};
use deft_macros::{js_methods, mrc_object};
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use skia_safe::Rect;
use winit::window::CursorIcon;
use yoga::{Direction, StyleUnit};

use crate::base::{EventContext, EventListener, EventRegistration};
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::entry::Entry;
use crate::element::image::Image;
use crate::element::scroll::Scroll;
use crate::event::{DragOverEventListener, BlurEventListener, BoundsChangeEventListener, CaretChangeEventListener, ClickEventListener, DragStartEventListener, DropEventListener, FocusEventListener, FocusShiftEventListener, KeyDownEventListener, KeyUpEventListener, MouseDownEventListener, MouseEnterEvent, MouseEnterEventListener, MouseLeaveEvent, MouseLeaveEventListener, MouseMoveEventListener, MouseUpEventListener, MouseWheelEventListener, ScrollEvent, ScrollEventListener, TextChangeEventListener, TextUpdateEventListener, TouchCancelEventListener, TouchEndEventListener, TouchMoveEventListener, TouchStartEventListener, BoundsChangeEvent, ContextMenuEventListener, MouseDownEvent, TouchStartEvent, DroppedFileEventListener, HoveredFileEventListener};
use crate::event_loop::{create_event_loop_callback};
use crate::ext::ext_window::{ELEMENT_TYPE_BUTTON, ELEMENT_TYPE_CONTAINER, ELEMENT_TYPE_ENTRY, ELEMENT_TYPE_IMAGE, ELEMENT_TYPE_LABEL, ELEMENT_TYPE_SCROLL, ELEMENT_TYPE_BODY, ELEMENT_TYPE_PARAGRAPH, ELEMENT_TYPE_CHECKBOX, ELEMENT_TYPE_RADIO, ELEMENT_TYPE_RADIO_GROUP, ELEMENT_TYPE_RICH_TEXT};
use crate::window::{Window, WindowWeak};
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;
use crate::resource_table::ResourceTable;
use crate::style::{LengthContext, ResolvedStyleProp, StyleNode, StyleProp, StylePropKey, StylePropVal};
use crate::{base, bind_js_event_listener, js_auto_upgrade, js_deserialize, js_serialize, js_value, ok_or_return};

pub mod container;
pub mod entry;
pub mod button;
pub mod scroll;
pub mod image;
pub mod label;
mod edit_history;
pub mod text;
pub mod paragraph;
pub mod body;
mod font_manager;
mod util;
pub mod checkbox;
pub mod radio;
mod common;
pub mod richtext;

use crate as deft;
use crate::computed::ComputedValue;
use crate::element::body::Body;
use crate::element::checkbox::Checkbox;
use crate::element::label::Label;
use crate::element::paragraph::Paragraph;
use crate::element::radio::{Radio, RadioGroup};
use crate::element::richtext::RichText;
use crate::js::JsError;
use crate::layout::LayoutRoot;
use crate::paint::MatrixCalculator;
use crate::render::RenderFn;
use crate::style::border_path::BorderPath;
use crate::style::css_manager::CssManager;
use crate::style_list::{ParsedStyleProp, StyleList};

thread_local! {
    pub static NEXT_ELEMENT_ID: Cell<u32> = Cell::new(1);
    pub static STYLE_VARS: ComputedValue<String> = ComputedValue::new();
    pub static CSS_MANAGER: RefCell<CssManager> = RefCell::new(CssManager::new());
}

bitflags! {

    struct StyleDirtyFlags: u8 {
        const SelfDirty = 0b1;
        const ChildrenDirty = 0b10;
        const LayoutDirty = 0b100;
    }

}

impl StyleDirtyFlags {
    pub fn is_layout_dirty(&self) -> bool {
        self.contains(Self::LayoutDirty)
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

#[js_methods]
impl Element {
    pub fn create<T: ElementBackend + 'static, F: FnOnce(&mut Element) -> T>(backend: F) -> Self {
        let empty_backend = EmptyElementBackend{};
        let inner =  Mrc::new(ElementData::new(empty_backend));
        let mut ele = Self {
            inner,
        };
        let weak = ele.as_weak();
        ele.style.bind_element(weak);
        let ele_weak = ele.inner.as_weak();
        // let bk = backend(ele_cp);
        ele.backend = Mrc::new(Box::new(backend(&mut ele)));
        ele.style.on_changed = Some(Box::new(move |key| {
            if let Ok(mut inner) = ele_weak.upgrade() {
                inner.backend.handle_style_changed(key);
            }
        }));
        //ele.backend.bind(ele_cp);
        ele
    }

    pub fn create_shadow(&mut self) {
        self.style = StyleNode::new_with_shadow();
        let weak = self.as_weak();
        self.style.bind_element(weak);
    }

    #[js_func]
    pub fn get_eid(&self) -> u32 {
        self.id
    }

    #[js_func]
    pub fn get_class(&self) -> String {
        let classes: Vec<String> = self.classes.iter()
            .map(|it| it.to_string())
            .collect();
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
            old_classes.iter().find(|it| cm.contains_class(it)).is_some()
                || self.classes.iter().find(|it| cm.contains_class(it)).is_some()
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
        let need_update_style = CSS_MANAGER.with_borrow(|cm| {
            cm.contains_attr(&key)
        });
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
        let need_update_style = CSS_MANAGER.with_borrow(|cm| {
            cm.contains_attr(&key)
        });
        self.attributes.remove(&key);
        self.backend.on_attribute_changed(&key, None);
        if need_update_style {
            self.select_style_recurse();
        }
    }

    #[js_func]
    pub fn set_draggable(&mut self, draggable: bool) {
        self.draggable = draggable;
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
    pub fn create_by_type(view_type: i32, context: JsValue) -> Result<Element, Error> {
        let mut view = match view_type {
            ELEMENT_TYPE_CONTAINER => Element::create(Container::create),
            ELEMENT_TYPE_SCROLL => Element::create(Scroll::create),
            ELEMENT_TYPE_LABEL => Element::create(Label::create),
            ELEMENT_TYPE_ENTRY => Element::create(Entry::create),
            ELEMENT_TYPE_BUTTON => Element::create(Button::create),
            ELEMENT_TYPE_IMAGE => Element::create(Image::create),
            ELEMENT_TYPE_BODY => Element::create(Body::create),
            ELEMENT_TYPE_PARAGRAPH => Element::create(Paragraph::create),
            ELEMENT_TYPE_CHECKBOX => Element::create(Checkbox::create),
            ELEMENT_TYPE_RADIO => Element::create(Radio::create),
            ELEMENT_TYPE_RADIO_GROUP => Element::create(RadioGroup::create),
            ELEMENT_TYPE_RICH_TEXT => Element::create(RichText::create),
            _ => return Err(anyhow!("invalid view_type")),
        };
        view.resource_table.put(ElementJsContext { context });
        view.set_element_type(ElementType::Widget);

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
    pub fn add_child(&mut self, child: Element, position: i32) -> Result<(), Error> {
        let position = if position < 0 { None } else { Some(position as u32) };
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
    pub fn add_js_event_listener(&mut self, event_type: String, listener: JsValue) -> Result<u32, JsError> {
        let mut id = bind_js_event_listener!(
            self, event_type.as_str(), listener.clone();
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
            "droppedfile" => DroppedFileEventListener,
            "hoveredfile" => HoveredFileEventListener,
        );
        if id.is_none() {
            id = self.backend.bind_js_listener(&event_type, listener);
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
    pub fn set_cursor(&mut self, cursor: CursorIcon) {
        self.cursor = cursor;
        self.mark_dirty(false);
    }

    #[js_func]
    pub fn get_cursor(&self) -> CursorIcon {
        self.cursor
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

    pub fn get_max_scroll_left(&self) -> f32 {
        let content_bounds = self.get_content_bounds();
        let width = content_bounds.width;
        (self.get_real_content_size().0 - width).max(0.0)
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

    pub fn get_max_scroll_top(&self) -> f32 {
        let content_bounds = self.get_content_bounds();
        let height = content_bounds.height;
        (self.get_real_content_size().1 - height).max(0.0)
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

    fn set_parent_internal(&mut self, parent: Option<Element>) {
        self.parent = match parent {
            None => {
                self.on_window_changed(&None);
                None
            },
            Some(p) => {
                self.on_window_changed(&p.get_window());
                Some(p.as_weak())
            },
        };
        self.applied_style.clear();
        if let Some(win) = self.get_window() {
            if let Ok(win) = win.upgrade_mut() {
                self.refresh_style_variables(&win.style_variables.as_weak());
            }
        }
        self.select_style_recurse();
        self.mark_style_dirty();
    }

    pub fn set_window(&mut self, window: Option<WindowWeak>) {
        self.window = window.clone();
        self.on_window_changed(&window);
        self.process_auto_focus();
    }

    pub fn on_window_changed(&mut self, window: &Option<WindowWeak>) {
        //TODO remove?
        for mut c in self.get_children() {
            c.on_window_changed(window);
        }
    }

    pub fn refresh_style_variables(&mut self, variables: &MrcWeak<HashMap<String, String>>) {
        self.style_list.set_variables(variables.clone());
        self.sync_style();
        for mut c in self.get_children() {
            c.refresh_style_variables(variables);
        }
    }

    fn sync_style(&mut self) {
        self.mark_style_dirty();
    }

    pub fn with_window<F: FnOnce(&mut Window)>(&self, callback: F) {
        if let Some(p) = self.get_parent() {
            return p.with_window(callback);
        } else if let Some(ww) = &self.window {
            if let Ok(mut w) = ww.upgrade() {
                callback(&mut w);
            }
        }
    }

    #[js_func]
    pub fn get_window(&self) -> Option<WindowWeak> {
        if let Some(p) = self.get_parent() {
            return p.get_window()
        } else if let Some(ww) = &self.window {
            return Some(ww.clone())
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

    #[js_func]
    pub fn get_size(&self) -> (f32, f32) {
        let layout = self.style.yoga_node.get_layout();
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
        let ml = self.style.yoga_node.get_layout();
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
            panic!("child({}) has parent({}) already", child.get_eid(), p.get_eid());
        }
        let pos = {
            let layout = &mut self.style;
            let pos = position.unwrap_or_else(|| layout.child_count());
            layout.insert_child(&mut child.style, pos);
            pos
        };
        self.mark_dirty(true);
        child.set_parent_internal(Some(self.clone()));
        child.set_dirty_state_recurse(true);
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
        c.set_parent_internal(None);
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
        self.before_layout_recurse();
        self.style.calculate_layout(available_width, available_height, Direction::LTR);
        if let Some(lr) = &mut self.layout_root {
            lr.update_layout();
            assert_eq!(false, self.dirty_flags.contains(StyleDirtyFlags::LayoutDirty));
        }
        self.on_layout_update();
    }

    pub fn get_border_width(&self) -> (f32, f32, f32, f32) {
        (
            self.style.yoga_node.get_style_border_top().de_nan(0.0),
            self.style.yoga_node.get_style_border_right().de_nan(0.0),
            self.style.yoga_node.get_style_border_bottom().de_nan(0.0),
            self.style.yoga_node.get_style_border_left().de_nan(0.0),
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
            result.insert(
                v.name().to_string(),
                JsValue::String(v.to_style_string())
            );
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

    pub fn set_style_props(&mut self, styles: Vec<StyleProp>) {
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

    #[js_func]
    pub fn get_bounding_client_rect(&self) -> base::Rect {
        self.get_origin_bounds()
    }

    //TODO remove
    fn calculate_changed_style<'a>(
        old_style_map: &'a HashMap<StylePropKey, StyleProp>,
        new_style_map: &'a HashMap<StylePropKey, StyleProp>,
        parent_changed: &Vec<StylePropKey>,
    ) -> Vec<StyleProp> {
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

    pub(crate) fn compute_font_size_recurse(&mut self, ctx: &LengthContext) {
        let style = self.style_list.get_styles(self.hover);
        let px = if let Some(StyleProp::FontSize(fs_prop)) = style.get(&StylePropKey::FontSize) {
            match fs_prop {
                StylePropVal::Custom(c) => {
                    c.to_px(&ctx)
                }
                _ => {
                    ctx.font_size
                }
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

    pub(crate) fn apply_style_update(&mut self, parent_changed: &Vec<StylePropKey>, length_ctx: &LengthContext) {
        let is_self_dirty = self.dirty_flags.contains(StyleDirtyFlags::SelfDirty);
        let is_children_dirty = self.dirty_flags.contains(StyleDirtyFlags::ChildrenDirty);
        let mut changed_keys = Vec::new();
        if is_self_dirty || !parent_changed.is_empty() {
            let changed_styles = self.apply_own_style(parent_changed, length_ctx);
            for s in changed_styles {
                changed_keys.push(s.key());
            }
        }
        if is_children_dirty || !changed_keys.is_empty() {
            let mut children = self.get_children();
            for c in &mut children {
                c.apply_style_update(&changed_keys, length_ctx);
            }
        }
        self.dirty_flags.remove(StyleDirtyFlags::ChildrenDirty);
        self.dirty_flags.remove(StyleDirtyFlags::SelfDirty);
    }

    pub fn apply_own_style(&mut self, parent_changed: &Vec<StylePropKey>, length_ctx: &LengthContext) -> Vec<ResolvedStyleProp> {
        let mut style_props = self.style_list.get_styles(self.hover);
        for (k, v) in &self.animation_style_props {
            style_props.insert(k.clone(), v.clone());
        }
        let old_style = self.applied_style.clone();
        let changed_style_props = Self::calculate_changed_style(&old_style, &style_props, parent_changed);
        // println!("new styles {} => {:?}", self.id, style_props);


        let mut changed_list = Vec::new();
        for sp in changed_style_props {
            let (repaint, need_layout, v) = self.apply_style_prop(sp, length_ctx);
            if need_layout || repaint {
                self.mark_dirty(need_layout);
                changed_list.push(v);
            }
        }
        // println!("changed list: {} {:?}", self.id, changed_list);
        self.applied_style = style_props;
        changed_list
    }

    pub fn apply_style_prop(&mut self, prop: StyleProp, length_ctx: &LengthContext) -> (bool, bool, ResolvedStyleProp) {
        if let Some(v) = self.backend.apply_style_prop(&prop, length_ctx) {
            v
        } else {
            self.style.set_style(prop, length_ctx)
        }
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, ElementWeak> + 'static>(&mut self, listener: H) -> u32 {
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
            //TODO optimize performance
            if self.parent.is_none() {
                self.update_select_style_recurse();
            }
            if self.style_list.has_hover_style() {
                self.mark_style_dirty();
            }
        } else if TypeId::of::<T>() == TypeId::of::<MouseLeaveEvent>() {
            self.hover = false;
            //TODO optimize performance
            if self.parent.is_none() {
                self.update_select_style_recurse();
            }
            if self.style_list.has_hover_style() {
                self.mark_style_dirty();
            }
        }
        let backend = self.get_backend_mut();
        let e: Box<&mut dyn Any> = Box::new(event);
        backend.on_event(e, ctx);
        if !ctx.propagation_cancelled {
            self.event_registration.emit(event, ctx);
            if event.allow_bubbles() && !ctx.propagation_cancelled {
                if let Some(mut p) = self.get_parent() {
                    p.handle_event(event, ctx);
                }
            }
        }
    }

    fn handle_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        if event.downcast_ref::<MouseDownEvent>().is_some() || event.downcast_ref::<TouchStartEvent>().is_some() {
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
        self.event_registration.remove_event_listener(&event_type, id)
    }

    pub fn set_as_layout_root(&mut self, layout_root: Option<Box<dyn LayoutRoot>>) {
        self.layout_root = layout_root;
    }

    pub fn mark_dirty(&mut self, layout_dirty: bool) {
        if layout_dirty && self.style.yoga_node.get_own_context_mut().is_some() {
            self.style.yoga_node.mark_dirty();
        }

        if layout_dirty {
            let parent = self.get_parent();
            if let Some(layout_root) = &mut self.layout_root {
                let should_propagate_dirty = layout_root.should_propagate_dirty();
                self.set_dirty_state_recurse(true);
                if should_propagate_dirty {
                    if let Some(mut p) = parent {
                        p.mark_dirty(true);
                        return;
                    }
                }
            } else if let Some(mut parent) = parent {
                parent.mark_dirty(true);
                return;
            } else {
                self.set_dirty_state_recurse(true);
            }
            self.with_window(|win| {
                win.invalid_layout();
            });
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
                w.render_tree.invalid_element(element);
                w.notify_update();
            });
        }
    }

    pub fn is_layout_dirty(&self) -> bool {
        self.dirty_flags.is_layout_dirty()
    }

    pub fn ensure_layout_update(&mut self) {
        if self.is_layout_dirty() {
            self.request_layout();
            assert_eq!(false, self.is_layout_dirty());
        }
    }

    fn request_layout(&mut self) {
        if let Some(lr) = &mut self.layout_root {
            lr.update_layout();
        } else if let Some(mut p) = self.get_parent() {
            p.request_layout();
        } else {
            panic!("Failed to layout elements");
        }
    }

    fn set_dirty_state_recurse(&mut self, dirty: bool) {
        if self.is_layout_dirty() != dirty {
            if dirty {
                self.dirty_flags.insert(StyleDirtyFlags::LayoutDirty);
            } else {
                self.dirty_flags.remove(StyleDirtyFlags::LayoutDirty);
            }
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

    pub fn before_layout_recurse(&mut self) {
        self.backend.before_layout();
        for c in &mut self.children {
            c.before_layout_recurse();
        }
    }

    pub fn on_layout_update(&mut self) {
        self.dirty_flags.remove(StyleDirtyFlags::LayoutDirty);
        //TODO emit size change
        let origin_bounds = self.get_origin_bounds();
        if origin_bounds != self.rect {
            self.rect = origin_bounds.clone();
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
            self.backend.handle_origin_bounds_change(&origin_bounds);
            for child in &mut self.get_children() {
                if let Some(p) = &mut child.layout_root {
                    p.update_layout();
                }
                child.on_layout_update();
            }
        }
    }

    pub fn get_border_path_mut(&mut self) -> &mut BorderPath {
        let bounds = self.get_bounds();
        let border_widths = self.get_border_width();
        let border_widths = [border_widths.0, border_widths.1, border_widths.2, border_widths.3];
        let bp = BorderPath::new(bounds.width, bounds.height, self.style.border_radius, border_widths);
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
        self.focusable && self.backend.clone().can_focus()
    }

    pub(crate) fn select_style(&mut self) {
        if self.element_type == ElementType::Widget {
            let (style, pseudo_styles) = CSS_MANAGER.with_borrow(|cm| {
                cm.match_styles(&self)
            });
            if self.style_list.set_selector_style(style) {
                self.mark_style_dirty();
            }
            if !pseudo_styles.is_empty() {
                let mut ps = HashMap::new();
                for (k, v) in pseudo_styles {
                    let mut style_props = StyleList::parse_style(&v);
                    for p in &mut style_props {
                        p.resolve(&self.style_list.variables)
                    }
                    ps.insert(k, style_props);
                }
                self.backend.accept_pseudo_styles(ps);
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

    fn select_style_recurse(&mut self) {
        self.select_style();
        for mut child in self.get_children() {
            child.select_style_recurse();
        }
    }

}

impl ElementWeak {
    pub fn emit<T: ViewEvent + 'static>(&self, event: T) {
        if let Ok(mut el) = self.upgrade() {
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

#[mrc_object]
pub struct Element {
    id: u32,
    backend: Mrc<Box<dyn ElementBackend>>,
    pub(crate) parent: Option<ElementWeak>,
    children: Vec<Element>,
    window: Option<WindowWeak>,
    event_registration: EventRegistration<ElementWeak>,
    pub style: StyleNode,
    pub(crate) animation_style_props: HashMap<StylePropKey, StyleProp>,
    pub(crate) hover: bool,
    auto_focus: bool,
    dirty_flags: StyleDirtyFlags,
    element_type: ElementType,

    applied_style: HashMap<StylePropKey, StyleProp>,
    // animation_instance: Option<AnimationInstance>,


    scroll_top: f32,
    scroll_left: f32,
    draggable: bool,
    cursor: CursorIcon,
    rect: base::Rect,
    resource_table: ResourceTable,
    children_decoration: (f32, f32, f32, f32),

    layout_root: Option<Box<dyn LayoutRoot>>,
    //TODO rename
    pub need_snapshot: bool,
    pub render_object_idx: Option<usize>,
    border_path: BorderPath,
    style_list: StyleList,
    focusable: bool,
    pub(crate) classes: HashSet<String>,
    pub(crate) attributes: HashMap<String, String>,
}

pub struct PaintInfo {
    pub scroll_left: f32,
    pub scroll_top: f32,
}

// js_weak_value!(Element, ElementWeak);
js_value!(Element);
js_auto_upgrade!(ElementWeak, Element);


impl ElementData {

    pub fn new<T: ElementBackend + 'static>(backend: T) -> Self {
        let id = NEXT_ELEMENT_ID.get();
        NEXT_ELEMENT_ID.set(id + 1);
        Self {
            id,
            backend: Mrc::new(Box::new(backend)),
            parent: None,
            window: None,
            event_registration: EventRegistration::new(),
            style: StyleNode::new(),
            animation_style_props: HashMap::new(),
            applied_style: HashMap::new(),
            hover: false,
            element_type: ElementType::Inner,

            scroll_top: 0.0,
            scroll_left: 0.0,
            draggable: false,
            cursor: CursorIcon::Default,
            rect: base::Rect::empty(),
            resource_table: ResourceTable::new(),
            children_decoration: (0.0, 0.0, 0.0, 0.0),
            children: Vec::new(),
            layout_root: None,
            need_snapshot: false,
            render_object_idx: None,
            border_path: BorderPath::new(0.0, 0.0, [0.0; 4], [0.0; 4]),
            style_list: StyleList::new(),
            auto_focus: false,
            focusable: false,
            dirty_flags: StyleDirtyFlags::LayoutDirty,
            classes: HashSet::new(),
            attributes: HashMap::new(),
        }
    }

}

pub struct EmptyElementBackend {

}

impl ElementBackend for EmptyElementBackend {
    fn create(_ele: &mut Element) -> Self {
        Self {}
    }

    fn get_name(&self) -> &str {
        "Empty"
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }
}

pub trait ElementBackend : 'static {

    fn create(element: &mut Element) -> Self where Self: Sized;

    fn get_name(&self) -> &str;

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend>;

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

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        if let Some(base) = self.get_base_mut() {
            base.on_event(event, ctx);
        }
    }

    fn execute_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
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

    fn apply_style_prop(&mut self, prop: &StyleProp, length_ctx: &LengthContext) -> Option<(bool, bool, ResolvedStyleProp)> {
        if let Some(base) = self.get_base_mut() {
            base.apply_style_prop(prop, length_ctx)
        } else {
            None
        }
    }

    fn accept_pseudo_styles(&mut self, styles: HashMap<String, Vec<ParsedStyleProp>>) {
        if let Some(base) = self.get_base_mut() {
            base.accept_pseudo_styles(styles);
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        if let Some(base) = self.get_base_mut() {
            base.on_attribute_changed(key, value);
        }
    }

    fn can_focus(&mut self) -> bool {
        if let Some(base) = self.get_base_mut() {
            base.can_focus()
        } else {
            true
        }
    }

    fn bind_js_listener(&mut self, event_type: &str, listener: JsValue) -> Option<u32> {
        if let Some(base) = self.get_base_mut() {
            base.bind_js_listener(event_type, listener)
        } else {
            None
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