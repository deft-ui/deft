use crate as deft;
use std::cell::RefCell;
use std::collections::HashMap;
use anyhow::{anyhow, Error};
use log::{debug, info, warn};
use quick_js::{JsValue, ResourceValue, ValueError};
use serde::{Deserialize, Serialize};
use winit::event::WindowEvent;
#[cfg(x11_platform)]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{WindowId as WinitWindowId};

use crate::app::{exit_app};
use crate::element::{ElementBackend, Element};
use crate::{js_deserialize, js_weak_value};
use crate::window::{Window, WindowWeak};


thread_local! {
    pub static WINDOWS: RefCell<HashMap<i32, Window>> = RefCell::new(HashMap::new());
    pub static WINIT_TO_WINDOW: RefCell<HashMap<WinitWindowId, WindowWeak>> = RefCell::new(HashMap::new());
    pub static MODAL_TO_OWNERS: RefCell<HashMap<WinitWindowId, WinitWindowId>> = RefCell::new(HashMap::new());
}

pub const WINDOW_TYPE_NORMAL: &str = "normal";
pub const WINDOW_TYPE_MENU: &str = "menu";

pub const ELEMENT_TYPE_CONTAINER: i32 = 1;
pub const ELEMENT_TYPE_LABEL: i32 = 2;
pub const ELEMENT_TYPE_BUTTON: i32 = 3;
pub const ELEMENT_TYPE_ENTRY: i32 = 4;
pub const ELEMENT_TYPE_SCROLL: i32 = 7;
pub const ELEMENT_TYPE_TEXT_EDIT: i32 = 8;
pub const ELEMENT_TYPE_IMAGE: i32 = 9;
pub const ELEMENT_TYPE_BODY: i32 = 10;
pub const ELEMENT_TYPE_PARAGRAPH: i32 = 11;
pub const ELEMENT_TYPE_CHECKBOX: i32 = 12;
pub const ELEMENT_TYPE_RADIO: i32 = 13;
pub const ELEMENT_TYPE_RADIO_GROUP: i32 = 14;
pub const ELEMENT_TYPE_RICH_TEXT: i32 = 15;

pub type ElementId = i32;

pub type WindowId = i32;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowAttrs {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub title: Option<String>,
    pub resizable: Option<bool>,
    pub decorations: Option<bool>,
    pub override_redirect: Option<bool>,
    pub position: Option<(f32, f32)>,
    pub visible: Option<bool>,
    pub window_type: Option<String>,
    pub preferred_renderers: Option<Vec<String>>,
}

js_deserialize!(WindowAttrs);

pub fn handle_window_event(window_id: WinitWindowId, event: WindowEvent) {
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
                debug!("modal window found");
                return
            }
        }
    }
    let mut window = WINIT_TO_WINDOW.with_borrow_mut(|m| {
        match m.get_mut(&window_id) {
            None => None,
            Some(f) => Some(f.clone())
        }
    });
    if let Some(window) = &mut window {
        if &WindowEvent::CloseRequested == &event {
            if let Ok(mut f) = window.upgrade() {
                let _ = f.close();
            }
        } else {
            if let Ok(mut window) = window.upgrade_mut() {
                window.handle_event(event);
            }
        }
    } else {
        warn!("No window found: {:?}", window_id);
    }
}

impl WindowWeak {

    pub fn set_body(&mut self, body: Element) {
        if let Ok(mut f) = self.upgrade_mut() {
            f.set_body(body)
        }
    }

}

js_weak_value!(Window, WindowWeak);