//! Which static types may hold a revocable object
//! (`rule:classes/revocable-objects`).
//!
//! A use of a revoked object throws, and the check that throws is compiled only
//! where the static type could hold one. This module answers that question
//! once per program, so the IR asks a set rather than walking the graph at
//! each access.
//!
//! # Why the answer is a set of names
//!
//! Only a class with no parent may implement `Revocable`, and every subclass
//! inherits it. So a type may hold a revocable object exactly when some
//! concrete revocable class reaches it, through `extends` or `implements`.
//! The set is every name a concrete revocable class reaches, itself included.
//! A class type that is not in it never holds a revoked object: its subclasses
//! reach it, and none of them is revocable, because their root is not. An
//! interface, a `Core` contract a user class implements, and a revocable class
//! are in it when a concrete revocable class reaches them, which needs no
//! lookup of the declaration's kind.
//!
//! `mixed`, `object` and a type variable may hold any object, so they need the
//! check whenever any revocable class exists. A program with no revocable class
//! compiles no check at all.

use nvs_hir::{ClassGraph, QName, implementors};
use rustc_hash::FxHashSet;

use crate::ty::{Ty, TypeId, TypeInterner};

/// Every name a concrete revocable class reaches. Built once per program by
/// [`Self::of`]. Memory: one name per class and interface above a revocable
/// class, held for one compile.
#[derive(Clone, Debug, Default)]
pub struct RevocableTypes {
    reached: FxHashSet<QName>,
}

impl RevocableTypes {
    /// The set for the program whose links are `graph`.
    #[must_use]
    pub fn of(graph: &ClassGraph) -> Self {
        let revocable = QName::parse(nvs_hir::interfaces::REVOCABLE);
        let mut reached = FxHashSet::default();
        let mut stack = implementors(&revocable, graph);
        while let Some(name) = stack.pop() {
            if let Some(links) = graph.get(&name) {
                stack.extend(links.extends.iter().cloned());
                stack.extend(links.implements.iter().cloned());
            }
            reached.insert(name);
        }
        Self { reached }
    }

    /// Whether `name`, a class or an interface, is reached by a revocable
    /// class. An access whose member resolved to `name` then needs the check.
    /// A class outside the set has no revocable subclass, so its accesses
    /// need none.
    #[must_use]
    pub fn contains(&self, name: &QName) -> bool {
        self.reached.contains(name)
    }

    /// Whether the program has no revocable class.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.reached.is_empty()
    }

    /// Every name in the set, in no order.
    pub fn names(&self) -> impl Iterator<Item = &QName> {
        self.reached.iter()
    }

    /// Whether a value of static type `ty` may be a revocable object, so a
    /// property access, a method call or a `clone` through it needs the check.
    ///
    /// A union or a nullable type needs it when one of its members does.
    #[must_use]
    pub fn may_be_revocable(&self, ty: TypeId, interner: &TypeInterner) -> bool {
        if self.reached.is_empty() {
            return false;
        }
        match interner.get(ty) {
            Ty::Mixed | Ty::Object | Ty::TypeVar(_) => true,
            Ty::Class(name, _) => self.reached.contains(name),
            Ty::Union(members) | Ty::Intersection(members) => members
                .iter()
                .any(|member| self.may_be_revocable(*member, interner)),
            _ => false,
        }
    }
}
