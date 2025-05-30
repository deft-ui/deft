use crate as deft;
use crate::base::{Event, EventHandler, EventRegistration};
use crate::event_loop::{create_event_loop_fn_mut, create_event_loop_proxy, AppEventProxy};
use crate::{js_deserialize, js_value};
use anyhow::Error;
use deft_macros::{js_methods, mrc_object};
use deft_tray::{Tray, TrayMenu};
use image::ImageReader;
use quick_js::JsValue;
use std::cell::Cell;

thread_local! {
    pub static NEXT_TRAY_ID: Cell<u32> = Cell::new(1);
}

#[mrc_object]
pub struct SystemTray {
    event_loop_proxy: AppEventProxy,
    event_registration: EventRegistration<SystemTray>,
    id: u32,
    tray_impl: Tray,
}

js_value!(SystemTray);

js_deserialize!(TrayMenu);

#[js_methods]
impl SystemTray {
    #[js_func]
    pub fn create(id: String) -> Result<SystemTray, Error> {
        let tray = SystemTray::create_tray(&id, create_event_loop_proxy());
        Ok(tray)
    }

    fn create_tray(tray_id: &str, event_loop_proxy: AppEventProxy) -> Self {
        let inner_id = NEXT_TRAY_ID.get();
        NEXT_TRAY_ID.set(inner_id + 1);

        let tray_impl = Tray::new(tray_id);

        let mut inst = SystemTrayData {
            event_loop_proxy,
            event_registration: EventRegistration::new(),
            id: inner_id,
            tray_impl,
        }
        .to_ref();

        let mut me = inst.clone();
        let mut menu_active_callback = create_event_loop_fn_mut(move |menu_id: String| {
            let mut event = Event::new("menuclick", menu_id, me.clone());
            me.event_registration.emit_event(&mut event);
        });

        let mut sr = inst.clone();
        let mut activate_callback = create_event_loop_fn_mut(move |()| {
            let mut event = Event::new("activate", (), sr.clone());
            sr.event_registration.emit_event(&mut event);
        });
        inst.tray_impl.set_active_callback(Box::new(move || {
            activate_callback.call(());
        }));
        inst.tray_impl.set_menu_click_callback(Box::new(move |id| {
            menu_active_callback.call(id);
        }));
        inst
    }

    pub fn add_event_listener(
        &mut self,
        event_type: String,
        handler: Box<EventHandler<SystemTray>>,
    ) -> u32 {
        self.inner
            .event_registration
            .add_event_listener(&event_type, handler)
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: i32) {
        self.inner
            .event_registration
            .remove_event_listener(&event_type, id as u32);
    }

    #[js_func]
    pub fn bind_event(&mut self, event_name: String, callback: JsValue) -> u32 {
        self.event_registration
            .add_js_event_listener(&event_name, callback) as u32
    }

    #[js_func]
    pub fn get_id(&self) -> u32 {
        self.id
    }

    #[js_func]
    pub fn set_title(&mut self, title: String) {
        self.tray_impl.set_title(&title);
    }

    #[js_func]
    pub fn set_icon(&mut self, icon: String) {
        #[cfg(target_os = "linux")]
        self.tray_impl.set_icon(&icon);
        #[cfg(not(target_os = "linux"))]
        if let Some((img, width, height)) = Self::load_image(&icon) {
            self.tray_impl.set_icon_from_rgba(img, width, height);
        }
    }

    #[js_func]
    pub fn set_menus(&mut self, menus: Vec<TrayMenu>) {
        self.tray_impl.set_menus(menus);
    }

    fn load_image(path: &str) -> Option<(Vec<u8>, u32, u32)> {
        let img = ImageReader::open(path).ok()?.decode().ok()?;
        let rgba_img = img.to_rgba8();
        let width = img.width();
        let height = img.height();
        Some((rgba_img.into_raw(), width, height))
    }
}
