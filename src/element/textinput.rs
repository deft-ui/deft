use crate as deft;
use crate::element::common::editable::{Editable, InputType};
use crate::js_module;
use crate::element::{Element, Widget, ElementDelegate, ElementWeak};
use crate::ok_or_return;
use crate::style::length::LengthOrPercent;
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropVal};
use deft_macros::{widget, js_methods, mrc_object};
use std::collections::HashMap;
use yoga::FlexDirection;

#[widget]
pub struct TextInput {
    element: ElementWeak,
    state: TextInputState,
}

#[js_methods]
impl TextInput {

    #[js_func]
    pub fn get_text(&self) -> String {
        self.state.editable.get_text()
    }

    #[js_func]
    pub fn set_text(&mut self, text: String) {
        self.state.editable.set_text(text);
    }

    #[js_func]
    pub fn set_placeholder(&mut self, placeholder: String) {
        self.state.editable.set_placeholder(placeholder);
    }

    #[js_func]
    pub fn get_placeholder(&self) -> String {
        self.state.editable.get_placeholder()
    }

    #[js_func]
    pub fn set_type(&mut self, input_type: InputType) {
        let mut el = ok_or_return!(self.element.upgrade());
        self.state.editable.set_type(input_type);
        el.allow_ime = match input_type {
            InputType::Text => true,
            InputType::Password => false,
        };
    }

    #[js_func]
    pub fn get_type(&self) -> InputType {
        self.state.editable.get_type()
    }

    #[js_func]
    pub fn create() -> Self {
        let mut element = Element::new("text-input");
        element.allow_ime = true;
        element.scrollable.vertical_bar.set_thickness(0.0);
        element.scrollable.horizontal_bar.set_thickness(0.0);
        element.set_style_props(vec![FixedStyleProp::FlexDirection(StylePropVal::Custom(
            FlexDirection::Row,
        ))]);
        let mut editable = Editable::new();
        editable.set_style_props(vec![
            FixedStyleProp::MinWidth(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            // FixedStyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
            // FixedStyleProp::MinHeight(StylePropVal::Custom(LengthOrPercent::Length(Length::EM(2.0)))),
            // FixedStyleProp::Height(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            // FixedStyleProp::BackgroundColor(StylePropVal::Custom(Color::from_rgb(0, 0, 0))),
        ]);
        element.add_child(&editable, Some(0)).unwrap();
        //TODO fix focusable
        // editable.element().clone().set_focusable(false);
        element.is_form_element = true;
        element.set_focusable(true);

        let state = TextInputStateData {
            editable,
        }.to_ref();

        element.set_delegate(state.clone());
        Self {
            element: element.as_weak(),
            state,
            el: element,
        }
    }
}

impl Widget for TextInput {

}

impl ElementDelegate for TextInputState {
    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        self.editable.accept_pseudo_element_styles(styles);
    }
}

#[mrc_object]
pub struct TextInputState {
    pub editable: Editable,
}

js_module!(TextInput);

