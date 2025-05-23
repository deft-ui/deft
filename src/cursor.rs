use crate::element::Element;
use winit::window::CursorIcon;
use winit::window::CursorIcon::*;

pub fn search_cursor(element: &Element) -> CursorIcon {
    let cursor = element.get_cursor();
    if cursor != Default {
        return cursor;
    }
    if let Some(p) = element.get_parent() {
        p.get_cursor()
    } else {
        Default
    }
}
