use crate as deft;
use crate::style::border::parse_border;
use crate::style::style_vars::StyleVars;
use crate::style::var_expr::StyleExpr;
use crate::style::{parse_style_obj, FixedStyleProp, PropValueParse, StylePropKey, StylePropVal};
use deft_macros::mrc_object;
use quick_js::JsValue;
use std::collections::{HashMap, HashSet};
use yoga::PositionType;

type CssValueResolver = Box<dyn Fn(&HashMap<String, String>) -> String>;

pub enum ParsedStyleProp {
    Fixed(FixedStyleProp),
    Var(String, String, StyleExpr, Vec<FixedStyleProp>),
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
    pub fn parse_all<T: AsRef<str>>(list: Vec<(T, T)>) -> (Vec<Self>, StyleVars) {
        let mut vars = StyleVars::new();
        let mut styles = Vec::new();
        for (k, v) in list {
            let k = k.as_ref();
            let v = v.as_ref();
            if k.starts_with("--") {
                vars.set(&k[2..].to_string(), v);
            } else {
                for e in Self::parse(k, v) {
                    styles.push(e);
                }
            }
        }
        (styles, vars)
    }

    fn parse(key: &str, value: &str) -> Vec<Self> {
        let mut result = Vec::new();
        if let Some(style_expr) = StyleExpr::parse(&value) {
            result.push(ParsedStyleProp::Var(
                key.to_string(),
                value.to_string(),
                style_expr,
                Vec::new(),
            ));
        } else {
            let list = StyleList::expand_style(key, value);
            for (k, v) in list {
                StyleList::str_to_style_prop(k, &v, &mut |p| {
                    result.push(ParsedStyleProp::Fixed(p));
                });
            }
        }
        result
    }

    pub fn fix_var(&mut self, vars: &StyleVars) {
        match self {
            ParsedStyleProp::Fixed(_v) => {}
            ParsedStyleProp::Var(key, _v, resolver, fixed) => {
                fixed.clear();
                if let Some(v) = resolver.resolve(&vars) {
                    StyleList::str_to_style_prop(&key, &v, &mut |c| fixed.push(c));
                }
            }
        }
    }

    pub fn fixed(&self) -> Vec<FixedStyleProp> {
        match self {
            ParsedStyleProp::Fixed(v) => vec![v.clone()],
            ParsedStyleProp::Var(_, _, _, v) => v.clone(),
        }
    }
    pub fn key(&self) -> String {
        match self {
            ParsedStyleProp::Fixed(p) => p.key().name().to_lowercase(),
            ParsedStyleProp::Var(k, _v, _, _) => k.to_lowercase(),
        }
    }
}

#[mrc_object]
pub struct StyleList {
    vars: StyleVars,
    default_style_props: Vec<ParsedStyleProp>,
    values: Vec<ParsedStyleProp>,
    hover_style_props: Vec<ParsedStyleProp>,
    selector_style_props: Vec<ParsedStyleProp>,
    pseudo_element_style_props: HashMap<String, Vec<ParsedStyleProp>>,
}

impl StyleList {
    pub fn new() -> StyleList {
        let mut default_style_props = Vec::new();
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
            default_style_props.push(ParsedStyleProp::Fixed(d));
        }
        StyleListData {
            default_style_props,
            values: Vec::new(),
            hover_style_props: Vec::new(),
            selector_style_props: Vec::new(),
            pseudo_element_style_props: HashMap::new(),
            vars: StyleVars::new(),
        }
        .to_ref()
    }

    pub fn resolve_variables(&mut self, parent_vars: &StyleVars) -> StyleVars {
        let mut vars = parent_vars.clone();
        vars.merge(self.vars.clone());
        StyleList::fix_style_vars(&mut self.values, &vars);
        StyleList::fix_style_vars(&mut self.hover_style_props, &vars);
        StyleList::fix_style_vars(&mut self.selector_style_props, &vars);
        for (_, v) in &mut self.pseudo_element_style_props {
            StyleList::fix_style_vars(v, &vars);
        }
        vars
    }

    fn fix_style_vars(table: &mut Vec<ParsedStyleProp>, vars: &StyleVars) {
        for v in table {
            v.fix_var(vars);
        }
    }

    fn collect_fixed_props(
        table: &Vec<ParsedStyleProp>,
        result: &mut HashMap<StylePropKey, FixedStyleProp>,
    ) {
        for v in table {
            for p in v.fixed() {
                result.insert(p.key(), p);
            }
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn set_style_props(&mut self, styles: Vec<FixedStyleProp>) {
        for style_prop in &styles {
            self.values.push(ParsedStyleProp::Fixed(style_prop.clone()));
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
        for p in list {
            self.values.push(p);
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
        //NOTE: Does not support variables in inline hover-style
        let (styles, _vars) = parse_style_obj(style);
        self.hover_style_props.clear();
        for st in styles {
            self.hover_style_props.push(st);
        }
    }

    pub fn set_hover_styles(&mut self, styles: Vec<FixedStyleProp>) {
        self.hover_style_props.clear();
        for st in styles {
            self.hover_style_props.push(ParsedStyleProp::Fixed(st));
        }
    }

    pub fn set_selector_style(&mut self, styles: Vec<String>) -> bool {
        let (new_style_props, new_vars) = self.parse_style_vec(&styles);
        if new_style_props != self.selector_style_props || new_vars != self.vars {
            self.selector_style_props = new_style_props;
            self.vars = new_vars;
            true
        } else {
            false
        }
    }

    pub fn set_pseudo_element_style(&mut self, styles_map: HashMap<String, Vec<String>>) -> bool {
        let mut pseudo_element_props = HashMap::new();
        for (k, styles) in styles_map {
            let (parsed_styles, _) = self.parse_style_vec(&styles);
            pseudo_element_props.insert(k, parsed_styles);
        }
        if pseudo_element_props != self.pseudo_element_style_props {
            self.pseudo_element_style_props = pseudo_element_props;
            true
        } else {
            false
        }
    }

    fn parse_style_vec(&self, styles: &Vec<String>) -> (Vec<ParsedStyleProp>, StyleVars) {
        let mut all_list = Vec::new();
        let mut vars = StyleVars::new();
        for s in styles {
            let (mut list, s_vars) = Self::parse_style(s);
            all_list.append(&mut list);
            vars.merge(s_vars);
        }
        let mut result = Vec::new();
        let mut added = HashSet::new();
        for p in all_list.into_iter().rev() {
            if added.insert(p.key()) {
                result.push(p);
            }
        }
        result.reverse();
        (result, vars)
    }

    pub fn parse_style(style: &str) -> (Vec<ParsedStyleProp>, StyleVars) {
        let list = Self::parse_style_list(style);
        ParsedStyleProp::parse_all(list)
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
        if k.starts_with("--") {
            return vec![(&k[2..], v_str.to_string())];
        }
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
        let list = StyleList::expand_style(&k, v_str);
        for (k, v) in list {
            if let Some(sp) = FixedStyleProp::parse(&k, &v) {
                c(sp);
            }
        }
    }

    fn is_variable_name_start(char: char) -> bool {
        char.is_ascii_alphabetic() || char == '_'
    }

    fn is_variable_name_continue(char: char) -> bool {
        char.is_ascii_alphanumeric() || char == '_' || char == '-'
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
