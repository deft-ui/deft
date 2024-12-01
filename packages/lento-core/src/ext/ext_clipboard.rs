use std::error::Error;
use clipboard::{ClipboardContext, ClipboardProvider};
use crate as lento;
use crate::js::JsError;

fn to_js_error(error: Box<dyn Error>) -> JsError {
    JsError::new(error.to_string())
}

#[lento_macros::js_func]
pub fn clipboard_write_text(text: String) -> Result<(), JsError> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(to_js_error)?;
    ctx.set_contents(text).map_err(to_js_error)?;
    Ok(())
}

#[lento_macros::js_func]
pub fn clipboard_read_text() -> Result<String, JsError> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(to_js_error)?;
    let text = ctx.get_contents().map_err(to_js_error)?;
    Ok(text)
}