use crate as deft;
use std::collections::HashMap;
use std::mem;
use deft_macros::mrc_object;
use log::debug;
use quick_js::JsValue;
use skia_safe::wrapper::NativeTransmutableWrapper;
use crate::computed::{ComputedValue, ComputedValueHandle};
use crate::{some_or_break, some_or_return};
use crate::style::{parse_style_obj, Style, StyleProp, StylePropKey, StylePropertyValue};

type CssValueResolver = Box<dyn Fn(&HashMap<String, String>) -> String>;

#[mrc_object]
pub struct StyleManager {
    var_values: HashMap<String, CssValueResolver>,
    values: HashMap<StylePropKey, StyleProp>,
    hover_style_props: HashMap<StylePropKey, StyleProp>,
    selector_style_props: HashMap<StylePropKey, StyleProp>,
}

impl StyleManager {
    pub fn new() -> StyleManager {
        StyleManagerData {
            var_values: HashMap::new(),
            values: HashMap::new(),
            hover_style_props: HashMap::new(),
            selector_style_props: HashMap::new(),
        }.to_ref()
    }

    pub fn resolve_variables(&mut self, variables: &HashMap<String, String>) {
        let mut values = HashMap::new();
        for (k, v) in &mut self.var_values {
            let resolved_value = v(variables);
            values.insert(k.clone(), resolved_value);
        }
        for (k, v) in variables {
            Self::str_to_style_prop(k, v, &mut |p: StyleProp| {
                self.values.insert(p.key(), p);
            });
        }
    }

    pub fn clear(&mut self) {
        self.var_values.clear();
        self.values.clear();
    }

    pub fn set_style_props(&mut self, styles: Vec<StyleProp>) {
        for style_prop in &styles {
            self.values.insert(style_prop.key(), style_prop.clone());
        }
    }

    pub fn set_style_obj(&mut self, style: JsValue) {
        if let JsValue::String(str) = &style {
            //TODO maybe style value contains ';' char ?
            for (k, v) in Self::parse_style_list(str) {
                self.set_style_str(k, v);
            }
        } else if let Some(obj) = style.get_properties() {
            //TODO use default style
            obj.into_iter().for_each(|(k, v)| {
                let v_str = match v {
                    JsValue::String(s) => s,
                    JsValue::Int(i) => i.to_string(),
                    JsValue::Float(f) => f.to_string(),
                    _ => return,
                };
                self.set_style_str(&k, &v_str);
            });
        }
    }

    pub fn set_style_str(&mut self, k: &str, v_str: &str) {
        if let Some(compute_fn) = Self::parse_variables(v_str) {
            self.var_values.insert(k.to_string(), compute_fn);
            return;
        }
        Self::str_to_style_prop(k, v_str, &mut |p| {
            self.values.insert(p.key(), p);
        });
    }

    pub fn get_styles(&self, is_hover: bool) -> HashMap<StylePropKey, StyleProp> {
        let mut style_props = self.selector_style_props.clone();
        for (k, v) in &self.values {
            style_props.insert(k.clone(), v.clone());
        }
        if is_hover {
            for (k, v) in &self.hover_style_props {
                style_props.insert(k.clone(), v.clone());
            }
        }
        style_props
    }

    pub fn remove_style(&mut self, key: &StylePropKey) {
        self.values.remove(key);
    }

    pub fn set_hover_style(&mut self, style: JsValue) {
        let styles = parse_style_obj(style);
        self.hover_style_props.clear();
        for st in styles {
            self.hover_style_props.insert(st.key().clone(), st);
        }
    }

    pub fn set_selector_style(&mut self, styles: Vec<String>) -> bool {
        let old_style_props = mem::take(&mut self.selector_style_props);
        for s in &styles {
            let list = Self::parse_style_list(s);
            for (k, v) in list {
                Self::str_to_style_prop(k, v, &mut |p: StyleProp| {
                    self.selector_style_props.insert(p.key(), p);
                });
            }
        }
        self.selector_style_props != old_style_props
    }

    pub fn has_hover_style(&self) -> bool {
        !self.hover_style_props.is_empty()
    }


