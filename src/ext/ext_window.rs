use crate as deft;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use winit::event::WindowEvent;
use winit::window::WindowId as WinitWindowId;

use crate::element::Element;
use crate::state::State;
use crate::window::{Window, WindowHandle};
use crate::{js_deserialize, js_value, js_weak_value};

thread_local! {
    pub static WINDOWS: RefCell<HashMap<i32, WindowHandle >> = RefCell::new(HashMap::new());
    pub static WINIT_TO_WINDOW: RefCell<HashMap<WinitWindowId, WindowHandle >> = RefCell::new(HashMap::new());
    pub static MODAL_TO_OWNERS: RefCell<HashMap<WinitWindowId, WindowHandle >> = RefCell::new(HashMap::new());
}

pub const WINDOW_TYPE_NORMAL: &str = "normal";
pub const WINDOW_TYPE_MENU: &str = "menu";

pub const ELEMENT_TYPE_CONTAINER: &str = "container";
pub const ELEMENT_TYPE_LABEL: &str = "label";
pub const ELEMENT_TYPE_BUTTON: &str = "button";
pub const ELEMENT_TYPE_ENTRY: &str = "entry";
pub const ELEMENT_TYPE_SCROLL: &str = "scroll";
pub const ELEMENT_TYPE_TEXT_INPUT: &str = "text-input";
pub const ELEMENT_TYPE_TEXT_EDIT: &str = "text-edit";
pub const ELEMENT_TYPE_IMAGE: &str = "image";
pub const ELEMENT_TYPE_BODY: &str = "body";
pub const ELEMENT_TYPE_PARAGRAPH: &str = "paragraph";
pub const ELEMENT_TYPE_CHECKBOX: &str = "checkbox";
pub const ELEMENT_TYPE_RADIO: &str = "radio";
pub const ELEMENT_TYPE_RADIO_GROUP: &str = "radio-group";
pub const ELEMENT_TYPE_RICH_TEXT: &str = "rich-text";

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
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,
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
                m.iter()
                    .find(|(_, o)| {
                        if let Ok(o) = o.upgrade_mut() {
                            o.get_window_id() == window_id
                        } else {
                            false
                        }
                    })
                    .is_some()
            });
            if has_modal {
                debug!("modal window found");
                return;
            }
        }
    }
    let mut window = WINIT_TO_WINDOW.with_borrow_mut(|m| match m.get_mut(&window_id) {
        None => None,
        Some(f) => Some(f.clone()),
    });
    if let Some(window) = &mut window {
        if &WindowEvent::CloseRequested == &event {
            if let Ok(mut f) = window.upgrade_mut() {
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

js_value!(WindowHandle);
