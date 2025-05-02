use crate::TrayMenu;

pub struct NoTray {

}

impl NoTray {
    pub fn new(_tray_id: &str) -> Self {
        Self {}
    }

    pub fn set_active_callback(&mut self, _cb: Box<dyn FnMut() + Send>) {
    }

    pub fn set_menu_click_callback(&mut self, _cb: Box<dyn FnMut(String) + Send>) {
    }

    pub fn set_title(&mut self, _title: &str) {

    }

    pub fn set_icon(&mut self, _icon: &str) {

    }

    pub fn set_menus(&mut self, _menus: Vec<TrayMenu>) {

    }
}