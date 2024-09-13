use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{anyhow, Error};
use quick_js::{JsValue, ResourceValue};
use serde::{Deserialize, Serialize};
use skia_window::skia_window::SkiaWindow;
use winit::dpi::{LogicalPosition, LogicalSize, Size};
use winit::dpi::Position::Logical;
use winit::event::WindowEvent;
#[cfg(feature = "x11")]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{Window, WindowId};

use crate::app::{exit_app};
use crate::element::{ElementBackend, ElementRef};
use crate::event_loop::{run_with_event_loop, send_event};
use crate::export_js_object_api;
use crate::frame::{FrameRef, FrameWeak};
use crate::js::js_runtime::JsContext;
use crate::js::js_value_util::{FromJsValue2, ToJsValue2};
use crate::mrc::Mrc;


thread_local! {
    pub static FRAMES: RefCell<HashMap<i32, FrameRef>> = RefCell::new(HashMap::new());
    pub static WINDOW_TO_FRAME: RefCell<HashMap<WindowId, FrameWeak>> = RefCell::new(HashMap::new());
    pub static MODAL_TO_OWNERS: RefCell<HashMap<WindowId, WindowId>> = RefCell::new(HashMap::new());
}

pub const VIEW_TYPE_CONTAINER: i32 = 1;
pub const VIEW_TYPE_LABEL: i32 = 2;
pub const VIEW_TYPE_BUTTON: i32 = 3;
pub const VIEW_TYPE_ENTRY: i32 = 4;
pub const VIEW_TYPE_SCROLL: i32 = 7;
pub const VIEW_TYPE_TEXT_EDIT: i32 = 8;
pub const VIEW_TYPE_IMAGE: i32 = 9;

pub type ViewId = i32;

pub type FrameId = i32;

#[derive(Debug, Serialize, Deserialize)]
pub struct FrameAttrs {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub title: Option<String>,
    pub resizable: Option<bool>,
    pub decorations: Option<bool>,
    pub override_redirect: Option<bool>,
    pub position: Option<(f32, f32)>,
}


pub fn create_frame(attrs: FrameAttrs) -> Result<FrameWeak, Error> {

    let mut attributes = Window::default_attributes();
    if let Some(t) = &attrs.title {
        attributes.title = t.to_string();
    } else {
        attributes.title = "".to_string();
    }
    attributes.resizable = attrs.resizable.unwrap_or(true);
    attributes.decorations = attrs.decorations.unwrap_or(true);
    let size = LogicalSize {
        width: attrs.width.unwrap_or(800.0) as f64,
        height: attrs.height.unwrap_or(600.0) as f64,
    };
    attributes.inner_size = Some(Size::Logical(size));
    #[cfg(feature = "x11")]
    {
        attributes = attributes.with_override_redirect(attrs.override_redirect.unwrap_or(false));
    }
    if let Some(position) = attrs.position {
        attributes.position = Some(Logical(LogicalPosition {
            x: position.0 as f64,
            y: position.1 as f64,
        }));
    }
    let frame = FrameRef::new(attributes);
    let window_id = frame.get_window_id();
    let frame_weak = frame.as_weak();
    FRAMES.with_borrow_mut(|m| m.insert(frame.get_id(), frame));
    WINDOW_TO_FRAME.with_borrow_mut(|m| m.insert(window_id, frame_weak.clone()));
    Ok(frame_weak)
}

pub fn frame_set_modal(mut frame: FrameWeak, owner: FrameWeak) -> Result<(), Error> {
    frame.upgrade_mut(|f| {
        owner.upgrade_mut(|o| {
            let _ = f.set_modal(o);
            let frame_id = f.get_window_id();
            MODAL_TO_OWNERS.with_borrow_mut(|m| m.insert(frame_id, o.get_window_id()));
        })
    });
    Ok(())
}

pub fn frame_close(frame: FrameWeak) -> Result<(), Error> {
    frame.upgrade_mut(|frame| {
        let window_id = frame.get_window_id();
        if frame.allow_close() {
            WINDOW_TO_FRAME.with_borrow_mut(|m| m.remove(&window_id));
            MODAL_TO_OWNERS.with_borrow_mut(|m| m.remove(&window_id));
            FRAMES.with_borrow_mut(|m| {
                m.remove(&frame.get_id());
                if m.is_empty() {
                    let _ = exit_app(0);
                }
            });
        }
    });
    Ok(())
}

pub fn handle_window_event(window_id: WindowId, event: WindowEvent) {
    match &event {
        WindowEvent::Resized(_) => {}
        WindowEvent::Moved(_) => {}
        WindowEvent::Destroyed => {}
        WindowEvent::ModifiersChanged(_) => {}
        WindowEvent::ScaleFactorChanged { .. } => {}
        WindowEvent::ThemeChanged(_) => {}
        WindowEvent::Occluded(_) => {}
        WindowEvent::RedrawRequested => {}
        _ => {
            let has_modal = MODAL_TO_OWNERS.with_borrow_mut(|m| {
                m.iter().find(|(_, o)| o == &&window_id).is_some()
            });
            if has_modal {
                return
            }
        }
    }
    let mut frame = WINDOW_TO_FRAME.with_borrow_mut(|m| {
        match m.get_mut(&window_id) {
            None => None,
            Some(f) => Some(f.clone())
        }
    });
    if let Some(frame) = &mut frame {
        if &WindowEvent::CloseRequested == &event {
            let _ = frame_close(frame.clone());
        } else {
            frame.upgrade_mut(|frame| {
                frame.handle_event(event);
            });
        }
    }
}

impl FrameWeak {
    pub fn set_body(&mut self, body: ElementRef) {
        self.upgrade_mut(|f| {
            f.set_body(body)
        });
    }

    pub fn set_title(&mut self, title: String) {
        self.upgrade_mut(|f| {
            f.set_title(title)
        });
    }

    pub fn bind_event(&mut self, name: String, callback: JsValue) {
        self.upgrade_mut(|f| {
            f.bind_event(name, callback)
        });
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.upgrade_mut(|f| {
            f.set_visible(visible)
        });
    }

    pub fn remove_event_listener(&mut self, name: String, eid: u32) {
        self.upgrade_mut(|f| {
            f.remove_event_listener(name, eid)
        });
    }

}

// Js Api

impl ToJsValue2 for FrameWeak {
    fn to_js_value(self) -> Result<JsValue, Error> {
        Ok(JsValue::Resource(ResourceValue {
            resource: Rc::new(RefCell::new(self)),
        }))
    }
}

impl FromJsValue2 for FrameWeak {
    fn from_js_value(value: JsValue) -> Result<Self, Error> {
        if let Some(r) = value.as_resource(|r: &mut FrameWeak| r.clone()) {
            //TODO fix
            Ok(r)
        } else {
            Err(anyhow!("invalid value"))
        }
    }
}