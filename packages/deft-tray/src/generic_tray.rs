use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconAttributes, TrayIconEvent};
use crate::{MenuKind, TrayMenu};

static EVENT_MANAGER: LazyLock<EventManager> = LazyLock::new(|| {
    TrayIconEvent::set_event_handler(Some(move |event| {
        match event {
            TrayIconEvent::Click {
                id,
                button,
                button_state,
                ..
            } => {
                if button == MouseButton::Left && button_state == MouseButtonState::Down {
                    // println!("Clicked with id {}", &id);
                    let id = id.0;
                    EVENT_MANAGER.emit_click(&id);
                }
            }
            _ => {}
        }
    }));

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        // println!("tray menu event {:?}", event);
        EVENT_MANAGER.emit_menu_click(&event.id.0);
    }));
    EventManager::default()
});

pub struct GenericTray {
    tray_icon: TrayIcon,
    menu_click_callback: Arc<Mutex<Box<dyn FnMut(String) + Send>>>,
    menu_ids: Vec<String>,
}

#[derive(Default)]
struct EventManager {
    /// tray_id => callback
    click_callbacks: Arc<Mutex<HashMap<String, Box<dyn FnMut() + Send>>>>,
    /// menu_id => callback
    menu_click_callbacks: Arc<Mutex<HashMap<String, Box<dyn FnMut() + Send>>>>,
}

impl EventManager {
    pub fn set_click_callback(&self, id: &str, callback: Box<dyn FnMut() + Send>) {
        let mut click_callbacks = self.click_callbacks.lock().unwrap();
        click_callbacks.insert(id.to_string(), Box::new(callback));
    }

    pub fn remove_click_callback(&self, id: &str) {
        let mut click_callbacks = self.click_callbacks.lock().unwrap();
        click_callbacks.remove(id);
    }

    pub fn set_menu_click_callback(&self, menu_id: &str, callback: Box<dyn FnMut() + Send>) {
        let mut menu_callbacks = self.menu_click_callbacks.lock().unwrap();
        menu_callbacks.insert(menu_id.to_string(), callback);
    }

    pub fn remove_menu_click_callback(&self, menu_id: &str) {
        let mut menu_callbacks = self.menu_click_callbacks.lock().unwrap();
        menu_callbacks.remove(menu_id);
    }

    pub fn emit_click(&self, id: &str) {
        let mut click_callbacks = self.click_callbacks.lock().unwrap();
        if let Some(callback) = click_callbacks.get_mut(id) {
            callback()
        }
    }

    pub fn emit_menu_click(&self, menu_id: &str) {
        let mut click_callbacks = self.menu_click_callbacks.lock().unwrap();
        if let Some(callback) = click_callbacks.get_mut(menu_id) {
            callback();
        }
    }
}

impl GenericTray {
    pub fn new(tray_id: &str) -> Self {
        //TODO no unwrap
        let tray_icon = TrayIcon::with_id(tray_id, TrayIconAttributes::default()).unwrap();
        tray_icon.set_show_menu_on_left_click(false);
        let callback = Box::new(|_| {});

        Self {
            tray_icon,
            menu_click_callback: Arc::new(Mutex::new(callback)),
            menu_ids: vec![],
        }
    }

    pub fn set_active_callback(&mut self, cb: Box<dyn FnMut() + Send>) {
        EVENT_MANAGER.set_click_callback(&self.tray_icon.id().0, cb);
    }

    pub fn set_menu_click_callback(&mut self, cb: Box<dyn FnMut(String) + Send>) {
        let mut menu_click_callback = self.menu_click_callback.lock().unwrap();
        *menu_click_callback = cb;
    }

    pub fn set_title(&mut self, title: &str) {
        #[cfg(target_os = "windows")]
        let _ = self.tray_icon.set_tooltip(Some(title));
        #[cfg(not(target_os = "windows"))]
        self.tray_icon.set_title(Some(title));
    }

    pub fn set_icon(&mut self, icon: &str) {
        if let Ok(icon) = Icon::from_path(icon, None) {
            let _ = self.tray_icon.set_icon(Some(icon));
        }
    }

    pub fn set_menus(&mut self, menus: Vec<TrayMenu>) {
        for old_menu_id in &self.menu_ids {
            EVENT_MANAGER.remove_menu_click_callback(old_menu_id);
        }
        self.menu_ids.clear();
        let menu = Menu::new();
        for m in menus {
            let kind = match MenuKind::from_str(&m.kind) {
                None => continue,
                Some(k) => k,
            };
            let label = m.label.unwrap_or("".to_string());
            //TODO auto generate id?
            let menu_id = match m.id {
                Some(id) => id,
                None => continue,
            };
            self.menu_ids.push(menu_id.clone());
            let menu_callback = self.menu_click_callback.clone();
            let activate: Box<dyn FnMut() + Send> = {
                let menu_id = menu_id.clone();
                Box::new(move || {
                    let mut menu_callback = menu_callback.lock().unwrap();
                    menu_callback(menu_id.clone());
                })
            };
            EVENT_MANAGER.set_menu_click_callback(&menu_id, activate);
            match kind {
                MenuKind::Standard => {
                    let std_menu = MenuItem::with_id(menu_id, label, true, None);
                    let _ = menu.append(&std_menu);
                }
                MenuKind::Checkmark => {
                    let checked = m.checked.unwrap_or(false);
                    let check_menu = CheckMenuItem::with_id(menu_id, label, true, checked, None);
                    let _ = menu.append(&check_menu);
                }
                MenuKind::Separator => {
                    let std_menu = MenuItem::new("-".to_string(), true, None);
                    let _ = menu.append(&std_menu);
                }
            }
        }
        self.tray_icon.set_menu(Some(Box::new(menu)));
    }
}

impl Drop for GenericTray {
    fn drop(&mut self) {
        EVENT_MANAGER.remove_click_callback(self.tray_icon.id().0.as_ref());
    }
}