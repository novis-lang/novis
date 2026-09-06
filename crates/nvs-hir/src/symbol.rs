//! The symbol table: one entry per declared class/interface/enum/type alias,
//! keyed by its fully-qualified [`QName`].

use std::collections::hash_map::Entry;

use nvs_diagnostics::Span;
use rustc_hash::FxHashMap;

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
    /// ([ADR 0015](/docs/adr/0015-no-name-aliasing.md) § 5).
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
/// ([ADR 0062](/docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
/// § 1) — see its
/// docs.
#[derive(Debug, Default)]
pub struct SymbolTable {
    by_name: FxHashMap<String, Symbol>,
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

    /// Whether a declaration exists under this fully-qualified name.
    #[must_use]
    pub fn contains(&self, qname: &QName) -> bool {
        self.by_name.contains_key(&qname.to_string())
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
