use crate::element::{Element};
use cssparser::{self, CowRcStr, ParseError, SourceLocation, ToCss};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::context::{MatchingMode, QuirksMode};
use selectors::parser::{Component, SelectorParseErrorKind};
use selectors::parser::{
    NonTSPseudoClass, Parser, Selector as GenericSelector, SelectorImpl, SelectorList,
};
use selectors::{self, matching, OpaqueElement};
use std::fmt;
use anyhow::{anyhow, Error};
use crate::some_or_return;

type LocalName = String;
type Namespace = String;

#[derive(Debug, Clone)]
pub struct DeftSelectors;

impl SelectorImpl for DeftSelectors {
    type ExtraMatchingData = ();
    type AttrValue = String;
    type Identifier = String;
    type ClassName = String;
    type PartName = String;
    type LocalName = String;
    type NamespaceUrl = String;
    type NamespacePrefix = String;
    type BorrowedNamespaceUrl = String;

    type BorrowedLocalName = String;
    type NonTSPseudoClass = PseudoClass;

    type PseudoElement = PseudoElement;
}

struct DeftParser;

impl<'i> Parser<'i> for DeftParser {
    type Impl = DeftSelectors;
    type Error = SelectorParseErrorKind<'i>;

    fn parse_non_ts_pseudo_class(
        &self,
        _location: SourceLocation,
        name: CowRcStr<'i>,
    ) -> Result<PseudoClass, ParseError<'i, SelectorParseErrorKind<'i>>> {
        use self::PseudoClass::*;
        if name.eq_ignore_ascii_case("focus") {
            Ok(Focus)
        } else if name.eq_ignore_ascii_case("hover") {
            Ok(Hover)
        } else {
            Ok(Unsupported(name.to_string()))
        }
    }

    fn parse_pseudo_element(&self, _location: SourceLocation, name: CowRcStr<'i>) -> Result<<Self::Impl as SelectorImpl>::PseudoElement, ParseError<'i, Self::Error>> {
        Ok(PseudoElement {
            name: name.to_string(),
        })
    }

}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum PseudoClass {
    Focus,
    Hover,
    Unsupported(String),
}

impl NonTSPseudoClass for PseudoClass {
    type Impl = DeftSelectors;

    fn is_active_or_hover(&self) -> bool {
        matches!(*self, PseudoClass::Hover)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(*self, PseudoClass::Hover | PseudoClass::Focus)
    }

    fn has_zero_specificity(&self) -> bool {
        false
    }
}

impl ToCss for PseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(match self.clone() {
            PseudoClass::Focus => ":focus",
            PseudoClass::Hover => ":hover",
            PseudoClass::Unsupported(_s) => ":unsupported",
        })
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct PseudoElement {
    pub name: String,
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&format!("::{}", self.name))
    }
}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = DeftSelectors;
}

impl selectors::Element for Element {
    type Impl = DeftSelectors;

    #[inline]
    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self)
    }

    #[inline]
    fn parent_element(&self) -> Option<Self> {
        self.get_parent()
    }
    #[inline]
    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }
    #[inline]
    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    #[inline]
    fn is_pseudo_element(&self) -> bool {
        false
    }
    #[inline]
    fn prev_sibling_element(&self) -> Option<Self> {
        //TODO fix
        None
    }
    #[inline]
    fn next_sibling_element(&self) -> Option<Self> {
        //TODO fix
        None
    }
    #[inline]
    fn is_html_element_in_html_document(&self) -> bool {
        true
    }
    #[inline]
    fn has_local_name(&self, name: &LocalName) -> bool {
        let backend = self.get_backend();
        backend.get_name().eq_ignore_ascii_case(name)
    }

    #[inline]
    fn has_namespace(&self, _namespace: &Namespace) -> bool {
        //TODO fix
        false
    }

    #[inline]
    fn is_same_type(&self, _other: &Self) -> bool {
        //TODO fixme
        false
    }
    #[inline]
    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&Namespace>,
        local_name: &LocalName,
        operation: &AttrSelectorOperation<&String>,
    ) -> bool {
        let attr = some_or_return!(self.attributes.get(local_name), false);
        match operation {
            AttrSelectorOperation::Exists => {
                true
            }
            AttrSelectorOperation::WithValue { expected_value, case_sensitivity, .. } => {
                match case_sensitivity {
                    CaseSensitivity::CaseSensitive => {
                        attr == *expected_value
                    }
                    CaseSensitivity::AsciiCaseInsensitive => {
                        attr.eq_ignore_ascii_case(expected_value)
                    }
                }
            }
        }
    }

    fn match_non_ts_pseudo_class<F>(
        &self,
        pseudo: &PseudoClass,
        _context: &mut matching::MatchingContext<DeftSelectors>,
        _flags_setter: &mut F,
    ) -> bool
    where
        F: FnMut(&Self, matching::ElementSelectorFlags),
    {
        match pseudo {
            PseudoClass::Focus => {
                self.is_focused()
            }
            PseudoClass::Hover => {
                self.hover
            }
            PseudoClass::Unsupported(_) => {
                false
            }
        }
    }

    fn match_pseudo_element(
        &self,
        _pseudo: &PseudoElement,
        _context: &mut matching::MatchingContext<DeftSelectors>,
    ) -> bool {
        true
    }

    #[inline]
    fn is_link(&self) -> bool {
        false
    }

    #[inline]
    fn is_html_slot_element(&self) -> bool {
        false
    }

    #[inline]
    fn has_id(&self, _id: &LocalName, _case_sensitivity: CaseSensitivity) -> bool {
        //TODO fix
        false
    }

    #[inline]
    fn has_class(&self, name: &LocalName, case_sensitivity: CaseSensitivity) -> bool {
        match case_sensitivity {
            CaseSensitivity::AsciiCaseInsensitive => {
                self.classes.iter().find(|it| it.eq_ignore_ascii_case(name)).is_some()
            }
            CaseSensitivity::CaseSensitive => {
                self.classes.contains(name)
            }
        }
    }

    #[inline]
    fn exported_part(&self, _: &LocalName) -> Option<LocalName> {
        None
    }

    #[inline]
    fn imported_part(&self, _: &LocalName) -> Option<LocalName> {
        None
    }

    #[inline]
    fn is_part(&self, _name: &LocalName) -> bool {
        false
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.get_children().is_empty()
    }

    #[inline]
    fn is_root(&self) -> bool {
        self.get_parent().is_none()
    }
}

