use std::cell::{Cell, RefCell};
use std::rc::Rc;
use anyhow::{anyhow, Error};
use ksni::{Handle, Tray};
use ksni::menu::StandardItem;
use quick_js::{JsValue, ResourceValue};
use serde::{Deserialize, Serialize};
use winit::event_loop::EventLoopProxy;
use crate::app::AppEvent;
use crate::base::{Event, EventHandler, EventRegistration};
use crate::event_loop::get_event_proxy;
use crate::ext::common::create_event_handler;
use crate::js::js_value_util::{FromJsValue2, ToJsValue2};
use crate::{js_event_bind, js_event_bind2};
use crate::mrc::Mrc;


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

pub struct SystemTray {
    event_loop_proxy: EventLoopProxy<AppEvent>,
    event_registration: EventRegistration<SystemTrayRef>,
    id: u32,
    handle: Handle<MyTray>,
}

#[derive(Clone)]
pub struct SystemTrayRef {
    inner: Mrc<SystemTray>,
}

unsafe impl Send for MyTray {}

unsafe impl Sync for MyTray {}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrayMenu {
    pub id: String,
    pub label: String,
}

impl SystemTrayRef {
    pub fn new(tray_id: &str, event_loop_proxy: EventLoopProxy<AppEvent>) -> Self {
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

        let inner = Mrc::new(SystemTray {
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
                if let Some(st) = inst_weak.upgrade() {
                    let mut str = SystemTrayRef {
                        inner: st,
                    };
                    str.activate_ts();
                }
            });
            t.menu_active_cb_generator = Box::new(move |id| {
                let inst_weak2 = inst_weak2.clone();
                let id = id.to_string();
                Box::new(move |_| {
                    if let Some(st) = inst_weak2.upgrade() {
                        let mut str = SystemTrayRef {
                            inner: st,
                        };
                        str.emit_menu_click(id.to_string());
                    }
                })
            });
        });
        inst
    }

    pub fn add_event_listener(&mut self, event_type: &str, handler: Box<EventHandler<SystemTrayRef>>) -> u32 {
        self.inner.event_registration.add_event_listener(event_type, handler)
    }

    pub fn remove_event_listener(&mut self, event_type: String, id: i32) {
        self.inner.event_registration.remove_event_listener(&event_type, id as u32);
    }

    pub fn bind_event(&mut self, event_name: String, callback: JsValue) -> u32 {
        let handler = create_event_handler(&event_name, callback);
        js_event_bind2!(self, "activate", (), &event_name, handler);
        js_event_bind2!(self, "menuclick", String, &event_name, handler);
        0
    }


    pub fn get_id(&self) -> u32 {
        self.inner.id
    }

    pub fn set_title(&self, title: String) {
        self.inner.handle.update(move |t| {
            t.title = title;
        })
    }

    pub fn set_icon(&self, icon: String) {
        self.inner.handle.update(move |t| {
            t.icon = icon;
        });
    }

    pub fn set_menus(&self, menus: Vec<TrayMenu>) {
        self.inner.handle.update(move |t| {
            t.menus = menus;
        });
    }

    fn emit_menu_click(&mut self, menu_id: String) {
        let mut sr = self.clone();
        self.inner.event_loop_proxy.send_event(AppEvent::CallbackWithEventLoop(Box::new(move |_| {
            let mut event = Event::new("menuclick", menu_id, sr.clone());
            sr.inner.event_registration.emit_event("menuclick", &mut event);
        }))).unwrap();
    }

    fn activate_ts(&mut self) {
        let mut sr = self.clone();
        self.inner.event_loop_proxy.send_event(AppEvent::CallbackWithEventLoop(Box::new(move |_| {
            let mut event = Event::new("activate", (), sr.clone());
            sr.inner.event_registration.emit_event("activate", &mut event);
        }))).unwrap();
    }
}

// Type conversion

impl ToJsValue2 for SystemTrayRef {
    fn to_js_value(self) -> Result<JsValue, Error> {
        Ok(JsValue::Resource(ResourceValue {
            resource: Rc::new(RefCell::new(self)),
        }))
    }
}

impl FromJsValue2 for SystemTrayRef {
    fn from_js_value(value: JsValue) -> Result<Self, Error> {
        if let Some(r) = value.as_resource(|r:&mut SystemTrayRef| r.clone()) {
            Ok(r)
        } else {
            Err(anyhow!("Invalid value"))
        }
    }
}


// Js Api


pub fn tray_create(id: String) -> Result<SystemTrayRef, Error> {
    let tray = SystemTrayRef::new(&id, get_event_proxy());
    Ok(tray)
}

pub fn tray_bind_event(mut n: SystemTrayRef, event_name: String, callback: JsValue) -> Result<i32, Error> {
    let handler = create_event_handler(&event_name, callback);
    js_event_bind!(n, "activate", (), &event_name, handler);
    js_event_bind!(n, "menuclick", String, &event_name, handler);
    Ok(0)
}

pub fn tray_set_menus(mut n: SystemTrayRef, menus: Vec<TrayMenu>) -> Result<i32, Error> {
    n.set_menus(menus);
    Ok(0)
}

pub fn tray_set_icon(mut n: SystemTrayRef,  icon: String) -> Result<(), Error> {
    n.set_icon(icon);
    Ok(())
}

pub fn tray_set_title(mut n: SystemTrayRef,  title: String) -> Result<(), Error> {
    n.set_title(title);
    Ok(())
}

pub fn tray_remove_event_listener(mut n: SystemTrayRef,  event_name: String, id: i32) -> Result<(), Error> {
    n.remove_event_listener(event_name, id);
    Ok(())
}