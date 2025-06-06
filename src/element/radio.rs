use crate as deft;
use crate::base::EventContext;
use crate::element::container::Container;
use crate::element::image::Image;
use crate::element::label::Label;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::{ClickEvent, Event};
use crate::style::length::LengthOrPercent;
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropVal};
use crate::{ok_or_return, some_or_return};
use deft_macros::{element_backend, event, js_methods};
use std::collections::HashMap;
use yoga::{Align, Display, FlexDirection};

#[element_backend]
pub struct Radio {
    element: ElementWeak,
    base: Container,
    img_element: Element,
    wrapper_element: Element,
    box_element: Element,
    label_element: Element,
    checked: bool,
}

#[event]
pub struct ChangeEvent {}

fn find_group(mut p: Element) -> Option<Element> {
    loop {
        p = p.get_parent()?;
        if p.is_backend::<RadioGroup>() {
            return Some(p);
        }
    }
}

#[js_methods]
impl Radio {
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
            if checked {
                self.uncheck_siblings();
            }
            self.checked = checked;
            self.update_children();
            self.element.emit(ChangeEvent {});
        }
    }

    fn uncheck_siblings(&mut self) {
        let element = ok_or_return!(self.element.upgrade());
        let mut group = some_or_return!(find_group(element)).clone();
        Self::uncheck_children_recurse(&mut group);
    }

    fn uncheck_children_recurse(element: &mut Element) {
        if element.is_backend::<Radio>() {
            let radio = element.get_backend_mut_as::<Radio>();
            radio.set_checked(false);
        } else {
            for mut c in element.get_children() {
                Self::uncheck_children_recurse(&mut c);
            }
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

impl ElementBackend for Radio {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        element.is_form_element = true;
        let base = Container::create(element);
        let mut wrapper_element = Element::create(Container::create);
        let mut box_element = Element::create(Container::create);
        let label_element = Element::create(Label::create);
        let mut img_element = Element::create(Image::create);
        img_element
            .get_backend_mut_as::<Image>()
            .set_src_svg_raw(include_bytes!("./selected.svg"));
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
        let mut inst = RadioData {
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
        if ClickEvent::is(event) {
            self.update_checked(true);
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

#[element_backend]
pub struct RadioGroup {
    base: Container,
}

impl ElementBackend for RadioGroup {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        element.register_js_event::<ChangeEvent>("change");
        let base = Container::create(element);
        RadioGroupData { base }.to_ref()
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        Some(&mut self.base)
    }
}
