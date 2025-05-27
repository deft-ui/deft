use std::any::Any;
use crate as deft;
use deft_macros::{element_backend, js_methods};
use quick_js::JsValue;
use yoga::FlexDirection;
use crate::base::EventContext;
use crate::element::common::editable::{Editable, InputType};
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::style::{FixedStyleProp, StylePropVal};
use crate::style::length::LengthOrPercent;

#[element_backend]
pub struct TextInput {
    element: ElementWeak,
    editable_element: Element,
    editable: Editable,
}

#[js_methods]
impl TextInput {
    #[js_func]
    pub fn get_text(&self) -> String {
        self.editable.get_text()
    }

    #[js_func]
    pub fn set_text(&mut self, text: String) {
        self.editable.set_text(text);
    }

    #[js_func]
    pub fn set_placeholder(&mut self, placeholder: String) {
        self.editable.set_placeholder(placeholder);
    }

    #[js_func]
    pub fn get_placeholder(&self) -> String {
        self.editable.get_placeholder()
    }

    #[js_func]
    pub fn set_type(&mut self, input_type: InputType) {
        self.editable.set_type(input_type);
    }

    #[js_func]
    pub fn get_type(&self) -> InputType {
        self.editable.get_type()
    }

}

impl ElementBackend for TextInput {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized
    {
        element.scrollable.vertical_bar.set_thickness(0.0);
        element.scrollable.horizontal_bar.set_thickness(0.0);
        element.set_style_props(vec![
            FixedStyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
        ]);
        let mut editable = Element::create(Editable::create);
        editable.set_style_props(vec![
            FixedStyleProp::MinWidth(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            // FixedStyleProp::FlexDirection(StylePropVal::Custom(FlexDirection::Row)),
            // FixedStyleProp::MinHeight(StylePropVal::Custom(LengthOrPercent::Length(Length::EM(2.0)))),
            // FixedStyleProp::Height(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            // FixedStyleProp::BackgroundColor(StylePropVal::Custom(Color::from_rgb(0, 0, 0))),
        ]);
        element.add_child(editable.clone(), 0).unwrap();
        editable.set_focusable(false);
        element.is_form_element = true;
        element.set_focusable(true);
        let backend = editable.get_backend_as::<Editable>().clone();
        TextInputData {
            element: element.as_weak(),
            editable_element: editable.clone(),
            editable: backend,
        }.to_ref()
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        // let el = ok_or_return!(self.element.upgrade());
        if ctx.target == self.element {
            let eb = self.editable_element.get_bounds();
            self.editable.handle_event(event, ctx, (-eb.x, -eb.y));
        }
    }

    fn bind_js_listener(&mut self, event_type: &str, listener: JsValue) -> Option<u32> {
        self.editable.bind_js_listener(event_type, listener)
    }
}

