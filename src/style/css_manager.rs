use anyhow::{anyhow, Error};
use lightningcss::printer::PrinterOptions;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use std::collections::HashMap;
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::{Element, ElementBackend};
use crate::style::select::{Selector, Selectors};

pub struct CssManager {
    rules: Vec<(Selector, String)>,
}

impl CssManager {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn add(&mut self, stylesheet_source: &str) -> Result<(), Error> {
        let mut stylesheet = StyleSheet::parse(&stylesheet_source, ParserOptions::default())
            .map_err(|e| anyhow!("failed to parse css"))?;
        let rules = &mut stylesheet.rules;
        for rule in &mut rules.0 {
            if let CssRule::Style(rule) = rule {
                let selectors = rule.selectors.to_css_string(PrinterOptions::default())?;
                let decl = rule.declarations.to_css_string(PrinterOptions::default())?;
                //println!("selectors: {:?} => {:?}", selectors, decl);
                let selectors = Selectors::compile(&selectors)?;
                for selector in selectors.0 {
                    self.rules.push((selector, decl.clone()));
                }
            }
        }
        Ok(())
    }

    pub fn match_styles(&self, element: &Element) -> (Vec<String>, HashMap<String, String>) {
        let mut list = Vec::new();
        let mut pm = HashMap::new();
        for (selector, rule) in &self.rules {
            if selector.matches(element) {
                let rule_str = rule.to_string();
                if let Some(pe) = selector.pseudo_element() {
                    pm.insert(pe.name.clone(), rule_str);
                } else {
                    list.push(rule_str);
                }
            }
        }
        (list, pm)
    }

}

#[test]
fn test_css_manager() {
    let mut manager = CssManager::new();
    manager.add(include_str!("../../tests/demo.css")).unwrap();
    assert_eq!(2, manager.rules.len());
    let button = Element::create(Button::create);
    let container = Element::create(Container::create);
    let (containers_styles, _) = manager.match_styles(&container);
    let (button_styles, _) = manager.match_styles(&button);
    assert_eq!(1, containers_styles.len());
    assert_eq!(1, button_styles.len());
}
