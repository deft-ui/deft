use crate as deft;
use crate::element::container::Container;
use crate::js_module;
use crate::element::image::Image;
use crate::element::label::Label;
use crate::element::{DescendantsChangeType, Element, Widget, ElementDelegate, ElementWeak};
use crate::event::{ClickEventListener};
use crate::style::length::LengthOrPercent;
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropVal};
use crate::some_or_return;
use deft_macros::{widget, event, js_methods, mrc_object};
use std::collections::HashMap;
use anyhow::anyhow;
use yoga::{Align, Display, FlexDirection};

#[widget]
pub struct Radio {
    base: Container,
    label: Label,
}

#[event]
pub struct ChangeEvent {}

#[mrc_object]
struct RadioGroupDelegate {
    radio_list: Vec<RadioDelegate>,
}

impl RadioGroupDelegate {
    pub fn new() -> Self {
        RadioGroupDelegateData {
            radio_list: vec![],
        }.to_ref()
    }
}

#[mrc_object]
struct RadioDelegate {
    element: ElementWeak,
    checked: bool,
    group: Option<RadioGroupDelegate>,
    img: Image,
    box_element: Container,
}

impl RadioDelegate {

    fn new(box_element: Container, element: ElementWeak, img: Image) -> Self {
        RadioDelegateData {
            box_element,
            element,
            checked: false,
            group: None,
            img,
        }.to_ref()
    }
    pub fn set_checked(&mut self, checked: bool) {
        match (checked, &mut self.group.clone()) {
            (true, Some(group)) => {
                for o in group.radio_list.iter_mut() {
                    let new_checked = self == o;
                    o.update_self_checked(new_checked);
                }
            },
            _ => self.update_self_checked(checked),
        }
    }

    fn update_self_checked(&mut self, new_checked: bool) {
        if self.checked != new_checked {
            self.checked = new_checked;
            self.update_children();
            self.element.mark_dirty(false);
        }
    }

    fn update_children(&mut self) {
        let display = if self.checked {
            Display::Flex
        } else {
            Display::None
        };
        self.img
            .set_style_props(vec![FixedStyleProp::Display(StylePropVal::Custom(display))]);
    }

    fn update_checked(&mut self, checked: bool) -> anyhow::Result<()> {
        if self.checked != checked {
            self.set_checked(checked);
            self.element.emit(ChangeEvent {});
        }
        Ok(())
    }
    
}
impl RadioGroupDelegate {

    fn search_radio_recursively(&self, element: &Element, ty: DescendantsChangeType) {
        if let Some(mut radio_state) = element.resource_table.get::<RadioDelegate>().cloned() {
            let mut group_state = self.clone();
            match ty {
                DescendantsChangeType::Attached => {
                    let has_checked = group_state.radio_list.iter().find(|r| r.checked).is_some();
                    if has_checked {
                        radio_state.update_self_checked(false);
                    }
                    radio_state.group = Some(group_state.clone());
                    group_state.radio_list.push(radio_state);
                }
                DescendantsChangeType::Removed => {
                    radio_state.group = None;
                    group_state.radio_list.retain(|r| r != &radio_state);
                }
            }
        } else {
            for c in element.get_children() {
                self.search_radio_recursively(&c, ty);
            }
        }
    }
}

#[js_methods]
impl Radio {

    #[js_func]
    pub fn set_label(&mut self, label: String) {
        self.label.set_text(label);
    }

    #[js_func]
    pub fn get_label(&mut self) -> String {
        self.label.get_text()
    }

    #[js_func]
    pub fn is_checked(&self) -> anyhow::Result<bool> {
        let state = self.get_state()?;
        Ok(state.checked)
    }

    #[js_func]
    pub fn set_checked(&mut self, checked: bool) {
        if checked {
            self.el.set_attribute("checked".to_string(), "".to_string());
        } else {
            self.el.remove_attribute("checked".to_string());
        }
    }

    fn get_state(&self) -> anyhow::Result<RadioDelegate> {
        let state = some_or_return!(self.el.resource_table.get::<RadioDelegate>(), Err(anyhow!("state not found")));
        Ok(state.clone())
    }

    #[js_func]
    pub fn create() -> Self {
        let mut element = Element::new("radio");
        element.is_form_element = true;
        let base = Container::create();
        let mut wrapper_element = Container::create();
        let mut box_element = Container::create();
        let label = Label::create();
        let mut img = Image::create();
        img.set_src_svg_raw(include_bytes!("./selected.svg"));
        img.set_style_props(vec![
            FixedStyleProp::Width(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            FixedStyleProp::Height(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
        ]);
        box_element.add_child(&img, Some(0)).unwrap();

        wrapper_element.add_child(&box_element, Some(0)).unwrap();
        wrapper_element.add_child(&label, Some(1)).unwrap();

        element.add_child(&wrapper_element, Some(0)).unwrap();
        wrapper_element.set_style_props(vec![
            FixedStyleProp::AlignItems(StylePropVal::Custom(Align::Center)),
            FixedStyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
        ]);
        let mut radio_state = RadioDelegate::new(box_element, element.as_weak(), img);
        radio_state.update_children();
        element.set_delegate(radio_state.clone());

        element.resource_table.put(radio_state.clone());
        let mut inst = Radio {
            el: element,
            base,
            label,
        };
        
        inst.el.register_event_listener(ClickEventListener::new(move |_e, _ctx| {
            let _ = radio_state.update_checked(true);
        }));
        inst
    }

}

impl Widget for Radio {
}

impl ElementDelegate for RadioDelegate {
    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(styles) = styles.get("box") {
            let styles = styles.iter().map(|s| s.to_unresolved()).collect::<Vec<_>>();
            self.box_element.set_style_props(styles);
        }
    }

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        match key {
            "checked" => {
                let _ = self.update_checked(value.is_some());
            },
            _ => {},
        }
    }
}

#[widget]
pub struct RadioGroup {
    base: Container,
}

#[js_methods]
impl RadioGroup {

    #[js_func]
    pub fn create() -> Self {
        let mut element = Element::new("radio-group");
        element.register_js_event::<ChangeEvent>("change");
        element.set_delegate(RadioGroupDelegate::new());
        let base = Container::create();
        RadioGroup {
            el: element,
            base
        }
    }
}

impl Widget for RadioGroup {

}

js_module!(RadioGroup);
js_module!(Radio);

impl ElementDelegate for RadioGroupDelegate {
    fn on_descendant_changed(&self, descendant_root: &Element, ty: DescendantsChangeType) {
        self.search_radio_recursively(descendant_root, ty);
    }
}