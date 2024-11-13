use crate as lento;
use crate::app::AppEvent;
use crate::base::{Event, EventHandler, EventRegistration};
use crate::event_loop::get_event_proxy;
use crate::mrc::Mrc;
use anyhow::Error;
use ksni::menu::StandardItem;
use ksni::{Handle, Tray};
use lento_macros::{js_func, js_methods, mrc_object};
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use winit::event_loop::EventLoopProxy;
use crate::{define_resource, js_deserialize, js_value};

struct MyTray {
    tray_id: String,
    activate_callback: Box<dyn FnMut()>,
    title: String,
    icon: String,
    menus: Vec<TrayMenu>,
    menu_active_cb_generator: Box<dyn Fn(&str) -> Box<dyn Fn(&mut MyTray)>>,
}

thread_local! {
    pub static NEXT_TRAY_ID: Cell<u32> = Cell::new(1);
}

impl Tray for MyTray {
    fn activate(&mut self, _x: i32, _y: i32) {
        println!("Activate");
        (self.activate_callback)();
    }
    fn icon_name(&self) -> String {
        self.icon.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    // NOTE: On some system trays, `id` is a required property to avoid unexpected behaviors
    fn id(&self) -> String {
        self.tray_id.clone()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut list: Vec<ksni::MenuItem<MyTray>> = Vec::new();
        for m in &self.menus {
            list.push(StandardItem {
                label: m.label.to_string(),
                activate: (self.menu_active_cb_generator)(&m.id),
                ..Default::default()
            }.into());
        }
        return list;
    }
}

#[mrc_object]
pub struct SystemTray {
    event_loop_proxy: EventLoopProxy<AppEvent>,
    event_registration: EventRegistration<SystemTray>,
    id: u32,
    handle: Handle<MyTray>,
}

js_value!(SystemTray);
//TODO remove
define_resource!(SystemTray);

unsafe impl Send for MyTray {}

unsafe impl Sync for MyTray {}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrayMenu {
    pub id: String,
    pub label: String,
}

js_deserialize!(TrayMenu);


#[js_methods]
impl SystemTray {

    #[js_func]
    pub fn tray_create(id: String) -> Result<SystemTray, Error> {
        let tray = SystemTray::create_tray(&id, get_event_proxy());
        Ok(tray)
    }

    fn create_tray(tray_id: &str, event_loop_proxy: EventLoopProxy<AppEvent>) -> Self {
        let inner_id = NEXT_TRAY_ID.get();
        NEXT_TRAY_ID.set(inner_id + 1);

        let service = ksni::TrayService::new(MyTray {
            tray_id: tray_id.to_string(),
            activate_callback: Box::new(|| {}),
            title: "".to_string(),
            icon: "".to_string(),
            menus: Vec::new(),
            menu_active_cb_generator: Box::new(|_| Box::new(|_| {})),
        });
        let handle = service.handle();
        service.spawn();

        let inner = Mrc::new(SystemTrayData {
            event_loop_proxy,
            event_registration: EventRegistration::new(),
            id: inner_id,
            handle,
        });
        let inst = Self {
            inner
        };

        let inst_weak = inst.inner.as_weak();
        let inst_weak2 = inst.inner.as_weak();
        inst.inner.handle.update(move |t| {
            t.activate_callback = Box::new(move || {
                if let Ok(st) = inst_weak.upgrade() {
                    let mut str = SystemTray {
                        inner: st,
                    };
                    str.activate_ts();
                }
            });
            t.menu_active_cb_generator = Box::new(move |id| {
                let inst_weak2 = inst_weak2.clone();
                let id = id.to_string();
                Box::new(move |_| {
                    if let Ok(st) = inst_weak2.upgrade() {
                        let mut str = SystemTray {
                            inner: st,
                        };
                        str.emit_menu_click(id.to_string());
                    }
                })
            });
        });
        inst
    }

    pub fn add_event_listener(&mut self, event_type: String, handler: Box<EventHandler<SystemTray>>) -> u32 {
        self.inner.event_registration.add_event_listener(&event_type, handler)
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: i32) {
        self.inner.event_registration.remove_event_listener(&event_type, id as u32);
    }

    #[js_func]
    pub fn bind_event(&mut self, event_name: String, callback: JsValue) -> u32 {
        self.event_registration.add_js_event_listener(&event_name, callback) as u32
    }


    #[js_func]
    pub fn get_id(&self) -> u32 {
        self.inner.id
    }

    #[js_func]
    pub fn set_title(&self, title: String) {
        self.inner.handle.update(move |t| {
            t.title = title;
        })
    }

    #[js_func]
    pub fn set_icon(&self, icon: String) {
        self.inner.handle.update(move |t| {
            t.icon = icon;
        });
    }

    #[js_func]
    pub fn set_menus(&self, menus: Vec<TrayMenu>) {
        self.inner.handle.update(move |t| {
            t.menus = menus;
        });
    }

    fn emit_menu_click(&mut self, menu_id: String) {
        let mut sr = self.clone();
        self.inner.event_loop_proxy.send_event(AppEvent::CallbackWithEventLoop(Box::new(move |_| {
            let mut event = Event::new("menuclick", menu_id, sr.clone());
            sr.inner.event_registration.emit_event(&mut event);
        }))).unwrap();
    }

    fn activate_ts(&mut self) {
        let mut sr = self.clone();
        self.inner.event_loop_proxy.send_event(AppEvent::CallbackWithEventLoop(Box::new(move |_| {
            let mut event = Event::new("activate", (), sr.clone());
            sr.inner.event_registration.emit_event(&mut event);
        }))).unwrap();
    }
}

