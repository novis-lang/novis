//! The symbol table: one entry per declared class/interface/enum/type alias,
//! keyed by its fully-qualified [`QName`].
//!
//! A class a loaded extension declares has no source text, so it has no
//! [`Symbol`] and no span: [`SymbolTable::declare_extension`] records its name
//! apart, and [`SymbolTable::contains`] answers for both. Every resolver that
//! asks "is this name declared" then reads an extension class as declared,
//! with no extension branch of its own, and every reader that needs a span —
//! a duplicate's secondary label, the editor's go-to-definition — finds none
//! to misuse.

use std::collections::hash_map::Entry;

use nvs_diagnostics::Span;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::qname::QName;

/// What kind of declaration a [`Symbol`] names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymbolKind {
    /// A `class` declaration.
    Class,
    /// An `interface` declaration.
    Interface,
    /// An `enum` declaration (`rule:enums/closed-integer-type`).
    Enum,
    /// A `type Name = TypeExpr;` declaration
    /// (`rule:types/type-alias`).
    TypeAlias,
}

impl SymbolKind {
    /// A lower-case noun for this kind, for diagnostic messages.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Class => "a class",
            Self::Interface => "an interface",
            Self::Enum => "an enum",
            Self::TypeAlias => "a type alias",
        }
    }
}

/// A class a loaded extension declares, as name resolution sees it: its name
/// and the names of its members. `nvs_types::ext_lib` builds one from each
/// manifest of the loaded set, and its types are that module's business.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionClass {
    /// The class's fully-qualified name.
    pub name: QName,
    /// Its `static` methods.
    pub methods: Vec<String>,
    /// Its `const` members.
    pub consts: Vec<String>,
}

/// One declared class, interface, trait, enum or `type` alias.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// What was declared.
    pub kind: SymbolKind,
    /// Its fully-qualified name.
    pub qname: QName,
    /// Where the declared name itself (not the whole declaration) was
    /// written — the span a duplicate-declaration diagnostic points at.
    pub decl_span: Span,
}

/// Every type-level declaration collected from one compilation, keyed by its
/// fully-qualified name.
///
/// Keying is case-sensitive, matching [`QName`]'s own decided behaviour
/// (`rule:classes/names-resolve-case-sensitively`) — see its
/// docs.
#[derive(Debug, Default)]
pub struct SymbolTable {
    by_name: FxHashMap<String, Symbol>,
    /// The classes the loaded extension set declares, which have no source.
    extensions: FxHashSet<String>,
}

impl SymbolTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a declaration.
    ///
    /// If `symbol.qname` was already taken, the table is left unchanged (the
    /// first declaration always wins) and the earlier [`Symbol`] is returned
    /// so the caller can report `E_DUPLICATE_DECLARATION` naming it.
    pub fn declare(&mut self, symbol: Symbol) -> Option<&Symbol> {
        match self.by_name.entry(symbol.qname.to_string()) {
            Entry::Occupied(entry) => Some(entry.into_mut()),
            Entry::Vacant(entry) => {
                entry.insert(symbol);
                None
            }
        }
    }

    /// Looks up a declaration by its fully-qualified name.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&Symbol> {
        self.by_name.get(&qname.to_string())
    }

    /// Records a class a loaded extension declares. Declared before any source
    /// file is collected, so a source declaration of the same name is the one
    /// refused ([`Self::is_extension`]).
    pub fn declare_extension(&mut self, qname: &QName) {
        self.extensions.insert(qname.to_string());
    }

    /// Whether a loaded extension declares `qname`.
    #[must_use]
    pub fn is_extension(&self, qname: &QName) -> bool {
        self.extensions.contains(&qname.to_string())
    }

    /// Whether a declaration exists under this fully-qualified name, in source
    /// or in a loaded extension.
    #[must_use]
    pub fn contains(&self, qname: &QName) -> bool {
        let name = qname.to_string();
        self.by_name.contains_key(&name) || self.extensions.contains(&name)
    }

    /// Every declared symbol, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.by_name.values()
    }

    /// How many symbols are declared.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether nothing has been declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;

    use super::*;

    fn sym(name: &str, kind: SymbolKind) -> Symbol {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", "x");
        Symbol {
            kind,
            qname: QName::parse(name),
            decl_span: Span::new(file, 0, 1),
        }
    }

    #[test]
    fn a_fresh_declaration_is_accepted() {
        let mut table = SymbolTable::new();
        assert!(table.declare(sym("Foo", SymbolKind::Class)).is_none());
        assert_eq!(table.len(), 1);
        assert!(table.contains(&QName::parse("Foo")));
    }

    #[test]
    fn a_second_declaration_of_the_same_name_is_rejected_and_the_first_kept() {
        let mut table = SymbolTable::new();
        table.declare(sym("Foo", SymbolKind::Class));
        let existing = table
            .declare(sym("Foo", SymbolKind::Interface))
            .expect("duplicate should be reported");
        assert_eq!(existing.kind, SymbolKind::Class, "first declaration wins");
        assert_eq!(table.len(), 1);
        assert_eq!(
            table.get(&QName::parse("Foo")).unwrap().kind,
            SymbolKind::Class
        );
    }

    #[test]
    fn different_namespaces_do_not_collide() {
        let mut table = SymbolTable::new();
        table.declare(sym("App\\User", SymbolKind::Class));
        assert!(
            table
                .declare(sym("Admin\\User", SymbolKind::Class))
                .is_none()
        );
        assert_eq!(table.len(), 2);
    }
}
