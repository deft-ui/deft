use crate as deft;
use crate::element::container::Container;
use crate::element::image::Image;
use crate::element::label::Label;
use crate::element::{Element, Widget, ElementDelegate, ElementWeak};
use crate::event::{ClickEventListener};
use crate::ok_or_return;
use crate::style::length::LengthOrPercent;
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropVal};
use deft_macros::{widget, event, js_methods, mrc_object};
use std::collections::HashMap;
use yoga::{Align, Display, FlexDirection};
use crate::js_module;
use crate::mrc::Mrc;

#[event]
pub struct ChangeEvent {}

struct CheckboxState {
    label: Label,
    delegate: CheckboxDelegate,
}

#[widget]
pub struct Checkbox {
    state: Mrc<CheckboxState>,
}

#[js_methods]
impl Checkbox {
    #[js_func]
    pub fn set_label(&mut self, label: String) {
        self.state.label.set_text(label);
    }

    #[js_func]
    pub fn get_label(&mut self) -> String {
        self.state.label.get_text()
    }

    #[js_func]
    pub fn is_checked(&self) -> bool {
        self.state.delegate.checked
    }

    #[js_func]
    pub fn set_checked(&mut self, checked: bool) {
        let mut el = ok_or_return!(self.state.delegate.element.upgrade());
        if checked {
            el.set_attribute("checked".to_string(), "".to_string());
        } else {
            el.remove_attribute("checked".to_string());
        }
    }

    #[js_func]
    pub fn create() -> Self {
        let mut element = Element::new("checkbox");
        element.is_form_element = true;
        element.register_js_event::<ChangeEvent>("change");
        let mut wrapper = Container::create();
        let mut box_container = Container::create();
        let label = Label::create();
        let mut img = Image::create();
        img.set_src_svg_raw(include_bytes!("./checked.svg"));
        img.set_style_props(vec![
            FixedStyleProp::Width(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            FixedStyleProp::Height(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
        ]);
        box_container.add_child(&img, Some(0)).unwrap();

        wrapper.add_child(&box_container, Some(0)).unwrap();
        wrapper.add_child(&label, Some(1)).unwrap();

        element.add_child(&wrapper, Some(0)).unwrap();
        wrapper.set_style_props(vec![
            FixedStyleProp::AlignItems(StylePropVal::Custom(Align::Center)),
            FixedStyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
        ]);
        let delegate = CheckboxDelegateData {
            box_container,
            img,
            checked: false,
            element: element.as_weak(),
        }.to_ref();
        element.set_delegate(delegate.clone());
        {
            let mut delegate = delegate.clone();
            element.register_event_listener(ClickEventListener::new(move |_e, _ctx| {
                let checked = !delegate.checked;
                delegate.update_checked(checked);
            }));
        }
        let mut inst = Checkbox {
            el: element,
            state: Mrc::new(CheckboxState {
                label,
                delegate,
            })
        };
        inst.state.delegate.update_children();
        inst
    }
}

impl Widget for Checkbox {}

impl ElementDelegate for CheckboxDelegate {
    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(styles) = styles.get("box") {
            let styles = styles.iter().map(|s| s.to_unresolved()).collect::<Vec<_>>();
            self.box_container.set_style_props(styles);
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        match key {
            "checked" => self.update_checked(value.is_some()),
            _ => {},
        }
    }
}

#[mrc_object]
struct CheckboxDelegate {
    element: ElementWeak,
    box_container: Container,
    checked: bool,
    img: Image,
}

impl CheckboxDelegate {
    fn update_checked(&mut self, checked: bool) {
        if self.checked != checked {
            self.checked = checked;
            self.update_children();
            self.element.emit(ChangeEvent {});
        }
    }

    fn update_children(&mut self) {
        let display = if self.checked {
            Display::Flex
        } else {
            Display::None
        };
        self.img.set_style_props(vec![FixedStyleProp::Display(StylePropVal::Custom(display))]);
    }
}

js_module!(Checkbox);
