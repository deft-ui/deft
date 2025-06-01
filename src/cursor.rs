use crate::element::Element;
use winit::window::CursorIcon::*;
use winit::window::Cursor;

pub fn search_cursor(element: &Element) -> Cursor {
    let cursor = element.get_cursor();
    if cursor != Cursor::Icon(Default) {
        return cursor;
    }
    if let Some(p) = element.get_parent() {
        p.get_cursor()
    } else {
        Cursor::Icon(Default)
    }
}