    fn parse_style_list(str: &str) -> Vec<(&str, &str)> {
        let mut result =  Vec::new();
        str.split(';').for_each(|item| {
            if let Some((k, v)) = item.split_once(':') {
                result.push((k.trim(), v.trim()));
            }
        });
        result
    }

    fn str_to_style_prop<C: FnMut(StyleProp)>(k: &str, v_str: &str, c: &mut C) {
        if let Some(sp) = StyleProp::parse(k, v_str) {
            c(sp);
        } else {
            match k.to_lowercase().as_str() {
                "background" => {
                    Self::str_to_style_prop("BackgroundColor", v_str, c);
                }
                "gap" => {
                    Self::str_to_style_prop("RowGap", v_str, c);
                    Self::str_to_style_prop("ColumnGap", v_str, c);
                }
                "border" => {
                    Self::str_to_style_prop("BorderTop", v_str, c);
                    Self::str_to_style_prop("BorderRight", v_str, c);
                    Self::str_to_style_prop("BorderBottom", v_str, c);
                    Self::str_to_style_prop("BorderLeft", v_str, c);
                }
                "margin" => {
                    let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                    Self::str_to_style_prop("MarginTop", &t.to_str("none"), c);
                    Self::str_to_style_prop("MarginRight", &r.to_str("none"), c);
                    Self::str_to_style_prop("MarginBottom", &b.to_str("none"), c);
                    Self::str_to_style_prop("MarginLeft", &l.to_str("none"), c);
                }
                "padding" => {
                    let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                    Self::str_to_style_prop("PaddingTop", &t.to_str("none"), c);
                    Self::str_to_style_prop("PaddingRight", &r.to_str("none"), c);
                    Self::str_to_style_prop("PaddingBottom", &b.to_str("none"), c);
                    Self::str_to_style_prop("PaddingLeft", &l.to_str("none"), c);
                }
                "borderradius" => {
                    let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                    Self::str_to_style_prop("BorderTopLeftRadius", &t.to_str("none"), c);
                    Self::str_to_style_prop("BorderTopRightRadius", &r.to_str("none"), c);
                    Self::str_to_style_prop("BorderBottomRightRadius", &b.to_str("none"), c);
                    Self::str_to_style_prop("BorderBottomLeftRadius", &l.to_str("none"), c);
                }
                _ => {}
            }
        }
    }

    fn is_variable_name_start(char: char) -> bool {
        char.is_ascii_alphabetic() || char == '_'
    }

    fn is_variable_name_continue(char: char) -> bool {
        char.is_ascii_alphanumeric() || char == '_' || char == '-'
    }

    fn parse_variables(mut value: &str) -> Option<Box<dyn Fn(&HashMap<String, String>) -> String>> {
        let mut keys = Vec::new();
        let chars = value.chars().collect::<Vec<_>>();
        let mut i = 0;
        while i < chars.len() - 1 {
            if chars[i] == '$' && Self::is_variable_name_start(chars[i + 1]) {
                let key_start = i;
                i += 2;
                while i < chars.len() && Self::is_variable_name_continue(chars[i]) {
                    i += 1;
                }
                let key = String::from_iter(&chars[key_start..i]);
                keys.push((key, key_start));
            } else {
                i += 1;
            }
        }
        if !keys.is_empty() {
            let value = value.to_string();
            let compute = Box::new(move |variables: &HashMap<String, String>| {
                let mut result = String::new();
                let mut str = value.as_str();
                let mut offset = 0;
                let mut consumed = 0;
                let empty = String::from("");
                for (k, start) in &keys {
                    if *start - consumed > 0 {
                        result.push_str(&str[consumed..*start]);
                    }
                    let var_value = variables.get(k).unwrap_or(&empty);
                    result.push_str(&var_value);
                    consumed = start + k.len();
                    offset += 1;
                }
                if str.len() > consumed {
                    result.push_str(&str[consumed..]);
                }
                result
            });
            Some(compute)
        } else {
            None
        }
    }

}

#[test]
fn test_style_manager() {
    let style_vars = ComputedValue::new();
    style_vars.update_value("height", "5".to_string());
    let mut sm = StyleManager::new();
    // sm.bind_style_variables(&style_vars);
    // sm.parse_style("width", "4");
    // sm.parse_style("transform", "translate(0, $height)");
    // style_vars.update_value("height", "6".to_string());
}