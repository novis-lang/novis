//! The document's HTML, parsed into a tree the rest of the engine walks.
//!
//! The parser is the repository's patched `html5ever` 0.39.0 (`vendor/html5ever`), the crate
//! `Core\Html` parses with, run with the same cap on the stack of open elements
//! (`rule:core-classes/html-parsing`): a start tag that arrives with [`OPEN_ELEMENTS_CAP`]
//! elements open first closes the current one, so a document nested deeper than the cap is
//! flattened into siblings and never fails. Scripting is on, as in `Core\Html`, so a
//! `<noscript>`'s content is text and never elements.
//!
//! The tree is an arena of [`Node`]s indexed from the document at 0. Comments, processing
//! instructions and the doctype are not kept: nothing a render draws or loads is in them. A
//! `<template>`'s contents are its own children, which the layout never enters.
//!
//! Memory: one [`Node`] per element and per run of text, each with its attributes, freed when the
//! render returns.

use std::borrow::Cow;
use std::cell::{Ref, RefCell};

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName, ns};

/// How many elements may be open at once while a document is parsed: the cap
/// `crates/nvs-stdlib/src/html.rs` gives `Core\Html`, and the depth Chromium, Firefox and
/// WebKit cap a tree at.
pub const OPEN_ELEMENTS_CAP: usize = 512;

/// The namespace an element is in. Only the two a render treats differently are told apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Html,
    Svg,
    Other,
}

/// One attribute: its local name, whether it is in the XLink namespace, and its value.
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub xlink: bool,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Document,
    /// An element's lower-case local name and its namespace.
    Element {
        name: String,
        space: Space,
        attrs: Vec<Attr>,
    },
    Text(String),
    /// A comment or a processing instruction: kept in the arena so the tree builder's handles
    /// stay valid, never attached.
    Ignored,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: Kind,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// The name the tree builder asks for, for elements only.
    qual: Option<QualName>,
}

impl Node {
    fn new(kind: Kind) -> Self {
        Node {
            kind,
            parent: None,
            children: Vec::new(),
            qual: None,
        }
    }

    /// The local name of an element in `space`, or `None` for anything else.
    pub fn element(&self, space: Space) -> Option<&str> {
        match &self.kind {
            Kind::Element { name, space: s, .. } if *s == space => Some(name),
            _ => None,
        }
    }

    /// The value of the attribute `name` outside the XLink namespace.
    pub fn attr(&self, name: &str) -> Option<&str> {
        match &self.kind {
            Kind::Element { attrs, .. } => attrs
                .iter()
                .find(|attr| !attr.xlink && attr.name == name)
                .map(|attr| attr.value.as_str()),
            _ => None,
        }
    }

    pub fn attrs(&self) -> &[Attr] {
        match &self.kind {
            Kind::Element { attrs, .. } => attrs,
            _ => &[],
        }
    }
}

/// A parsed document: the arena, with the document node at 0.
#[derive(Debug)]
pub struct Dom {
    pub nodes: Vec<Node>,
}

impl Dom {
    /// The nodes under `root` in document order, `root` first. The walk keeps its own stack, so
    /// a deep tree costs heap and never the call stack.
    pub fn preorder(&self, root: usize) -> Vec<usize> {
        let mut order = Vec::new();
        let mut stack = vec![root];
        while let Some(at) = stack.pop() {
            order.push(at);
            stack.extend(self.nodes[at].children.iter().rev());
        }
        order
    }

    /// The text of `at`'s descendants, concatenated in document order.
    pub fn text(&self, at: usize) -> String {
        let mut text = String::new();
        for node in self.preorder(at) {
            if let Kind::Text(t) = &self.nodes[node].kind {
                text.push_str(t);
            }
        }
        text
    }
}

/// `html`, parsed by the WHATWG algorithm under [`OPEN_ELEMENTS_CAP`].
pub fn parse(html: &str) -> Dom {
    let mut opts = html5ever::ParseOpts::default();
    opts.tree_builder.max_open_elements = Some(OPEN_ELEMENTS_CAP);
    html5ever::parse_document(Sink::default(), opts).one(html)
}

struct Sink {
    nodes: RefCell<Vec<Node>>,
}

impl Default for Sink {
    fn default() -> Self {
        Sink {
            nodes: RefCell::new(vec![Node::new(Kind::Document)]),
        }
    }
}

