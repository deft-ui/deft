use crate as deft;
use crate::base::EventContext;
use crate::element::container::Container;
use crate::element::image::Image;
use crate::element::text::Text;
use crate::element::util::is_form_event;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::ClickEvent;
use crate::js::FromJsValue;
use crate::ok_or_return;
use crate::style::{LengthOrPercent, StyleProp, StylePropVal};
use crate::style_list::ParsedStyleProp;
use deft_macros::{element_backend, event, js_methods};
use quick_js::JsValue;
use std::any::Any;
use std::collections::HashMap;
use yoga::{Align, Display, FlexDirection};

#[event]
pub struct ChangeEvent {}

#[element_backend]
pub struct Checkbox {
    element: ElementWeak,
    base: Container,
    img_element: Element,
    wrapper_element: Element,
    box_element: Element,
    label_element: Element,
    checked: bool,
    disabled: bool,
}

#[js_methods]
impl Checkbox {
    #[js_func]
    pub fn set_label(&mut self, label: String) {
        self.label_element
            .get_backend_mut_as::<Text>()
            .set_text(label);
    }

    #[js_func]
    pub fn get_label(&mut self) -> String {
        self.label_element.get_backend_mut_as::<Text>().get_text()
    }

    #[js_func]
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    #[js_func]
    pub fn set_checked(&mut self, checked: bool) {
        let mut el = ok_or_return!(self.element.upgrade());
        if checked {
            el.set_attribute("checked".to_string(), "".to_string());
        } else {
            el.remove_attribute("checked".to_string());
        }
    }

    #[js_func]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    #[js_func]
    pub fn set_disabled(&mut self, disabled: bool) {
        let mut ele = ok_or_return!(self.element.upgrade());
        if disabled {
            ele.set_attribute("disabled".to_string(), "".to_string());
        } else {
            ele.remove_attribute("disabled".to_string());
        }
    }

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
        self.img_element
            .set_style_props(vec![StyleProp::Display(StylePropVal::Custom(display))]);
    }
}

impl ElementBackend for Checkbox {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        let base = Container::create(element);
        let mut wrapper_element = Element::create(Container::create);
        let mut box_element = Element::create(Container::create);
        let label_element = Element::create(Text::create);
        let mut img_element = Element::create(Image::create);
        img_element
            .get_backend_mut_as::<Image>()
            .set_src_svg_raw(include_bytes!("./checked.svg"));
        img_element.set_style_props(vec![
            StyleProp::Width(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            StyleProp::Height(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
        ]);
        box_element.add_child(img_element.clone(), 0).unwrap();

        wrapper_element.add_child(box_element.clone(), 0).unwrap();
        wrapper_element.add_child(label_element.clone(), 1).unwrap();

        element.add_child(wrapper_element.clone(), 0).unwrap();
        wrapper_element.set_style_props(vec![
            StyleProp::AlignItems(StylePropVal::Custom(Align::Center)),
            StyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
        ]);
        let mut inst = CheckboxData {
            element: element.as_weak(),
            base,
            img_element,
            wrapper_element,
            box_element,
            label_element,
            checked: false,
            disabled: false,
        }
        .to_ref();
        inst.update_children();
        inst
    }

    fn get_name(&self) -> &str {
        "Checkbox"
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        Some(&mut self.base)
    }

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        if self.disabled && is_form_event(&event) {
            ctx.propagation_cancelled = true;
            return;
        }
        if event.downcast_ref::<ClickEvent>().is_some() {
            self.update_checked(!self.checked);
        } else {
            self.base.on_event(event, ctx);
        }
    }

    fn accept_pseudo_styles(&mut self, styles: HashMap<String, Vec<ParsedStyleProp>>) {
        if let Some(styles) = styles.get("box") {
            let mut list = Vec::new();
            for s in styles {
                if let ParsedStyleProp::Fixed(p) = s {
                    list.push(p.clone());
                }
            }
            //TODO support set parsedStyleProp?
            self.box_element.set_style_props(list);
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        match key {
            "checked" => self.update_checked(value.is_some()),
            "disabled" => self.disabled = value.is_some(),
            _ => self.base.on_attribute_changed(key, value),
        }
    }

    fn bind_js_listener(&mut self, event_type: &str, listener: JsValue) -> Option<u32> {
        let mut element = self.element.upgrade().ok()?;
        let id = match event_type {
            "change" => {
                element.register_event_listener(ChangeEventListener::from_js_value(listener).ok()?)
            }
            _ => return None,
        };
        Some(id)
    }
}
