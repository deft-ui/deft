use crate as deft;
use crate::mrc::MrcWeak;
use crate::style::border::parse_border;
use crate::style::{parse_style_obj, FixedStyleProp, PropValueParse, StylePropKey, StylePropVal};
use deft_macros::mrc_object;
use quick_js::JsValue;
use std::collections::HashMap;
use yoga::PositionType;

type CssValueResolver = Box<dyn Fn(&HashMap<String, String>) -> String>;

pub enum ParsedStyleProp {
    Fixed(FixedStyleProp),
    Var(String, String, CssValueResolver, Option<FixedStyleProp>),
}

impl PartialEq for ParsedStyleProp {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ParsedStyleProp::Fixed(v) => match other {
                ParsedStyleProp::Fixed(o) => v == o,
                ParsedStyleProp::Var(_, _, _, _) => false,
            },
            ParsedStyleProp::Var(k, v, _, _) => match other {
                ParsedStyleProp::Fixed(_) => false,
                ParsedStyleProp::Var(ok, ov, _, _) => k == ok && v == ov,
            },
        }
    }
}

impl ParsedStyleProp {
    pub fn parse(key: &str, value: &str) -> Vec<Self> {
        let mut result = Vec::new();
        let list = StyleList::expand_style(key, value);
        for (k, v) in list {
            if let Some(compute_fn) = StyleList::parse_variables(&v) {
                result.push(ParsedStyleProp::Var(
                    k.to_string(),
                    v,
                    Box::new(compute_fn),
                    None,
                ));
            } else {
                StyleList::str_to_style_prop(k, &v, &mut |p| {
                    result.push(ParsedStyleProp::Fixed(p));
                });
            }
        }
        result
    }

    pub fn fix(&mut self, vars: &MrcWeak<HashMap<String, String>>) {
        match self {
            ParsedStyleProp::Fixed(_v) => {}
            ParsedStyleProp::Var(key, _v, resolver, fixed) => {
                *fixed = None;
                if let Ok(vars) = vars.upgrade() {
                    let v = resolver(&vars);
                    StyleList::str_to_style_prop(&key, &v, &mut |c| *fixed = Some(c));
                }
            }
        }
    }
    pub fn fixed(&self) -> Option<FixedStyleProp> {
        match self {
            ParsedStyleProp::Fixed(v) => Some(v.clone()),
            ParsedStyleProp::Var(_, _, _, v) => v.clone(),
        }
    }
    pub fn key(&self) -> StylePropKey {
        match self {
            ParsedStyleProp::Fixed(p) => p.key(),
            ParsedStyleProp::Var(k, _v, _, _) => {
                //TODO no unwrap
                StylePropKey::parse(k).unwrap()
            }
        }
    }
}

#[mrc_object]
pub struct StyleList {
    default_style_props: HashMap<StylePropKey, ParsedStyleProp>,
    values: HashMap<StylePropKey, ParsedStyleProp>,
    hover_style_props: HashMap<StylePropKey, ParsedStyleProp>,
    selector_style_props: HashMap<StylePropKey, ParsedStyleProp>,
    pseudo_element_style_props: HashMap<String, HashMap<StylePropKey, ParsedStyleProp>>,
    pub(crate) variables: MrcWeak<HashMap<String, String>>,
}

impl StyleList {
    pub fn new() -> StyleList {
        let mut default_style_props = HashMap::new();
        let default_styles = vec![
            FixedStyleProp::Position(StylePropVal::Custom(PositionType::Static)),
            FixedStyleProp::Color(StylePropVal::Inherit),
            FixedStyleProp::FontSize(StylePropVal::Inherit),
            FixedStyleProp::LineHeight(StylePropVal::Inherit),
            FixedStyleProp::FontFamily(StylePropVal::Inherit),
            FixedStyleProp::FontWeight(StylePropVal::Inherit),
            FixedStyleProp::FontStyle(StylePropVal::Inherit),
        ];
        for d in default_styles {
            default_style_props.insert(d.key(), ParsedStyleProp::Fixed(d));
        }
        StyleListData {
            default_style_props,
            values: HashMap::new(),
            hover_style_props: HashMap::new(),
            selector_style_props: HashMap::new(),
            variables: MrcWeak::new(),
            pseudo_element_style_props: HashMap::new(),
        }
        .to_ref()
    }

    pub fn set_variables(&mut self, variables: MrcWeak<HashMap<String, String>>) {
        StyleList::fix_variables(&mut self.values, &variables);
        StyleList::fix_variables(&mut self.hover_style_props, &variables);
        StyleList::fix_variables(&mut self.selector_style_props, &variables);
        self.variables = variables;
    }

    fn fix_variables(
        table: &mut HashMap<StylePropKey, ParsedStyleProp>,
        variables: &MrcWeak<HashMap<String, String>>,
    ) {
        for (_k, v) in table {
            v.fix(variables);
        }
    }

