use crate as deft;
use crate::base::EventContext;
use crate::element::container::Container;
use crate::element::image::Image;
use crate::element::label::Label;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::{ClickEvent, Event};
use crate::ok_or_return;
use crate::style::length::LengthOrPercent;
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropVal};
use deft_macros::{element_backend, event, js_methods};
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
}

#[js_methods]
impl Checkbox {
    #[js_func]
    pub fn set_label(&mut self, label: String) {
        self.label_element
            .get_backend_mut_as::<Label>()
            .set_text(label);
    }

    #[js_func]
    pub fn get_label(&mut self) -> String {
        self.label_element.get_backend_mut_as::<Label>().get_text()
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
            .set_style_props(vec![FixedStyleProp::Display(StylePropVal::Custom(display))]);
    }
}

impl ElementBackend for Checkbox {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        element.is_form_element = true;
        element.register_js_event::<ChangeEvent>("change");
        let base = Container::create(element);
        let mut wrapper_element = Element::create(Container::create);
        let mut box_element = Element::create(Container::create);
        let label_element = Element::create(Label::create);
        let mut img_element = Element::create(Image::create);
        img_element
            .get_backend_mut_as::<Image>()
            .set_src_svg_raw(include_bytes!("./checked.svg"));
        img_element.set_style_props(vec![
            FixedStyleProp::Width(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            FixedStyleProp::Height(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
        ]);
        box_element.add_child(img_element.clone(), 0).unwrap();

        wrapper_element.add_child(box_element.clone(), 0).unwrap();
        wrapper_element.add_child(label_element.clone(), 1).unwrap();

        element.add_child(wrapper_element.clone(), 0).unwrap();
        wrapper_element.set_style_props(vec![
            FixedStyleProp::AlignItems(StylePropVal::Custom(Align::Center)),
            FixedStyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
        ]);
        let mut inst = CheckboxData {
            element: element.as_weak(),
            base,
            img_element,
            wrapper_element,
            box_element,
            label_element,
            checked: false,
        }
        .to_ref();
        inst.update_children();
        inst
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        Some(&mut self.base)
    }

    fn on_event(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
        if event.downcast_ref::<ClickEvent>().is_some() {
            self.update_checked(!self.checked);
        } else {
            self.base.on_event(event, ctx);
        }
    }

    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(styles) = styles.get("box") {
            let styles = styles.iter().map(|s| s.to_unresolved()).collect::<Vec<_>>();
            self.box_element.set_style_props(styles);
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        match key {
            "checked" => self.update_checked(value.is_some()),
            _ => self.base.on_attribute_changed(key, value),
        }
    }
}
