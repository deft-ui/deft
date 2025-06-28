use crate as deft;
use crate::js::JsError;
use clipboard::{ClipboardContext, ClipboardProvider};
use deft_macros::js_methods;
use std::error::Error;

fn to_js_error(error: Box<dyn Error>) -> JsError {
    JsError::new(error.to_string())
}

pub struct Clipboard;

#[js_methods]
impl Clipboard {
    #[js_func]
    pub fn write_text(text: String) -> Result<(), JsError> {
        #[cfg(target_os = "android")]
        {
            crate::android::clipboard_write_text(&text)?;
            return Ok(());
        }
        let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(to_js_error)?;
        ctx.set_contents(text).map_err(to_js_error)?;
        Ok(())
    }

    #[js_func]
    pub fn read_text() -> Result<String, JsError> {
        let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(to_js_error)?;
        let text = ctx.get_contents().map_err(to_js_error)?;
        Ok(text)
    }
}
