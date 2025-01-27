use crate as deft;
use std::cell::RefCell;
use std::collections::HashMap;

use anyhow::{anyhow, Error};
use quick_js::{JsValue, ResourceValue, ValueError};
use serde::{Deserialize, Serialize};
use winit::event::WindowEvent;
#[cfg(feature = "x11")]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{Window, WindowId};

use crate::app::{exit_app};
use crate::element::{ElementBackend, Element};
use crate::{js_deserialize, js_weak_value};
use crate::frame::{Frame, FrameWeak};


thread_local! {
    pub static FRAMES: RefCell<HashMap<i32, Frame>> = RefCell::new(HashMap::new());
    pub static WINDOW_TO_FRAME: RefCell<HashMap<WindowId, FrameWeak>> = RefCell::new(HashMap::new());
    pub static MODAL_TO_OWNERS: RefCell<HashMap<WindowId, WindowId>> = RefCell::new(HashMap::new());
}

pub const FRAME_TYPE_NORMAL: &str = "normal";
pub const FRAME_TYPE_MENU: &str = "menu";

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
#[serde(rename_all = "camelCase")]
pub struct FrameAttrs {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub title: Option<String>,
    pub resizable: Option<bool>,
    pub decorations: Option<bool>,
    pub override_redirect: Option<bool>,
    pub position: Option<(f32, f32)>,
    pub visible: Option<bool>,
    pub frame_type: Option<String>,
}

js_deserialize!(FrameAttrs);

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
            if let Ok(mut f) = frame.upgrade() {
                let _ = f.close();
            }
        } else {
            if let Ok(mut frame) = frame.upgrade_mut() {
                frame.handle_event(event);
            }
        }
    }
}

impl FrameWeak {

    pub fn set_body(&mut self, body: Element) {
        if let Ok(mut f) = self.upgrade_mut() {
            f.set_body(body)
        }
    }

}

// Js Api
//TODO remove
// define_resource!(FrameWeak);

js_weak_value!(Frame, FrameWeak);