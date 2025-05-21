use crate::style::{ResolvedStyleProp, StylePropKey};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Styles {
    list: HashMap<StylePropKey, ResolvedStyleProp>,
}

impl Styles {
    pub fn from_map(list: HashMap<StylePropKey, ResolvedStyleProp>) -> Self {
        Self { list }
    }

    pub fn new() -> Self {
        Self {
            list: HashMap::new(),
        }
    }

    pub fn compute_changed_style<F: Fn(StylePropKey) -> ResolvedStyleProp>(
        &self,
        old_styles: &Styles,
        default_value: F,
    ) -> Vec<ResolvedStyleProp> {
        let mut changed_style_props = Vec::new();
        let mut keys = HashSet::new();
        for k in old_styles.list.keys() {
            keys.insert(k);
        }
        for k in self.list.keys() {
            keys.insert(k);
        }
        for k in keys {
            let old_value = old_styles.list.get(k);
            #[allow(suspicious_double_ref_op)]
            let new_value = match self.list.get(k) {
                Some(t) => t.clone().clone(),
                None => default_value(k.clone()),
            };
            if old_value != Some(&new_value) {
                changed_style_props.push(new_value);
            }
        }
        changed_style_props
    }
}
