use crate as deft;
use crate::base::Id;
use crate::js_module;
use crate::element::CSS_MANAGER;
use crate::ext::ext_window::WINDOWS;
use crate::js::JsError;
use crate::style::css_manager::CSS;
use deft_macros::{js_methods};

pub struct Stylesheet {
    
}

js_module!(Stylesheet, include_str!("./ext/stylesheet.js"));

#[js_methods]
impl Stylesheet {
    #[js_func]
    pub fn add(source: String) -> Result<Id<CSS>, JsError> {
        let id = CSS_MANAGER
            .with_borrow_mut(|manager| manager.add(&source))
            .map_err(|e| JsError::new(format!("failed to add stylesheet: {}", e)));
        refresh_windows_style();
        id
    }

    #[js_func]
    pub fn remove(id: Id<CSS>) -> Result<(), JsError> {
        CSS_MANAGER.with_borrow_mut(|manager| manager.remove(&id));
        refresh_windows_style();
        Ok(())
    }

    #[js_func]
    pub fn update(id: Id<CSS>, source: String) -> Result<(), JsError> {
        CSS_MANAGER.with_borrow_mut(|manager| {
            let _ = manager.update(&id, &source);
        });
        refresh_windows_style();
        Ok(())
    }
}

fn refresh_windows_style() {
    WINDOWS.with_borrow_mut(|windows| {
        for (_, window) in windows.iter_mut() {
            if let Ok(mut window) = window.upgrade() {
                window.get_body_mut().element_mut().update_select_style_recurse()
            }
        }
    });
}

