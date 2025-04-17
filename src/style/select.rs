use crate::element::button::Button;
use crate::element::container::Container;
use crate::element::{Element, ElementBackend, ElementData};
use cssparser::{self, CowRcStr, ParseError, SourceLocation, ToCss};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::context::{MatchingMode, QuirksMode};
use selectors::parser::SelectorParseErrorKind;
use selectors::parser::{
    NonTSPseudoClass, Parser, Selector as GenericSelector, SelectorImpl, SelectorList,
};
use selectors::{self, matching, OpaqueElement};
use std::fmt;
use std::fmt::Write;
use anyhow::{anyhow, Error};

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
        location: SourceLocation,
        name: CowRcStr<'i>,
    ) -> Result<PseudoClass, ParseError<'i, SelectorParseErrorKind<'i>>> {
        use self::PseudoClass::*;
        if name.eq_ignore_ascii_case("focus") {
            Ok(Focus)
        } else if name.eq_ignore_ascii_case("hover") {
            Ok(Hover)
        } else {
            Err(
                location.new_custom_error(SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                    name,
                )),
            )
        }
    }

    fn parse_pseudo_element(&self, location: SourceLocation, name: CowRcStr<'i>) -> Result<<Self::Impl as SelectorImpl>::PseudoElement, ParseError<'i, Self::Error>> {
        Ok(PseudoElement {
            name: name.to_string(),
        })
    }

}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum PseudoClass {
    Focus,
    Hover,
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
        dest.write_str(match *self {
            PseudoClass::Focus => ":focus",
            PseudoClass::Hover => ":hover",
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
        dest.write_str(&format!("::{}", self.name));
        Ok(())
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
    fn has_namespace(&self, namespace: &Namespace) -> bool {
        //TODO fix
        false
    }

    #[inline]
    fn is_same_type(&self, other: &Self) -> bool {
        //TODO fixme
        false
    }
    #[inline]
    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&Namespace>,
        local_name: &LocalName,
        operation: &AttrSelectorOperation<&String>,
    ) -> bool {
        //TODO fix
        false
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
    fn has_id(&self, id: &LocalName, case_sensitivity: CaseSensitivity) -> bool {
        //TODO fix
        false
    }

    #[inline]
    fn has_class(&self, name: &LocalName, case_sensitivity: CaseSensitivity) -> bool {
        //TODO fix
        false
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
pub struct Selector(GenericSelector<DeftSelectors>);

impl Selectors {
    pub fn compile(s: &str) -> Result<Selectors, Error> {
        let mut input = cssparser::ParserInput::new(s);
        match SelectorList::parse(&DeftParser, &mut cssparser::Parser::new(&mut input)) {
            Ok(list) => Ok(Selectors(list.0.into_iter().map(Selector).collect())),
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
        matching::matches_selector(&self.0, 0, None, element, &mut context, &mut |_, _| {})
    }

    pub fn pseudo_element(&self) -> Option<&PseudoElement> {
        self.0.pseudo_element()
    }
}

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