    fn collect_fixed_props(
        table: &HashMap<StylePropKey, ParsedStyleProp>,
        result: &mut HashMap<StylePropKey, FixedStyleProp>,
    ) {
        for (_, v) in table {
            if let Some(v) = v.fixed() {
                result.insert(v.key(), v);
            }
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn set_style_props(&mut self, styles: Vec<FixedStyleProp>) {
        for style_prop in &styles {
            self.values
                .insert(style_prop.key(), ParsedStyleProp::Fixed(style_prop.clone()));
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
            p.fix(&self.variables);
            self.values.insert(p.key(), p);
        }
    }

    pub fn get_styles(&self, is_hover: bool) -> HashMap<StylePropKey, FixedStyleProp> {
        let mut result = HashMap::new();
        Self::collect_fixed_props(&self.default_style_props, &mut result);
        Self::collect_fixed_props(&self.selector_style_props, &mut result);
        Self::collect_fixed_props(&self.values, &mut result);
        if is_hover {
            Self::collect_fixed_props(&self.hover_style_props, &mut result);
        }
        result
    }

    pub fn get_pseudo_element_style_props(
        &self,
    ) -> HashMap<String, HashMap<StylePropKey, FixedStyleProp>> {
        let mut result = HashMap::new();
        for (name, styles) in self.pseudo_element_style_props.iter() {
            let mut props = HashMap::new();
            Self::collect_fixed_props(styles, &mut props);
            result.insert(name.clone(), props);
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
        let mut new_style_props = HashMap::new();
        self.parse_style_vec(&styles, &mut new_style_props);
        if new_style_props != self.selector_style_props {
            self.selector_style_props = new_style_props;
            true
        } else {
            false
        }
    }

    pub fn set_pseudo_element_style(&mut self, styles_map: HashMap<String, Vec<String>>) -> bool {
        let mut pseudo_element_props = HashMap::new();
        for (k, styles) in styles_map {
            let mut parsed_styles = HashMap::new();
            self.parse_style_vec(&styles, &mut parsed_styles);
            pseudo_element_props.insert(k, parsed_styles);
        }
        if pseudo_element_props != self.pseudo_element_style_props {
            self.pseudo_element_style_props = pseudo_element_props;
            true
        } else {
            false
        }
    }

    fn parse_style_vec(
        &self,
        styles: &Vec<String>,
        result: &mut HashMap<StylePropKey, ParsedStyleProp>,
    ) {
        for s in styles {
            let list = Self::parse_style(s);
            for mut p in list {
                p.fix(&self.variables);
                result.insert(p.key(), p);
            }
        }
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
        let mut result = Vec::new();
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
                let mut result = Vec::new();
                result.append(&mut Self::expand_style("BorderTop", v_str));
                result.append(&mut Self::expand_style("BorderRight", v_str));
                result.append(&mut Self::expand_style("BorderBottom", v_str));
                result.append(&mut Self::expand_style("BorderLeft", v_str));
                result
            }
            "margin" => {
                let (t, r, b, l) = crate::style::parse_box_prop(v_str, "none");
                vec![
                    ("MarginTop", t),
                    ("MarginRight", r),
                    ("MarginBottom", b),
                    ("MarginLeft", l),
                ]
            }
            "padding" => {
                let (t, r, b, l) = crate::style::parse_box_prop(v_str, "none");
                vec![
                    ("PaddingTop", t),
                    ("PaddingRight", r),
                    ("PaddingBottom", b),
                    ("PaddingLeft", l),
                ]
            }
            "borderradius" => {
                let (t, r, b, l) = crate::style::parse_box_prop(v_str, "none");
                vec![
                    ("BorderTopLeftRadius", t),
                    ("BorderTopRightRadius", r),
                    ("BorderBottomRightRadius", b),
                    ("BorderBottomLeftRadius", l),
                ]
            }
            "bordertop" => {
                let (width, color) = parse_border(v_str);
                //TODO no re-encoding?
                vec![
                    ("BorderTopWidth", width.to_style_string()),
                    ("BorderTopColor", color.to_style_string()),
                ]
            }
            "borderright" => {
                let (width, color) = parse_border(v_str);
                //TODO no re-encoding?
                vec![
                    ("BorderRightWidth", width.to_style_string()),
                    ("BorderRightColor", color.to_style_string()),
                ]
            }
            "borderbottom" => {
                let (width, color) = parse_border(v_str);
                //TODO no re-encoding?
                vec![
                    ("BorderBottomWidth", width.to_style_string()),
                    ("BorderBottomColor", color.to_style_string()),
                ]
            }
            "borderleft" => {
                let (width, color) = parse_border(v_str);
                //TODO no re-encoding?
                vec![
                    ("BorderLeftWidth", width.to_style_string()),
                    ("BorderLeftColor", color.to_style_string()),
                ]
            }
            _ => {
                vec![(k, v_str.to_string())]
            }
        }
    }

    fn str_to_style_prop<C: FnMut(FixedStyleProp)>(k: &str, v_str: &str, c: &mut C) {
        let k = k.to_lowercase().replace("-", "");
        if let Some(sp) = FixedStyleProp::parse(&k, v_str) {
            c(sp);
        }
    }

    fn is_variable_name_start(char: char) -> bool {
        char.is_ascii_alphabetic() || char == '_'
    }

    fn is_variable_name_continue(char: char) -> bool {
        char.is_ascii_alphanumeric() || char == '_' || char == '-'
    }

    fn parse_variables(value: &str) -> Option<Box<dyn Fn(&HashMap<String, String>) -> String>> {
        let mut keys = Vec::new();
        let chars = value.chars().collect::<Vec<_>>();
        if chars.len() < 2 {
            return None;
        }
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
                let str = value.as_str();
                let mut consumed = 0;
                let empty = String::from("");
                for (k, start) in &keys {
                    if *start - consumed > 0 {
                        result.push_str(&str[consumed..*start]);
                    }
                    let var_value = variables.get(&k[1..]).unwrap_or(&empty);
                    result.push_str(&var_value);
                    consumed = start + k.len();
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

#[cfg(test)]
pub mod tests {
    use crate::computed::ComputedValue;

    #[test]
    fn test_style_manager() {
        let style_vars = ComputedValue::new();
        style_vars.update_value("height", "5".to_string());
        // let sm = StyleList::new();
        // sm.bind_style_variables(&style_vars);
        // sm.parse_style("width", "4");
        // sm.parse_style("transform", "translate(0, $height)");
        // style_vars.update_value("height", "6".to_string());
    }
}
