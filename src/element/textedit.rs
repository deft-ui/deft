use crate as deft;
use crate::element::common::editable::Editable;
use crate::js_module;
use crate::element::{Element, Widget, ElementWeak};
use crate::style::length::{Length, LengthOrPercent};
use crate::style::{FixedStyleProp, StylePropVal};
use deft_macros::{widget, js_methods};
use crate::element::textinput::{TextInputState, TextInputStateData};
use crate::event::FocusEventListener;

#[widget]
pub struct TextEdit {
    element: ElementWeak,
    state: TextInputState,
}

#[js_methods]
impl TextEdit {
    
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
    pub fn set_max_history(&mut self, max_history: usize) {
        self.state.editable.set_max_history(max_history);
    }

    #[js_func]
    pub fn get_max_history(&self) -> usize {
        self.state.editable.get_max_history()
    }

    #[js_func]
    pub fn get_placeholder(&self) -> String {
        self.state.editable.get_placeholder()
    }

    #[js_func]
    pub fn set_selection_by_char_offset(&mut self, start: usize, end: usize) {
        self.state.editable.set_selection_by_char_offset(start, end);
    }

    #[js_func]
    pub fn set_caret_by_char_offset(&mut self, char_offset: usize) {
        self.state.editable.set_caret_by_char_offset(char_offset);
    }

    #[js_func]
    pub fn create() -> Self {
        let mut element = Element::new("text-edit");
        element.allow_ime = true;
        element.set_focusable(true);
        element.is_form_element = true;
        let mut editable = Editable::new();
        editable.set_style_props(vec![
            FixedStyleProp::MinHeight(StylePropVal::Custom(LengthOrPercent::Length(Length::EM(
                2.0,
            )))),
            // FixedStyleProp::MinHeight(StylePropVal::Custom(LengthOrPercent::Percent(100.0))),
            // FixedStyleProp::BackgroundColor(StylePropVal::Custom(Color::from_argb(80, 80, 80, 80))),
        ]);
        element.add_child(&editable, Some(0)).unwrap();
        //TODO fix focusable
        // editable.element().clone().set_focusable(false);
        editable.set_multiple_line(true);

        let state = TextInputStateData {
            editable
        }.to_ref();

        {
            let mut state = state.clone();
            element.register_event_listener(FocusEventListener::new(move |_d, _ctx| {
                state.editable.focus();
            }));
        }

        element.set_delegate(state.clone());
        Self {
            state,
            element: element.as_weak(),
            el: element,
        }
    }
}

impl Widget for TextEdit {

    // fn on_event(&mut self, event: &mut Event, ctx: &mut EventContext<ElementWeak>) {
    //     if ctx.target == self.element {
    //         let eb = self.state.editable.element().get_bounds();
    //         self.state.editable.handle_event(event, ctx, (-eb.x, -eb.y));
    //     }
    // }

    // fn execute_default_behavior(
    //     &mut self,
    //     event: &mut Event,
    //     ctx: &mut EventContext<ElementWeak>,
    // ) -> bool {
    //     if ctx.target == self.element {
    //         return self.state.editable.on_execute_default_behavior(event);
    //     }
    //     false
    // }
}

js_module!(TextEdit);
