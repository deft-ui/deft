use crate as deft;
use std::collections::HashMap;
use std::mem;
use deft_macros::mrc_object;
use log::debug;
use quick_js::JsValue;
use skia_safe::wrapper::NativeTransmutableWrapper;
use crate::computed::{ComputedValue, ComputedValueHandle};
use crate::{some_or_break, some_or_continue, some_or_return};
use crate::mrc::MrcWeak;
use crate::style::{parse_style_obj, Style, StyleProp, StylePropKey, StylePropertyValue};

type CssValueResolver = Box<dyn Fn(&HashMap<String, String>) -> String>;

pub enum ParsedStyleProp {
    Fixed(StyleProp),
    Var(String, String, CssValueResolver, Option<StyleProp>),
}

impl PartialEq for ParsedStyleProp {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ParsedStyleProp::Fixed(v) => {
                match other {
                    ParsedStyleProp::Fixed(o) => v == o,
                    ParsedStyleProp::Var(_, _, _, _) => false,
                }
            }
            ParsedStyleProp::Var(k, v, _, _) => {
                match other {
                    ParsedStyleProp::Fixed(_) => false,
                    ParsedStyleProp::Var(ok, ov, _, _) => k == ok && v == ov,
                }
            }
        }
    }
}

impl ParsedStyleProp {
    pub fn parse(key: &str, value: &str) -> Vec<Self> {
        let mut result = Vec::new();
        let list = StyleList::expand_style(key, value);
        for (k, v) in list {
            if let Some(compute_fn) = StyleList::parse_variables(&v) {
                let key = some_or_continue!(StylePropKey::parse(&k));
                result.push(ParsedStyleProp::Var(k.to_string(), v, Box::new(compute_fn), None));
            } else {
                StyleList::str_to_style_prop(k, &v, &mut |p| {
                    result.push(ParsedStyleProp::Fixed(p));
                });
            }
        }
        result
    }

    pub fn resolve(&mut self, vars: &MrcWeak<HashMap<String, String>>) {
        match self {
            ParsedStyleProp::Fixed(v) => {},
            ParsedStyleProp::Var(key, v, resolver, resolved) => {
                *resolved = None;
                if let Ok(vars) = vars.upgrade() {
                    let v = resolver(&vars);
                    StyleList::str_to_style_prop(&key, &v, &mut |c| *resolved = Some(c));
                }
            }
        }
    }
    pub fn resolved(&self) -> Option<StyleProp> {
        match self {
            ParsedStyleProp::Fixed(v) => Some(v.clone()),
            ParsedStyleProp::Var(_, _, _, v) => v.clone(),
        }
    }
    pub fn key(&self) -> StylePropKey {
        match self {
            ParsedStyleProp::Fixed(p) => p.key(),
            ParsedStyleProp::Var(k, v, _, _) => {
                //TODO no unwrap
                StylePropKey::parse(k).unwrap()
            }
        }
    }
}



#[mrc_object]
pub struct StyleList {
    values: HashMap<StylePropKey, ParsedStyleProp>,
    hover_style_props: HashMap<StylePropKey, ParsedStyleProp>,
    selector_style_props: HashMap<StylePropKey, ParsedStyleProp>,
    pub(crate) variables: MrcWeak<HashMap<String, String>>,
}

impl StyleList {
    pub fn new() -> StyleList {
        StyleListData {
            values: HashMap::new(),
            hover_style_props: HashMap::new(),
            selector_style_props: HashMap::new(),
            variables: MrcWeak::new(),
        }.to_ref()
    }

    pub fn set_variables(&mut self, variables: MrcWeak<HashMap<String, String>>) {
        StyleList::resolve_variables(&mut self.values, &variables);
        StyleList::resolve_variables(&mut self.hover_style_props, &variables);
        StyleList::resolve_variables(&mut self.selector_style_props, &variables);
        self.variables = variables;
    }

