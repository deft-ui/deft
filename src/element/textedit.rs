use crate as deft;
use crate::base::EventContext;
use crate::element::common::editable::Editable;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::style::length::{Length, LengthOrPercent};
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropVal};
use deft_macros::{element_backend, js_methods};
use std::any::Any;
use std::collections::HashMap;

#[element_backend]
pub struct TextEdit {
    element: ElementWeak,
    editable_element: Element,
    editable: Editable,
}

#[js_methods]
impl TextEdit {
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
    pub fn set_selection_by_char_offset(&mut self, start: usize, end: usize) {
        self.editable.set_selection_by_char_offset(start, end);
    }

    #[js_func]
    pub fn set_caret_by_char_offset(&mut self, char_offset: usize) {
        self.editable.set_caret_by_char_offset(char_offset);
    }
}

impl ElementBackend for TextEdit {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        element.allow_ime = true;
        element.set_focusable(true);
        element.is_form_element = true;
        let mut editable = Element::create(Editable::create);
        editable.set_style_props(vec![
            FixedStyleProp::MinHeight(StylePropVal::Custom(LengthOrPercent::Length(Length::EM(
                2.0,
            )))),
            // FixedStyleProp::MinHeight(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            // FixedStyleProp::BackgroundColor(StylePropVal::Custom(Color::from_argb(80, 80, 80, 80))),
        ]);
        element.add_child(editable.clone(), 0).unwrap();
        editable.set_focusable(false);
        let mut backend = editable.get_backend_as::<Editable>().clone();
        backend.set_multiple_line(true);
        TextEditData {
            editable_element: editable.clone(),
            editable: backend,
            element: element.as_weak(),
        }
        .to_ref()
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn on_event(&mut self, event: &mut Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        // let el = ok_or_return!(self.element.upgrade());
        if ctx.target == self.element {
            let eb = self.editable_element.get_bounds();
            self.editable.handle_event(event, ctx, (-eb.x, -eb.y));
        }
    }

    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        self.editable.accept_pseudo_element_styles(styles);
    }
}
