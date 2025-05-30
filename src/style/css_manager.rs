use crate::base::{Id, IdKey};
use crate::element::Element;
use crate::style::select::{Selector, Selectors};
use anyhow::{anyhow, Error};
use simplecss::StyleSheet;
use std::collections::HashMap;

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
        let id = Id::next(&STYLESHEET_ID_KEY);
        let mut css = CSS {
            id,
            rules: Vec::new(),
            declared_classes: Vec::new(),
            declared_attrs: Vec::new(),
        };
        Self::update_css(&mut css, stylesheet_source)?;
        self.stylesheets.push(css);
        Ok(id)
    }

    pub fn update(&mut self, id: &Id<CSS>, stylesheet_source: &str) -> Result<(), Error> {
        for css in &mut self.stylesheets {
            if css.id == *id {
                Self::update_css(css, stylesheet_source)?;
                return Ok(());
            }
        }
        Err(anyhow!("style sheet not found: {}", id))
    }

    pub fn remove(&mut self, id: &Id<CSS>) {
        self.stylesheets.retain(|css| css.id != *id);
    }

    pub fn contains_class(&self, clazz: &str) -> bool {
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

    pub fn match_styles(&self, element: &Element) -> (Vec<String>, HashMap<String, Vec<String>>) {
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
                pm.entry(pe.name.clone())
                    .or_insert_with(Vec::new)
                    .push(rule_str);
            } else {
                list.push(rule_str);
            }
        }
        (list, pm)
    }

    fn update_css(css: &mut CSS, stylesheet_source: &str) -> Result<(), Error> {
        css.declared_classes.clear();
        css.declared_attrs.clear();
        css.rules.clear();
        let stylesheet = StyleSheet::parse(&stylesheet_source);
        for rule in &stylesheet.rules {
            let selectors = rule.selector.source().to_string();
            let mut declarations = Vec::new();
            for decl in &rule.declarations {
                declarations.push(format!("{}:{}", decl.name, decl.value));
            }
            //println!("selectors: {:?} => {:?}", selectors, declarations.join(";"));
            let selectors = Selectors::compile(&selectors)?;
            for selector in selectors.0 {
                css.declared_classes
                    .append(&mut selector.get_classes().clone());
                css.declared_attrs
                    .append(&mut selector.get_attribute_names().clone());
                let rule = CSSRule {
                    selector,
                    declarations: declarations.join(";"),
                    id: css.id,
                };
                css.rules.push(rule);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::element::button::Button;
    use crate::element::container::Container;
    use crate::element::{Element, ElementBackend};
    use crate::style::css_manager::CssManager;

    #[test]
    fn test_css_manager() {
        let mut manager = CssManager::new();
        manager.add(include_str!("../../tests/demo.css")).unwrap();
        assert_eq!(1, manager.stylesheets.len());
        let mut button = Element::create(Button::create);
        button.set_tag("button".to_string());
        let mut container = Element::create(Container::create);
        container.set_tag("container".to_string());
        let (containers_styles, _) = manager.match_styles(&container);
        let (button_styles, _) = manager.match_styles(&button);
        assert_eq!(1, containers_styles.len());
        assert_eq!(1, button_styles.len());
    }
}