pub struct Selectors(pub Vec<Selector>);

#[derive(Clone)]
pub struct Selector {
    selector: GenericSelector<DeftSelectors>,
    class_names: Vec<String>,
    attribute_names: Vec<String>
}

impl Selectors {
    pub fn compile(s: &str) -> Result<Selectors, Error> {
        let mut input = cssparser::ParserInput::new(s);
        match SelectorList::parse(&DeftParser, &mut cssparser::Parser::new(&mut input)) {
            Ok(list) => Ok(
                Selectors(
                    list.0.into_iter().map(|s| Selector::new(s)).collect()
                )
            ),
            Err(e) => Err(anyhow!("failed to parse css: {:?}", e)),
        }
    }

    pub fn matches(&self, element: &Element) -> bool {
        self.0.iter().any(|s| s.matches(element))
    }

    pub fn selectors(&self) -> Vec<Selector> {
        self.0.clone()
    }

}

impl Selector {

    pub fn new(selector: GenericSelector<DeftSelectors>) -> Self {
        let mut list = Vec::new();
        let mut attribute_names = Vec::new();
        for e in selector.iter_raw_match_order() {
            match e {
                Component::Class(c) => {
                    list.push(c.clone());
                }
                Component::AttributeInNoNamespaceExists { local_name, .. } => {
                    attribute_names.push(local_name.clone());
                }
                Component::AttributeInNoNamespace { local_name, .. } => {
                    attribute_names.push(local_name.clone());
                }
                Component::AttributeOther(a) => {
                    attribute_names.push(a.local_name.clone());
                }
                Component::Combinator(_) => {}
                Component::ExplicitAnyNamespace => {}
                Component::ExplicitNoNamespace => {}
                Component::DefaultNamespace(_) => {}
                Component::Namespace(_, _) => {}
                Component::ExplicitUniversalType => {}
                Component::LocalName(_) => {}
                Component::ID(_) => {}
                Component::Negation(_) => {}
                Component::FirstChild => {}
                Component::LastChild => {}
                Component::OnlyChild => {}
                Component::Root => {}
                Component::Empty => {}
                Component::Scope => {}
                Component::NthChild(_, _) => {}
                Component::NthLastChild(_, _) => {}
                Component::NthOfType(_, _) => {}
                Component::NthLastOfType(_, _) => {}
                Component::FirstOfType => {}
                Component::LastOfType => {}
                Component::OnlyOfType => {}
                Component::NonTSPseudoClass(_) => {}
                Component::Slotted(_) => {}
                Component::Part(_) => {}
                Component::Host(_) => {}
                Component::PseudoElement(_) => {}
            }
        }
        Self {
            selector,
            class_names: list,
            attribute_names,
        }
    }

    pub fn matches(&self, element: &Element) -> bool {
        let mode = if self.pseudo_element().is_some() {
            MatchingMode::ForStatelessPseudoElement
        } else {
            MatchingMode::Normal
        };
        let mut context = matching::MatchingContext::new(
            mode,
            None,
            None,
            QuirksMode::NoQuirks,
        );
        matching::matches_selector(&self.selector, 0, None, element, &mut context, &mut |_, _| {})
    }

    pub fn pseudo_element(&self) -> Option<&PseudoElement> {
        self.selector.pseudo_element()
    }

    pub fn get_classes(&self) -> &Vec<String> {
        &self.class_names
    }

    pub fn get_attribute_names(&self) -> &Vec<String> {
        &self.attribute_names
    }

    pub fn specificity(&self) -> u32 {
        self.selector.specificity()
    }

}

#[cfg(test)]
pub mod tests {
    use crate::element::button::Button;
    use crate::element::container::Container;
    use crate::element::{Element, ElementBackend};
    use crate::style::select::Selectors;

    #[test]
    fn test_select() {
        let btn_selector = Selectors::compile("button").unwrap();
        let container_selector = Selectors::compile("container").unwrap();
        let button = Element::create(Button::create);
        let container = Element::create(Container::create);
        assert!(btn_selector.matches(&button));
        assert!(container_selector.matches(&container));
        assert!(!btn_selector.matches(&container));
        assert!(!container_selector.matches(&button));
    }

    #[test]
    fn test_class() {
        let selectors = Selectors::compile("p.a .b button").unwrap();
        let selector = selectors.0.get(0).unwrap();
        let classes = selector.get_classes();
        assert_eq!(classes, &vec!["b", "a"]);
    }
}


