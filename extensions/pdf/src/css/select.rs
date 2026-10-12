//! Selectors: the `selectors` crate parses and matches them over the document's [`Dom`].
//!
//! The subset's selectors are the ones the crate parses with nothing turned on: type, class, id
//! and attribute selectors, the four combinators, the structural pseudo-classes and `:not()`.
//! The one pseudo-element pair is `::before` and `::after`. Any other pseudo-class or
//! pseudo-element, and `:is()`, `:where()` and `:has()`, does not parse, so its rule is dropped
//! and reported.
//!
//! [`Tree`] holds each element's parent and neighbouring elements, computed once, so a sibling
//! or `:nth-child()` step is a lookup.

use std::fmt;

use cssparser::{CowRcStr, ParseError, Parser as CssParser, ToCss};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::context::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches,
};
use selectors::matching::{ElementSelectorFlags, matches_selector};
use selectors::parser::{ParseRelative, Selector, SelectorParseErrorKind};
use selectors::{OpaqueElement, SelectorList};

use crate::parse::{Dom, Kind, Space};

const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// An identifier, a name or a namespace in a selector.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ident(pub String);

impl From<&str> for Ident {
    fn from(text: &str) -> Self {
        Ident(text.to_owned())
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ToCss for Ident {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        cssparser::serialize_identifier(&self.0, dest)
    }
}

impl PrecomputedHash for Ident {
    fn precomputed_hash(&self) -> u32 {
        // FNV-1a: the crate's Bloom filter is the only reader, and it is never given one.
        self.0.bytes().fold(0x811c_9dc5_u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        })
    }
}

/// No pseudo-class beyond the ones the crate knows is in the subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoClass {}

impl ToCss for PseudoClass {
    fn to_css<W: fmt::Write>(&self, _dest: &mut W) -> fmt::Result {
        match *self {}
    }
}

impl selectors::parser::NonTSPseudoClass for PseudoClass {
    fn is_active_or_hover(&self) -> bool {
        match *self {}
    }

    fn is_user_action_state(&self) -> bool {
        match *self {}
    }
}

/// `::before` or `::after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoElement {
    Before,
    After,
}

impl ToCss for PseudoElement {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(match self {
            PseudoElement::Before => "::before",
            PseudoElement::After => "::after",
        })
    }
}

impl selectors::parser::PseudoElement for PseudoElement {
    fn is_before_or_after(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct Impl;

impl selectors::SelectorImpl for Impl {
    type ExtraMatchingData<'a> = ();
    type AttrValue = Ident;
    type Identifier = Ident;
    type LocalName = Ident;
    type NamespaceUrl = Ident;
    type NamespacePrefix = Ident;
    type BorrowedNamespaceUrl = Ident;
    type BorrowedLocalName = Ident;
    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;
}

struct SubsetParser;

impl<'i> selectors::Parser<'i> for SubsetParser {
    type Impl = Impl;
    type Error = SelectorParseErrorKind;

    fn parse_pseudo_element(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<PseudoElement, ParseError<Self::Error>> {
        if name.eq_ignore_ascii_case("before") {
            Ok(PseudoElement::Before)
        } else if name.eq_ignore_ascii_case("after") {
            Ok(PseudoElement::After)
        } else {
            Err(ParseError::custom(
                SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }
}

/// The selector list `text`, or `None` when any selector in it is outside the subset.
pub fn parse(text: &str) -> Option<SelectorList<Impl>> {
    let mut parser = CssParser::new(text);
    SelectorList::parse(&SubsetParser, &mut parser, ParseRelative::No).ok()
}

/// Each element's parent, neighbouring elements and first element child.
pub struct Tree<'a> {
    pub dom: &'a Dom,
    parent: Vec<Option<usize>>,
    prev: Vec<Option<usize>>,
    next: Vec<Option<usize>>,
    first_child: Vec<Option<usize>>,
}

impl<'a> Tree<'a> {
    pub fn new(dom: &'a Dom) -> Self {
        let n = dom.nodes.len();
        let mut tree = Tree {
            dom,
            parent: vec![None; n],
            prev: vec![None; n],
            next: vec![None; n],
            first_child: vec![None; n],
        };
        for (at, node) in dom.nodes.iter().enumerate() {
            let mut last: Option<usize> = None;
            for &child in &node.children {
                if !matches!(dom.nodes[child].kind, Kind::Element { .. }) {
                    continue;
                }
                if matches!(node.kind, Kind::Element { .. }) {
                    tree.parent[child] = Some(at);
                }
                match last {
                    Some(prev) => {
                        tree.prev[child] = Some(prev);
                        tree.next[prev] = Some(child);
                    }
                    None => tree.first_child[at] = Some(child),
                }
                last = Some(child);
            }
        }
        tree
    }

    /// Whether `selector` matches the element at `at`. A selector ending in a pseudo-element
    /// matches the element it is generated for, and only when `pseudo` names that one.
    pub fn matches(
        &self,
        selector: &Selector<Impl>,
        at: usize,
        pseudo: Option<PseudoElement>,
        caches: &mut SelectorCaches,
    ) -> bool {
        if selector.pseudo_element().copied() != pseudo {
            return false;
        }
        let mode = match pseudo {
            Some(_) => MatchingMode::ForStatelessPseudoElement,
            None => MatchingMode::Normal,
        };
        let mut context = MatchingContext::new(
            mode,
            None,
            caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        matches_selector(selector, 0, None, &El { tree: self, at }, &mut context)
    }
}

#[derive(Clone, Copy)]
struct El<'t, 'a> {
    tree: &'t Tree<'a>,
    at: usize,
}

impl fmt::Debug for El<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "element {}", self.at)
    }
}

impl<'t, 'a> El<'t, 'a> {
    fn with(&self, at: Option<usize>) -> Option<Self> {
        at.map(|at| El {
            tree: self.tree,
            at,
        })
    }

    fn node(&self) -> &'a crate::parse::Node {
        &self.tree.dom.nodes[self.at]
    }

    fn space(&self) -> Space {
        match self.node().kind {
            Kind::Element { space, .. } => space,
            _ => Space::Other,
        }
    }
}

impl selectors::Element for El<'_, '_> {
    type Impl = Impl;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.node())
    }