impl Sink {
    fn push(&self, node: Node) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(node);
        nodes.len() - 1
    }

    /// `target`'s parent and its place among the parent's children. The search runs from the
    /// last child, where foster parenting asks, so a table with many misplaced nodes stays linear.
    fn locate(&self, target: usize) -> Option<(usize, usize)> {
        let nodes = self.nodes.borrow();
        let parent = nodes[target].parent?;
        let at = nodes[parent].children.iter().rposition(|&n| n == target)?;
        Some((parent, at))
    }

    fn detach(&self, target: usize) {
        let Some((parent, at)) = self.locate(target) else {
            return;
        };
        let mut nodes = self.nodes.borrow_mut();
        nodes[parent].children.remove(at);
        nodes[target].parent = None;
    }

    fn attach(&self, parent: usize, child: usize) {
        self.detach(child);
        let mut nodes = self.nodes.borrow_mut();
        nodes[child].parent = Some(parent);
        nodes[parent].children.push(child);
    }

    fn insert(&self, parent: usize, at: usize, child: usize) {
        let mut nodes = self.nodes.borrow_mut();
        nodes[child].parent = Some(parent);
        nodes[parent].children.insert(at, child);
    }

    /// Appends `text` to the node at `at` if it is text, and says whether it did, so adjacent
    /// text is one node.
    fn merged(&self, at: Option<usize>, text: &str) -> bool {
        let Some(at) = at else {
            return false;
        };
        match &mut self.nodes.borrow_mut()[at].kind {
            Kind::Text(existing) => {
                existing.push_str(text);
                true
            }
            _ => false,
        }
    }
}

fn space(name: &QualName) -> Space {
    if name.ns == ns!(html) {
        Space::Html
    } else if name.ns == ns!(svg) {
        Space::Svg
    } else {
        Space::Other
    }
}

impl TreeSink for Sink {
    type Handle = usize;
    type Output = Dom;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Dom {
        Dom {
            nodes: self.nodes.into_inner(),
        }
    }

    /// Discarded: the algorithm already says how to recover from each, and a render draws the
    /// recovered tree.
    fn parse_error(&self, _why: Cow<'static, str>) {}

    fn get_document(&self) -> usize {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a usize) -> Ref<'a, QualName> {
        Ref::map(self.nodes.borrow(), |nodes| {
            nodes[*target]
                .qual
                .as_ref()
                .expect("the tree builder asks only an element for its name")
        })
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, _: ElementFlags) -> usize {
        let attrs = attrs
            .into_iter()
            .map(|attr| Attr {
                name: attr.name.local.to_string(),
                xlink: attr.name.ns == ns!(xlink),
                value: attr.value.to_string(),
            })
            .collect();
        let mut node = Node::new(Kind::Element {
            name: name.local.to_string(),
            space: space(&name),
            attrs,
        });
        node.qual = Some(name);
        self.push(node)
    }

    fn create_comment(&self, _text: StrTendril) -> usize {
        self.push(Node::new(Kind::Ignored))
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> usize {
        self.push(Node::new(Kind::Ignored))
    }

    fn append(&self, parent: &usize, child: NodeOrText<usize>) {
        match child {
            NodeOrText::AppendNode(node) => self.attach(*parent, node),
            NodeOrText::AppendText(text) => {
                let last = self.nodes.borrow()[*parent].children.last().copied();
                if !self.merged(last, &text) {
                    let node = self.push(Node::new(Kind::Text(text.to_string())));
                    self.attach(*parent, node);
                }
            }
        }
    }

    fn append_before_sibling(&self, sibling: &usize, new_node: NodeOrText<usize>) {
        match new_node {
            NodeOrText::AppendNode(node) => {
                // Detached first: if `node`'s old parent is this one, removing it moves the
                // sibling's position.
                self.detach(node);
                if let Some((parent, at)) = self.locate(*sibling) {
                    self.insert(parent, at, node);
                }
            }
            NodeOrText::AppendText(text) => {
                let Some((parent, at)) = self.locate(*sibling) else {
                    return;
                };
                let before = at
                    .checked_sub(1)
                    .map(|before| self.nodes.borrow()[parent].children[before]);
                if !self.merged(before, &text) {
                    let node = self.push(Node::new(Kind::Text(text.to_string())));
                    self.insert(parent, at, node);
                }
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &usize,
        prev_element: &usize,
        child: NodeOrText<usize>,
    ) {
        if self.nodes.borrow()[*element].parent.is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(&self, _: StrTendril, _: StrTendril, _: StrTendril) {}

    fn get_template_contents(&self, target: &usize) -> usize {
        *target
    }

    fn same_node(&self, x: &usize, y: &usize) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn add_attrs_if_missing(&self, target: &usize, attrs: Vec<Attribute>) {
        let mut nodes = self.nodes.borrow_mut();
        if let Kind::Element { attrs: had, .. } = &mut nodes[*target].kind {
            for attr in attrs {
                let xlink = attr.name.ns == ns!(xlink);
                let name = attr.name.local.to_string();
                if !had.iter().any(|a| a.xlink == xlink && a.name == name) {
                    had.push(Attr {
                        name,
                        xlink,
                        value: attr.value.to_string(),
                    });
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &usize) {
        self.detach(*target);
    }

    fn reparent_children(&self, node: &usize, new_parent: &usize) {
        let moved = std::mem::take(&mut self.nodes.borrow_mut()[*node].children);
        let mut nodes = self.nodes.borrow_mut();
        for &child in &moved {
            nodes[child].parent = Some(*new_parent);
        }
        nodes[*new_parent].children.extend(moved);
    }

    /// Refused, so a `<template shadowrootmode>` stays a template, which the layout never enters.
    fn allow_declarative_shadow_roots(&self, _intended_parent: &usize) -> bool {
        false
    }
}
