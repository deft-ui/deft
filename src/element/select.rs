use crate as deft;
use crate::base::{EventContext, Rect};
use crate::canvas_util::CanvasHelper;
use crate::element::common::editable::Editable;
use crate::element::common::image_object::ImageObject;
use crate::element::container::Container;
use crate::element::label::Label;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::{ClickEvent, ClickEventListener, Event};
use crate::mrc::Mrc;
use crate::render::RenderFn;
use crate::style::length::{Length, LengthOrPercent};
use crate::style::{FixedStyleProp, ResolvedStyleProp, StylePropKey, StylePropVal};
use crate::text::textbox::TextBox;
use crate::window::popup::Popup;
use crate::{js_deserialize, js_serialize, ok_or_return, some_or_return};
use deft_macros::{element_backend, event, js_methods};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::DerefMut;

#[derive(Serialize, Deserialize, Clone)]
pub struct SelectOption {
    value: String,
    label: String,
}

js_serialize!(SelectOption);
js_deserialize!(SelectOption);

#[event]
pub struct ChangeEvent {}

#[element_backend]
pub struct Select {
    element_weak: ElementWeak,
    label: Label,
    placeholder: TextBox,
    value: String,
    options: Vec<SelectOption>,
    select_img: ImageObject,
    options_style: Vec<FixedStyleProp>,
    option_style: Vec<FixedStyleProp>,
    option_hover_style: Vec<FixedStyleProp>,
}

#[js_methods]
impl Select {
    #[js_func]
    pub fn set_value(&mut self, value: String) {
        if self.value != value {
            let label = self
                .options
                .iter()
                .find(|o| o.value == value)
                .map(|it| &it.label)
                .unwrap_or(&value)
                .to_string();
            self.label.set_text(label);
            self.value = value;
            self.element_weak.emit(ChangeEvent {});
        }
    }

    #[js_func]
    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    #[js_func]
    pub fn set_options(&mut self, options: Vec<SelectOption>) {
        self.options = options;
    }

    #[js_func]
    pub fn get_options(&self) -> Vec<SelectOption> {
        self.options.clone()
    }

    #[js_func]
    pub fn set_placeholder(&mut self, value: String) {
        self.placeholder.clear();
        self.placeholder.add_line(Editable::build_line(value));
        self.element_weak.mark_dirty(false);
    }

    #[js_func]
    pub fn get_placeholder(&self) -> String {
        self.placeholder.get_text()
    }

    fn build_options_element<F: FnOnce(String) + Clone + 'static>(
        &self,
        value_setter: F,
    ) -> Element {
        let mut wrapper = Element::create(Container::create);
        wrapper.set_style_props(self.options_style.clone());
        for option in &self.options {
            let mut label_el = Element::create(Label::create);
            label_el.set_style_props(self.option_style.clone());
            label_el.set_hover_styles(self.option_hover_style.clone());
            let setter = value_setter.clone();
            let value = option.clone();
            label_el.register_event_listener(ClickEventListener::new(move |_e, _ctx| {
                (setter.clone())(value.value.clone());
            }));
            let mut label = label_el.get_backend_as::<Label>().clone();
            label.set_text(option.label.clone());
            wrapper
                .add_child(label_el, wrapper.children.len() as i32)
                .unwrap();
        }
        wrapper
    }
}

impl ElementBackend for Select {
    fn create(element: &mut Element) -> Self {
        element.is_form_element = true;
        element.register_js_event::<ChangeEvent>("change");
        let label_el = Element::create(Label::create);
        let label = label_el.get_backend_as::<Label>().clone();
        element.add_child(label_el.clone(), 0).unwrap();

        let select_img = ImageObject::from_svg_bytes(include_bytes!("./select.svg"));

        let placeholder = TextBox::new();

        SelectData {
            element_weak: element.as_weak(),
            placeholder,
            label,
            value: "".to_string(),
            options: vec![],
            select_img,
            options_style: vec![],
            option_style: vec![],
            option_hover_style: vec![],
        }
        .to_ref()
    }
    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }

    fn render(&mut self) -> RenderFn {
        let el = ok_or_return!(self.element_weak.upgrade(), RenderFn::empty());
        let bounds = el.get_bounds();
        let (img_width, img_height) = self.select_img.get_container_size();
        let y = (bounds.height - img_height) / 2.0;
        let x = bounds.width - img_width - y;
        let img = self.select_img.render();
        let placeholder_renderer = if self.label.get_text().is_empty() {
            Some(self.placeholder.render())
        } else {
            None
        };
        let (pt, _, _, pl) = el.get_padding();
        RenderFn::new(move |painter| {
            if let Some(pr) = placeholder_renderer {
                painter.canvas.session(|c| {
                    c.translate((pl, pt));
                    pr.run(painter);
                });
            }
            painter.canvas.translate((x, y));
            img.run(painter);
        })
    }

    fn on_event(&mut self, event: &mut Event, _ctx: &mut EventContext<ElementWeak>) {
        let el = ok_or_return!(self.element_weak.upgrade());
        let w = some_or_return!(el.get_window());
        let window = ok_or_return!(w.upgrade_mut());
        let bounds = el.get_origin_bounds();
        if let Some(_) = ClickEvent::cast(event) {
            let mut popup: Mrc<Option<Popup>> = Mrc::new(None);
            let mut popup_mrc = popup.clone();
            let weak = self.as_weak();
            let value_setter = move |v| {
                let mut select = ok_or_return!(weak.upgrade());
                select.set_value(v);
                if let Some(p) = popup_mrc.deref_mut() {
                    let _ = p.clone().close();
                }
            };
            let mut options_el = self.build_options_element(value_setter);
            options_el.set_style_props(vec![FixedStyleProp::MinWidth(StylePropVal::Custom(
                LengthOrPercent::Length(Length::PX(bounds.width)),
            ))]);
            *popup = Some(Popup::new(options_el, bounds, &w));
        }
    }
    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        if let Some(styles) = styles.get("options") {
            let styles: Vec<FixedStyleProp> = styles.iter().map(|it| it.to_unresolved()).collect();
            self.options_style = styles;
        }
        if let Some(styles) = styles.get("option") {
            let styles: Vec<FixedStyleProp> = styles.iter().map(|it| it.to_unresolved()).collect();
            self.option_style = styles;
        }
        if let Some(styles) = styles.get("option-hover") {
            let styles: Vec<FixedStyleProp> = styles.iter().map(|it| it.to_unresolved()).collect();
            self.option_hover_style = styles;
        }
        if let Some(placeholder_styles) = styles.get("placeholder") {
            for style in placeholder_styles {
                match style {
                    ResolvedStyleProp::Color(color) => {
                        self.placeholder.set_color(*color);
                        self.placeholder.layout();
                    }
                    _ => {}
                }
            }
        }
    }
    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        let el = ok_or_return!(self.element_weak.upgrade());
        let content_bounds = el.get_content_bounds();
        let height = bounds.height - 4.0;
        self.select_img.set_container_size((height, height));
        self.placeholder
            .set_line_height(Some(content_bounds.height));
        self.placeholder.layout();
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        let mut el = ok_or_return!(self.element_weak.upgrade());
        match key {
            StylePropKey::Color => {
                if self.select_img.set_color(el.style.color) {
                    el.mark_dirty(false);
                }
            }
            _ => {}
        }
    }
}
