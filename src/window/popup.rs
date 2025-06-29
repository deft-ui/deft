use crate as deft;
use crate::base::Rect;
use crate::element::body::Body;
use crate::element::{Element, ElementBackend, ElementType};
use crate::event::ClickEventListener;
use crate::ext::ext_window::WindowAttrs;
use crate::platform::support_multiple_windows;
use crate::window::page::PageWeak;
use crate::window::{Window, WindowHandle, WindowResizeEventListener};
use crate::winit::dpi::Position;
use crate::{js_weak_value, ok_or_return};
use deft_macros::{js_methods, mrc_object};
use winit::dpi::LogicalPosition;
#[cfg(windows)]
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::WindowAttributes;

enum PopupWrapper {
    Window(WindowHandle),
    Page(PageWeak),
}

#[mrc_object]
pub struct Popup {
    wrapper: PopupWrapper,
}

js_weak_value!(Popup, PopupWeak);

#[js_methods]
impl Popup {
    pub fn new(element: Element, target: Rect, owner_handle: &WindowHandle) -> Popup {
        Self::new_ex(element, target, owner_handle, true)
    }
    pub fn new_ex(
        element: Element,
        target: Rect,
        owner_handle: &WindowHandle,
        focusable: bool,
    ) -> Popup {
        //TODO no unwrap
        let mut owner = owner_handle.upgrade_mut().unwrap();
        if support_multiple_windows() {
            let (win_x, win_y) = owner.inner_position();
            let pos_x = target.x + win_x;
            let pos_y = target.bottom() + win_y;
            let window_attrs = WindowAttrs {
                width: None,
                height: None,
                title: None,
                resizable: Some(false),
                decorations: Some(false),
                override_redirect: Some(true),
                position: Some((pos_x, pos_y)),
                visible: None,
                closable: None,
                minimizable: None,
                maximizable: None,
                window_type: Some("menu".to_string()),
                preferred_renderers: Some(vec!["softbuffer".to_string()]),
            };
            let winit_attrs = WindowAttributes::default();
            #[cfg(windows_platform)]
            let winit_attrs = winit_attrs.with_skip_taskbar(true).with_active(false);
            let window_handle = Window::create_with_raw_attrs(window_attrs, winit_attrs).unwrap();
            //TODO no unwrap
            let mut window = window_handle.upgrade_mut().unwrap();
            let mut body = Element::create(Body::create);
            body.tag = "body".to_string();
            body.set_element_type(ElementType::Widget);
            let _ = body.add_child(element.clone(), 0);
            let _ = window.set_body(body);
            let current_monitor = owner.window.current_monitor();
            let window_weak = window_handle.clone();
            window.register_event_listener(WindowResizeEventListener::new(move |e, _| {
                if let Some(m) = &current_monitor {
                    let window = ok_or_return!(window_weak.upgrade_mut());
                    let content_width = e.width as f32;
                    let content_height = e.height as f32;
                    let scale_factor = m.scale_factor();
                    let monitor_size = m.size().to_logical::<f32>(scale_factor);
                    let new_pos_y =
                        fix_pos(pos_y, target.height, content_height, monitor_size.height);
                    let new_pos_x =
                        fix_pos(pos_x, -target.width, content_width, monitor_size.width);
                    if new_pos_x != pos_x || new_pos_y != pos_y {
                        window
                            .window
                            .set_outer_position(Position::Logical(LogicalPosition {
                                x: new_pos_x as f64,
                                y: new_pos_y as f64,
                            }))
                    }
                }
            }));

            PopupData {
                wrapper: PopupWrapper::Window(window_handle),
            }
            .to_ref()
        } else {
            let page = owner.create_page_ex(element, target.x, target.bottom(), focusable);
            let page_weak = page.as_weak();
            page.get_body()
                .clone()
                .register_event_listener(ClickEventListener::new(move |_e, _ctx| {
                    if let Ok(p) = page_weak.upgrade() {
                        p.close();
                    }
                }));
            PopupData {
                wrapper: PopupWrapper::Page(page.as_weak()),
            }
            .to_ref()
        }
    }

    #[js_func]
    pub fn close(&self) {
        match &self.wrapper {
            PopupWrapper::Window(w) => {
                if let Ok(mut w) = w.upgrade_mut() {
                    let _ = w.close();
                }
            }
            PopupWrapper::Page(page) => {
                let page = page.upgrade().unwrap();
                page.close();
            }
        }
    }
}

fn fix_pos(offset: f32, target_length: f32, content_length: f32, max_length: f32) -> f32 {
    if offset + content_length > max_length {
        let offset_new = offset - target_length - content_length;
        if offset_new > 0.0 {
            offset_new
        } else {
            max_length - content_length
        }
    } else {
        offset
    }
}
