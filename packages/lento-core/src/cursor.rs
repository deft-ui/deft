use winit::window::CursorIcon;
use winit::window::CursorIcon::*;
use crate::element::ElementRef;

pub fn parse_cursor(str: &str) -> Option<CursorIcon> {
    let icon = match str {
        "default" => Default,
        "context-menu" => ContextMenu,
        "help" => Help,
        "pointer" => Pointer,
        "progress" => Progress,
        "wait" => Wait,
        "cell" => Cell,
        "crosshair" => Crosshair,
        "text" => Text,
        "vertical-text" => VerticalText,
        "alias" => Alias,
        "copy" => Copy,
        "move" => Move,
        "no-drop" => NoDrop,
        "not-allowed" => NotAllowed,
        "grab" => Grab,
        "grabbing" => Grabbing,
        "e-resize" => EResize,
        "n-resize" => NResize,
        "ne-resize" => NeResize,
        "nw-resize" => NwResize,
        "s-resize" => SResize,
        "se-resize" => SeResize,
        "sw-resize" => SwResize,
        "w-resize" => WResize,
        "ew-resize" => EwResize,
        "ns-resize" => NsResize,
        "nesw-resize" => NeswResize,
        "nwse-resize" => NwseResize,
        "col-resize" => ColResize,
        "row-resize" => RowResize,
        "all-scroll" => AllScroll,
        "zoom-in" => ZoomIn,
        "zoom-out" => ZoomOut,
        _ => return None,
    };
    Some(icon)
}


pub fn search_cursor(element: &ElementRef) -> CursorIcon {
    let cursor = element.get_cursor();
    if cursor != Default {
        return cursor
    }
    if let Some(p) = element.get_parent() {
        p.get_cursor()
    } else {
        Default
    }
}
