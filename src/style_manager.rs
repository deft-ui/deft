use crate as deft;
use std::collections::HashMap;
use deft_macros::mrc_object;
use log::debug;
use quick_js::JsValue;
use skia_safe::wrapper::NativeTransmutableWrapper;
use crate::computed::{ComputedValue, ComputedValueHandle};
use crate::{some_or_break, some_or_return};
use crate::style::{Style, StyleProp, StylePropKey, StylePropertyValue};

#[mrc_object]
pub struct StyleManager {
    raw_expressions: HashMap<String, String>,
    values: HashMap<StylePropKey, StyleProp>,
    computed_handles: HashMap<String, ComputedValueHandle>,
    variables: ComputedValue<String>,
}

impl StyleManager {
    pub fn new() -> StyleManager {
        StyleManagerData {
            raw_expressions: HashMap::new(),
            computed_handles: HashMap::new(),
            values: HashMap::new(),
            variables: ComputedValue::new(),
        }.to_ref()
    }

    pub fn bind_style_variables(&mut self, variables: &ComputedValue<String>) {
        //TODO dont save?
        self.variables = variables.clone();
        for (k, v) in self.raw_expressions.clone() {
            self.parse_style(&k, &v);
        }
    }

    pub fn parse_style_obj(&mut self, style: JsValue) {
        if let JsValue::String(str) = &style {
            //TODO maybe style value contains ';' char ?
            str.split(';').for_each(|item| {
                if let Some((k, v)) = item.split_once(':') {
                    self.parse_style(k.trim(), v.trim());
                }
            });
        } else if let Some(obj) = style.get_properties() {
            //TODO use default style
            obj.into_iter().for_each(|(k, v)| {
                let v_str = match v {
                    JsValue::String(s) => s,
                    JsValue::Int(i) => i.to_string(),
                    JsValue::Float(f) => f.to_string(),
                    _ => return,
                };
                self.parse_style(&k, &v_str);
            });
        }
    }

    pub fn get_styles(&self) -> &HashMap<StylePropKey, StyleProp> {
        &self.values
    }

    fn parse(&mut self, key: &str, value: &str) -> bool {
        if let Some(p) = StyleProp::parse(key, value) {
            self.values.insert(p.key(), p);
            true
        } else {
            false
        }
    }

    fn parse_style(&mut self, k: &str, v_str: &str) {
        let k = k.to_lowercase();
        let weak = self.as_weak();
        let key_str = k.to_string();
        let key2 = k.to_string();
        if let Some(handle) = Self::parse_variables(v_str, &self.variables, move |v| {
            if let Ok(mut sm) = weak.upgrade() {
                sm.parse(&key_str, &v);
            }
        }) {
            self.raw_expressions.insert(k.clone(), v_str.to_string());
            self.computed_handles.insert(key2, handle);
            return;
        };
        self.computed_handles.remove(&k.to_string());
        if !self.parse(&k, v_str) {
            let key = k;
            let k = key.as_str();
            match k {
                "background" => {
                    self.parse("BackgroundColor", v_str);
                },
                "gap" => {
                    self.parse("RowGap", v_str);
                    self.parse("ColumnGap", v_str);
                },
                "border" => {
                    self.parse("BorderTop", v_str);
                    self.parse("BorderRight", v_str);
                    self.parse("BorderBottom", v_str);
                    self.parse("BorderLeft", v_str);
                },
                "margin" => {
                    let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                    self.parse("MarginTop", &t.to_str("none"));
                    self.parse("MarginRight", &r.to_str("none"));
                    self.parse("MarginBottom", &b.to_str("none"));
                    self.parse("MarginLeft", &l.to_str("none"));
                }
                "padding" => {
                    let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                    self.parse("PaddingTop", &t.to_str("none"));
                    self.parse("PaddingRight", &r.to_str("none"));
                    self.parse("PaddingBottom", &b.to_str("none"));
                    self.parse("PaddingLeft", &l.to_str("none"));
                }
                "borderradius" => {
                    let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                    self.parse("BorderTopLeftRadius", &t.to_str("none"));
                    self.parse("BorderTopRightRadius", &r.to_str("none"));
                    self.parse("BorderBottomRightRadius", &b.to_str("none"));
                    self.parse("BorderBottomLeftRadius", &l.to_str("none"));
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

    fn parse_variables<F: FnMut(String) + 'static>(mut value: &str, variables: &ComputedValue<String>, mut value_consumer: F) -> Option<ComputedValueHandle> {
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
            let variables2 = variables.clone();
            let key_names = keys.iter().map(|(k, _)| (&k.as_str()[1..]).to_string()).collect();
            let handle = variables.dep(&key_names, move |v| {
                let mut result = String::new();
                let mut str = value.as_str();
                let variables = variables2.clone();
                let mut offset = 0;
                let mut consumed = 0;
                for (k, start) in &keys {
                    if *start - consumed > 0 {
                        result.push_str(&str[consumed..*start]);
                    }
                    let var_value = &v[offset];
                    result.push_str(&var_value);
                    consumed = start + k.len();
                    offset += 1;
                }
                if str.len() > consumed {
                    result.push_str(&str[consumed..]);
                }
                value_consumer(result);
            });
            Some(handle)
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
    sm.bind_style_variables(&style_vars);
    sm.parse_style("width", "4");
    sm.parse_style("transform", "translate(0, $height)");
    style_vars.update_value("height", "6".to_string());
}