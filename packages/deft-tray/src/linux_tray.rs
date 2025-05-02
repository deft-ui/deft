use std::sync::{Arc, Mutex};
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::MenuItem::{Checkmark, Separator};
use ksni::{Handle, Tray};
use crate::{MenuKind, TrayMenu};

pub struct MyTray {
    pub(crate) tray_id: String,
    pub(crate) activate_callback: Box<dyn FnMut() + Send>,
    pub(crate) title: String,
    pub(crate) icon: String,
    pub(crate) menus: Vec<TrayMenu>,
    pub(crate) menu_click_callback: Arc<Mutex<Box<dyn FnMut(String) + Send>>>,
}


impl Tray for MyTray {
    fn activate(&mut self, _x: i32, _y: i32) {
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
            let menu_callback = self.menu_click_callback.clone();
            let menu_id = m.id.clone().unwrap_or("".to_string());
            let activate: Box<dyn Fn(&mut MyTray)> = Box::new(move |_| {
                let mut menu_callback = menu_callback.lock().unwrap();
                menu_callback(menu_id.clone());
            });
            let enabled = m.enabled.unwrap_or(true);
            let checked = m.checked.unwrap_or(false);
            let label = m.label.clone().unwrap_or("".to_string());
            let kind = match MenuKind::from_str(&m.kind) {
                Some(kind) => kind,
                None => continue,
            };
            match kind {
                MenuKind::Standard => {
                    list.push(StandardItem {
                        label,
                        activate,
                        enabled,
                        ..Default::default()
                    }.into());
                }
                MenuKind::Checkmark => {
                    list.push(Checkmark(CheckmarkItem {
                        label,
                        activate,
                        enabled,
                        checked,
                        ..Default::default()
                    }))
                }
                MenuKind::Separator => {
                    list.push(Separator);
                }
            }
        }
        list
    }
}

pub struct LinuxTray {
    pub(crate) handle: Handle<MyTray>,
}

impl LinuxTray {
    pub fn new(tray_id: &str) -> Self {
        let service = ksni::TrayService::new(MyTray {
            tray_id: tray_id.to_string(),
            activate_callback: Box::new(|| {}),
            title: "".to_string(),
            icon: "".to_string(),
            menus: Vec::new(),
            menu_click_callback: Arc::new(Mutex::new(Box::new(|_| {}))),
        });
        let handle = service.handle();
        service.spawn();
        Self {
            handle
        }
    }

    pub fn set_active_callback(&mut self, cb: Box<dyn FnMut() + Send>) {
        self.handle.update(|handle| {
            handle.activate_callback = cb;
        });
    }

    pub fn set_menu_click_callback(&mut self, cb:  Box<dyn FnMut(String) + Send>) {
        self.handle.update(move|handle| {
            handle.menu_click_callback = Arc::new(Mutex::new(cb));
        });
    }

    pub fn set_title(&mut self, title: &str) {
        let title = title.to_string();
        self.handle.update(move |t| {
            t.title = title;
        });
    }

    pub fn set_icon(&mut self, icon: &str) {
        self.handle.update(move |t| {
            t.icon = icon.to_string();
        });
    }

    pub fn set_menus(&mut self, menus: Vec<TrayMenu>) {
        self.handle.update(move |t| {
            t.menus = menus;
        });
    }

}