    fn parent_element(&self) -> Option<Self> {
        self.with(self.tree.parent[self.at])
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.with(self.tree.prev[self.at])
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.with(self.tree.next[self.at])
    }

    fn first_element_child(&self) -> Option<Self> {
        self.with(self.tree.first_child[self.at])
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.space() == Space::Html
    }

    fn has_local_name(&self, local_name: &Ident) -> bool {
        match &self.node().kind {
            Kind::Element { name, .. } => *name == local_name.0,
            _ => false,
        }
    }

    fn has_namespace(&self, ns: &Ident) -> bool {
        match self.space() {
            Space::Html => ns.0 == HTML_NS,
            Space::Svg => ns.0 == SVG_NS,
            Space::Other => false,
        }
    }

    fn is_same_type(&self, other: &Self) -> bool {
        match (&self.node().kind, &other.node().kind) {
            (
                Kind::Element {
                    name: a, space: x, ..
                },
                Kind::Element {
                    name: b, space: y, ..
                },
            ) => a == b && x == y,
            _ => false,
        }
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&Ident>,
        local_name: &Ident,
        operation: &AttrSelectorOperation<&Ident>,
    ) -> bool {
        self.node().attrs().iter().any(|attr| {
            let in_ns = match ns {
                NamespaceConstraint::Any => true,
                NamespaceConstraint::Specific(url) => url.0.is_empty() != attr.xlink,
            };
            in_ns && attr.name == local_name.0 && operation.eval_str(&attr.value)
        })
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &PseudoClass,
        _context: &mut MatchingContext<Impl>,
    ) -> bool {
        match *pc {}
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _context: &mut MatchingContext<Impl>,
    ) -> bool {
        true
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        false
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(&self, id: &Ident, case_sensitivity: CaseSensitivity) -> bool {
        self.node()
            .attr("id")
            .is_some_and(|value| case_sensitivity.eq(value.as_bytes(), id.0.as_bytes()))
    }

    fn has_class(&self, name: &Ident, case_sensitivity: CaseSensitivity) -> bool {
        self.node().attr("class").is_some_and(|value| {
            value
                .split_ascii_whitespace()
                .any(|class| case_sensitivity.eq(class.as_bytes(), name.0.as_bytes()))
        })
    }

    fn has_custom_state(&self, _name: &Ident) -> bool {
        false
    }

    fn imported_part(&self, _name: &Ident) -> Option<Ident> {
        None
    }

    fn is_part(&self, _name: &Ident) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.node()
            .children
            .iter()
            .all(|&child| match &self.tree.dom.nodes[child].kind {
                Kind::Element { .. } => false,
                Kind::Text(text) => text.is_empty(),
                _ => true,
            })
    }

    fn is_root(&self) -> bool {
        self.tree.parent[self.at].is_none()
            && self
                .node()
                .parent
                .is_some_and(|p| matches!(self.tree.dom.nodes[p].kind, Kind::Document))
    }

    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        false
    }
}
