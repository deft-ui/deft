use anyhow::Error;
use lightningcss::printer::PrinterOptions;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use std::collections::HashMap;
use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::{Element, ElementBackend};
use crate::style::select::Selectors;

pub struct CssManager {
    rules: Vec<(Selectors, String)>,
}

impl CssManager {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn add(&mut self, stylesheet_source: &str) -> Result<(), Error> {
        //TODO no unwrap
        let mut stylesheet = StyleSheet::parse(&stylesheet_source, ParserOptions::default()).unwrap();
        let rules = &mut stylesheet.rules;
        for rule in &mut rules.0 {
            if let CssRule::Style(rule) = rule {
                let selectors = rule.selectors.to_css_string(PrinterOptions::default())?;
                let decl = rule.declarations.to_css_string(PrinterOptions::default())?;
                //println!("selectors: {:?} => {:?}", selectors, decl);
                //TODO no unwrap
                let selectors = Selectors::compile(&selectors).unwrap();
                self.rules.push((selectors, decl));
            }
        }
        Ok(())
    }

    pub fn match_styles(&self, element: &Element) -> Vec<String> {
        let mut list = Vec::new();
        for (selectors, rule) in &self.rules {
            if selectors.matches(element) {
                list.push(rule.to_string());
            }
        }
        list
    }

}

#[test]
fn test_css_manager() {
    let mut manager = CssManager::new();
    manager.add(include_str!("../../tests/demo.css")).unwrap();
    assert_eq!(2, manager.rules.len());
    let button = Element::create(Button::create);
    let container = Element::create(Container::create);
    let containers_styles = manager.match_styles(&container);
    let button_styles = manager.match_styles(&button);
    assert_eq!(1, containers_styles.len());
    assert_eq!(1, button_styles.len());
}
