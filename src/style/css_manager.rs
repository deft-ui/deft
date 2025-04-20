use anyhow::{anyhow, Error};
use lightningcss::printer::PrinterOptions;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use std::collections::HashMap;
use crate::base::{Id, IdKey};
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::{Element, ElementBackend};
use crate::style::select::{Selector, Selectors};

thread_local! {
    static STYLESHEET_ID_KEY: IdKey = IdKey::new();
}

pub struct CSS {
    id: Id<CSS>,
    rules: Vec<CSSRule>,
    declared_classes: Vec<String>,
    declared_attrs: Vec<String>,
}

pub struct CSSRule {
    selector: Selector,
    declarations: String,
    id: Id<CSS>,
}

pub struct CssManager {
    stylesheets: Vec<CSS>,
}

impl CssManager {
    pub fn new() -> Self {
        Self {
            stylesheets: Vec::new(),
        }
    }

    pub fn add(&mut self, stylesheet_source: &str) -> Result<Id<CSS>, Error> {
        let mut stylesheet = StyleSheet::parse(&stylesheet_source, ParserOptions::default())
            .map_err(|e| anyhow!("failed to parse css"))?;
        let rules = &mut stylesheet.rules;
        let id = Id::next(&STYLESHEET_ID_KEY);
        let mut css = CSS {
            id,
            rules: Vec::new(),
            declared_classes: Vec::new(),
            declared_attrs: Vec::new(),
        };
        for rule in &mut rules.0 {
            if let CssRule::Style(rule) = rule {
                let selectors = rule.selectors.to_css_string(PrinterOptions::default())?;
                let decl = rule.declarations.to_css_string(PrinterOptions::default())?;
                //println!("selectors: {:?} => {:?}", selectors, decl);
                let selectors = Selectors::compile(&selectors)?;
                for selector in selectors.0 {
                    css.declared_classes.append(&mut selector.get_classes().clone());
                    css.declared_attrs.append(&mut selector.get_attribute_names().clone());
                    let rule = CSSRule {
                        selector,
                        declarations: decl.clone(),
                        id,
                    };
                    css.rules.push(rule);
                }
            }
        }
        self.stylesheets.push(css);
        Ok(id)
    }

    pub fn remove(&mut self, id: &Id<CSS>) {
        self.stylesheets.retain(|css| css.id != *id);
    }

    pub fn contains_class(&self, clazz: &str)  -> bool {
        for ss in &self.stylesheets {
            for c in &ss.declared_classes {
                if c == clazz {
                   return true;
                }
            }
        }
        false
    }

    pub fn contains_attr(&self, attr: &str) -> bool {
        for ss in &self.stylesheets {
            for a in &ss.declared_attrs {
                if a == attr {
                    return true;
                }
            }
        }
        false
    }

    pub fn match_styles(&self, element: &Element) -> (Vec<String>, HashMap<String, String>) {
        let mut list = Vec::new();
        let mut pm = HashMap::new();
        let mut rules = Vec::new();
        for css in &self.stylesheets {
            for rule in &css.rules {
                if rule.selector.matches(element) {
                    rules.push(rule);
                }
            }
        }
        rules.sort_by(|a, b| {
            let a = a.selector.specificity();
            let b = b.selector.specificity();
            a.cmp(&b)
        });
        for rule in rules {
            let rule_str = rule.declarations.clone();
            if let Some(pe) = rule.selector.pseudo_element() {
                pm.insert(pe.name.clone(), rule_str);
            } else {
                list.push(rule_str);
            }
        }
        (list, pm)
    }

}

#[test]
fn test_css_manager() {
    let mut manager = CssManager::new();
    manager.add(include_str!("../../tests/demo.css")).unwrap();
    assert_eq!(1, manager.stylesheets.len());
    let button = Element::create(Button::create);
    let container = Element::create(Container::create);
    let (containers_styles, _) = manager.match_styles(&container);
    let (button_styles, _) = manager.match_styles(&button);
    assert_eq!(1, containers_styles.len());
    assert_eq!(1, button_styles.len());
}