    fn resolve_variables(table: &mut HashMap<StylePropKey, ParsedStyleProp>, variables: &MrcWeak<HashMap<String, String>>)  {
        for (k, v) in table {
            v.resolve(variables);
        }
    }

    fn collect_resolved_props(table: &HashMap<StylePropKey, ParsedStyleProp>, result: &mut HashMap<StylePropKey, StyleProp>) {
        for (_, v) in table {
            if let Some(v) = v.resolved() {
                result.insert(v.key(), v);
            }
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn set_style_props(&mut self, styles: Vec<StyleProp>) {
        for style_prop in &styles {
            self.values.insert(style_prop.key(), ParsedStyleProp::Fixed(style_prop.clone()));
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
        let list = ParsedStyleProp::parse(k, v_str);
        for mut p in list {
            p.resolve(&self.variables);
            self.values.insert(p.key(), p);
        }
    }

    pub fn get_styles(&self, is_hover: bool) -> HashMap<StylePropKey, StyleProp> {
        let mut result = HashMap::new();
        Self::collect_resolved_props(&self.selector_style_props, &mut result);
        Self::collect_resolved_props(&self.values, &mut result);
        if is_hover {
            Self::collect_resolved_props(&self.hover_style_props, &mut result);
        }
        result
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
            let list = Self::parse_style(s);
            for mut p in list {
                p.resolve(&self.variables);
                self.selector_style_props.insert(p.key(), p);
            }
        }
        self.selector_style_props != old_style_props
    }

    pub fn parse_style(style: &str) -> Vec<ParsedStyleProp> {
        let list = Self::parse_style_list(style);
        let mut result = Vec::new();
        for (k, v) in list {
            let mut props = ParsedStyleProp::parse(k, v);
            result.append(&mut props);
        }
        result
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

    pub fn expand_style<'a>(k: &'a str, v_str: &str) -> Vec<(&'a str, String)> {
        let key = k.to_lowercase().replace("-", "");
        match key.as_str() {
            "background" => {
                vec![("BackgroundColor", v_str.to_string())]
            }
            "gap" => {
                vec![
                    ("RowGap", v_str.to_string()),
                    ("ColumnGap", v_str.to_string()),
                ]
            }
            "border" => {
                vec![
                    ("BorderTop", v_str.to_string()),
                    ("BorderRight", v_str.to_string()),
                    ("BorderBottom", v_str.to_string()),
                    ("BorderLeft", v_str.to_string()),

                ]
            }
            "margin" => {
                let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                vec![
                    ("MarginTop", t.to_str("none")),
                    ("MarginRight", r.to_str("none")),
                    ("MarginBottom", b.to_str("none")),
                    ("MarginLeft", l.to_str("none")),
                ]
            }
            "padding" => {
                let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                vec![
                    ("PaddingTop", t.to_str("none")),
                    ("PaddingRight", r.to_str("none")),
                    ("PaddingBottom", b.to_str("none")),
                    ("PaddingLeft", l.to_str("none")),
                ]
            }
            "borderradius" => {
                let (t, r, b, l) = crate::style::parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                vec![
                    ("BorderTopLeftRadius", t.to_str("none")),
                    ("BorderTopRightRadius", r.to_str("none")),
                    ("BorderBottomRightRadius", b.to_str("none")),
                    ("BorderBottomLeftRadius", l.to_str("none")),
                ]
            }
            _ => {
                vec![(k, v_str.to_string())]
            }
        }
    }

    fn str_to_style_prop<C: FnMut(StyleProp)>(k: &str, v_str: &str, c: &mut C) {
        let k = k.to_lowercase().replace("-", "");
        if let Some(sp) = StyleProp::parse(&k, v_str) {
            c(sp);
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
                    let var_value = variables.get(&k[1..]).unwrap_or(&empty);
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
    let mut sm = StyleList::new();
    // sm.bind_style_variables(&style_vars);
    // sm.parse_style("width", "4");
    // sm.parse_style("transform", "translate(0, $height)");
    // style_vars.update_value("height", "6".to_string());
}