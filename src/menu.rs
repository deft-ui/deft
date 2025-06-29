use crate as deft;
use crate::element::container::Container;
use crate::element::label::Label;
use crate::element::{Element, ElementBackend, ElementType};
use crate::event::ClickEventListener;
use crate::js_value;
use crate::mrc::Mrc;
use deft_macros::{js_methods, mrc_object};
use log::error;
use quick_js::JsValue;

#[mrc_object]
pub struct Menu {
    items: Vec<MenuItem>,
}

js_value!(Menu);

#[js_methods]
impl Menu {
    #[js_func]
    pub fn new() -> Self {
        MenuData { items: Vec::new() }.to_ref()
    }
    pub fn add_item(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    #[js_func]
    pub fn add_standard_item(&mut self, standard_item: StandardMenuItem) {
        self.items.push(MenuItem::Standard(standard_item));
    }

    #[js_func]
    pub fn add_separator(&mut self) {
        self.items.push(MenuItem::Separator);
    }
}

#[mrc_object]
pub struct StandardMenuItem {
    pub disabled: bool,
    pub label: String,
    pub onclick: Mrc<Option<Box<dyn FnMut()>>>,
}

js_value!(StandardMenuItem);

#[js_methods]
impl StandardMenuItem {
    pub fn new<F: FnMut() + 'static>(label: &str, callback: F) -> Self {
        StandardMenuItemData {
            disabled: false,
            label: label.to_string(),
            onclick: Mrc::new(Some(Box::new(callback))),
        }
        .to_ref()
    }

    #[js_func]
    pub fn js_new(label: String, callback: JsValue) -> Self {
        StandardMenuItemData {
            disabled: false,
            label,
            onclick: Mrc::new(Some(Box::new(move || {
                if let Err(e) = callback.clone().call_as_function(vec![]) {
                    error!("Error calling callback: {}", e);
                }
            }))),
        }
        .to_ref()
    }

    #[js_func]
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    #[js_func]
    pub fn get_disabled(&self) -> bool {
        self.disabled
    }
}

#[derive(Clone)]
pub enum MenuItem {
    Separator,
    Standard(StandardMenuItem),
}

pub fn build_menu_elements(menu: Menu) -> Element {
    let mut root = Element::create(Container::create);
    root.tag = "menu".to_string();
    root.set_element_type(ElementType::Widget);
    for it in menu.items.clone() {
        match it {
            MenuItem::Separator => {
                let mut e = Element::create(Container::create);
                e.tag = "menu-item-separator".to_string();
                e.set_element_type(ElementType::Widget);
                root.add_child(e, -1).unwrap();
            }
            MenuItem::Standard(s) => {
                let mut e = Element::create(Label::create);
                e.is_form_element = true;
                e.tag = "menu-item-standard".to_string();
                e.set_element_type(ElementType::Widget);
                e.set_disabled(s.disabled);
                let label = e.get_backend_mut_as::<Label>();
                label.set_text(s.label.to_string());
                let mut onclick = s.onclick.clone();
                e.register_event_listener(ClickEventListener::new(move |_, _| {
                    if let Some(onclick) = &mut *onclick {
                        onclick();
                    }
                }));
                root.add_child(e, -1).unwrap();
            }
        }
    }
    root
}
