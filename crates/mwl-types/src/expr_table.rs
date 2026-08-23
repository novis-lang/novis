//! The typed-expression table — the architecture decision `mwl-ir` widening
//! past scalars needed before it could lower a call, `new`, or a member
//! access: see `NEXT_SESSION_PROMPT.md`'s history for the two options this
//! was weighed against (`mwl-ir` depending on `mwl-types` and duplicating its
//! resolution logic, versus this crate publishing a persisted result
//! `mwl-ir` reads back) and why the second was chosen.
//!
//! # What this is, and why it's shaped this way
//!
//! [`expr::infer`](crate::expr::infer) already resolves a call's or `new`'s
//! target — walking [`crate::signatures::SignatureTable`] and
//! [`mwl_hir::ClassGraph`] to find the actual declaring class and its
//! [`crate::signatures::MethodSig`] — but until now threw that resolution
//! away the moment it returned a [`crate::ty::TypeId`]. [`ExprTypeTable`] is
//! where [`crate::check::check_program`] persists it instead: one
//! [`ExprInfo`] entry per expression whose *resolved identity* (not just its
//! type) a later pass needs and cannot cheaply re-derive from the AST alone.
//!
//! This is a narrow, deliberately incomplete table — it exists to answer
//! exactly the questions `mwl-ir`'s widening needed answered, not to become a
//! second, general-purpose typed-AST. [`ExprInfo::Property`] is the first
//! instance of the pattern this module's docs originally predicted: widening
//! `mwl-ir` further (array access, `match`/ternary result identity, ...) is
//! expected to keep growing [`ExprInfo`] with one new variant per question,
//! each populated at its own `expr::infer`/`check_property_access`-style call
//! site — not to replace this shape. See the crate's own known-gaps list for
//! exactly which expression shapes have no entry here yet.
//!
//! # Why a lookup is keyed by [`mwl_diagnostics::Span`], not assignment order
//!
//! [`ExprId`] is a real, stable id — assigned once, in the order
//! [`crate::check::check_program`]'s single left-to-right AST walk first
//! records each entry, and never reused. But a consumer in another crate
//! (`mwl-ir`) cannot re-derive *that* order for itself: its own lowering walk
//! is a second, independently-shaped traversal of the same AST (for example,
//! it may skip a dynamic member-name sub-expression this crate's checker
//! still visits), so "the Nth entry this crate recorded" and "the Nth
//! call-shaped node `mwl-ir` visits" are not guaranteed to line up. The one
//! thing both crates *do* agree on without coordinating their walk order is
//! the source [`mwl_diagnostics::Span`] each AST node already carries — so
//! [`ExprTypeTable::lookup`] takes a span, not an id, and [`ExprId`] itself is
//! never constructed outside this module. This mirrors why `mwl-ir`'s own
//! [`ids`](../mwl_ir/ids/index.html) module numbers `StmtId`/`EdgeId` from a
//! *single* deterministic walk rather than letting two passes agree on
//! numbering independently — the same hazard, resolved the other way because
//! here the two walks unavoidably live in two different crates.
//!
//! # What it costs
//!
//! One [`ExprInfo`] (a resolved [`mwl_hir::QName`] plus a handful of already-
//! interned [`crate::ty::TypeId`]s) and one span-keyed hash-map entry per
//! recorded call/`new` in the compiled file — attributable to the request
//! that compiled it, freed with the rest of the check run's tables, and paid
//! once per compile rather than per request the compiled code later serves
//! (ADR 0017's cache makes a compile a rare event, not a per-request cost).

use mwl_diagnostics::Span;
use mwl_hir::QName;
use rustc_hash::FxHashMap;

use crate::ty::TypeId;

/// A stable id for one [`ExprInfo`] recorded in an [`ExprTypeTable`] —
/// assigned in the order [`ExprTypeTable::record`] is first called for a
/// given expression. Never constructed outside this module; a consumer
/// recovers one only via [`ExprTypeTable::lookup`] — see the module docs for
/// why a span, not this id, is the lookup key across a crate boundary.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExprId(u32);

/// A statically resolved call target — an instance method call
/// (`$obj->method(...)`), a static call (`self::method(...)`/
/// `Class::method(...)`), or `new`'s own constructor invocation — whenever
/// [`crate::signatures::resolve_method`] found a declared signature for it.
/// Deliberately does not record *whether* it needs a receiver value at the
/// call site: that is a property of which [`mwl_syntax::ast::ExprKind`]
/// produced the entry (a method call needs one, a static call or a
/// constructor invocation does not), which the caller already knows from the
/// AST node it looked this entry up for.
#[derive(Clone, Debug)]
pub struct ResolvedCall {
    /// The class that actually declares the resolved method — the receiver's
    /// or `new` target's own class, or an ancestor it inherited the method
    /// from; never `self`/`static`/`parent` unresolved.
    pub class: QName,
    /// The method's own name.
    pub method: String,
    /// Each parameter's declared type, positional — [`crate::signatures::MethodSig::params`].
    pub param_tys: Vec<TypeId>,
    /// Whether the last parameter is variadic — [`crate::signatures::MethodSig::variadic`].
    pub variadic: bool,
    /// The declared return type.
    pub return_ty: TypeId,
}

/// One resolved expression a later pass (today, only `mwl-ir`) needs more
/// than just a [`TypeId`] for. `#[non_exhaustive]`: expect new variants as
/// `mwl-ir` widens past what this first slice needed — see the module docs.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ExprInfo {
    /// A resolved instance or static method call. Carried by an
    /// [`mwl_syntax::ast::ExprKind::MethodCall`] or
    /// [`mwl_syntax::ast::ExprKind::StaticCall`] whose receiver/class side
    /// resolved to a known signature.
    Call(ResolvedCall),
    /// `new Target(...)`. `ctor` is `None` for a class with no explicit
    /// `constructor` — legal per [`crate::expr`]'s own known gaps (no arity
    /// check against zero parameters), so a consumer must handle a `New`
    /// entry with no resolved constructor rather than treating it as an
    /// error.
    New {
        /// The constructed class.
        class: QName,
        /// Its resolved `constructor`, if it declares one.
        ctor: Option<ResolvedCall>,
        /// The `new` expression's own result type — always `Ty::Class(class)`,
        /// recorded directly so a consumer never needs to re-intern it.
        ty: TypeId,
    },
    /// A resolved property access (`$obj->prop`) whose receiver statically
    /// resolved to a known declaring class — never recorded for a shape or
    /// plain-`object` receiver, since ADR 0036 § 4 erases either to `mixed`
    /// with no declaring class to name at all (see
    /// [`crate::expr::check_property_access`]'s own docs for that erasure).
    /// A consumer with no entry for a `PropertyAccess` span must treat it the
    /// same way the checker did: nothing compile-time-known to read.
    Property {
        /// The class that actually declares the property — the receiver's
        /// own class, or an ancestor it inherited the property from.
        class: QName,
        /// The property's own name, `$`-sigil not included.
        name: String,
        /// The property's declared type.
        ty: TypeId,
    },
    /// A resolved array-element access (`$arr[$expr]`, read or write) whose
    /// base statically resolved to a known `Ty::Array` element type — never
    /// recorded when the base erased to `mixed` (an untyped/unresolved
    /// array), mirroring [`ExprInfo::Property`]'s own "nothing compile-time-
    /// known to read" treatment of a shape/plain-`object` receiver. Recorded
    /// for a read exactly like a write: `check_assign`'s general (non-plain-
    /// local) arm routes an assignment target back through the same
    /// [`crate::expr::check_expr`]/`ExprKind::Index` path a bare read takes,
    /// so both are keyed by the `Index` expression's own span, the same "one
    /// resolution, read or write" shape [`ExprInfo::Property`] already has.
    /// Unlike `Property`, no receiver identity needs recording alongside the
    /// type — an array has no declaring class for a consumer to name.
    Index {
        /// The element's declared type.
        elem_ty: TypeId,
    },
}

/// Every [`ExprInfo`] [`crate::check::check_program`] recorded this run,
/// looked up by the source span of the expression it describes. See the
/// module docs for the full design and why a span is the lookup key.
#[derive(Debug, Default)]
pub struct ExprTypeTable {
    entries: Vec<ExprInfo>,
    by_span: FxHashMap<Span, ExprId>,
    methods: FxHashMap<Span, String>,
}

impl ExprTypeTable {
    /// An empty table — what a fresh [`crate::check::check_program`] run
    /// starts from.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `info` for the expression at `span`, returning its freshly
    /// assigned [`ExprId`]. Only [`crate::expr`] calls this, at the same
    /// point it already resolved `info` for its own type-checking purposes —
    /// see the module docs for why nothing outside this crate ever
    /// constructs an entry directly.
    ///
    /// If `span` was already recorded (not expected in the current single
    /// left-to-right walk, but not a correctness hazard either way), the new
    /// entry simply gets its own id and `by_span` is repointed at it — the
    /// most recent recording for a given span always wins the lookup.
    pub(crate) fn record(&mut self, span: Span, info: ExprInfo) -> ExprId {
        let id = ExprId(
            u32::try_from(self.entries.len())
                .expect("far fewer than u32::MAX expressions are ever checked in one compilation"),
        );
        self.entries.push(info);
        self.by_span.insert(span, id);
        id
    }

    /// The [`ExprInfo`] recorded for the expression at `span`, if any — `None`
    /// both for a span nothing was ever recorded for and for one belonging to
    /// an expression shape this table doesn't cover yet (see [`ExprInfo`]'s
    /// own known-gaps note).
    #[must_use]
    pub fn lookup(&self, span: Span) -> Option<&ExprInfo> {
        self.by_span
            .get(&span)
            .map(|id| &self.entries[id.0 as usize])
    }

    /// Records the `Class::method` label of the method *declaration* whose
    /// own name sits at `span`. See [`Self::method_label`] for why a
    /// declaration is recorded in a table otherwise about expressions.
    pub(crate) fn record_method(&mut self, span: Span, label: String) {
        self.methods.insert(span, label);
    }

    /// The `Class::method` label of the method declaration whose name sits at
    /// `span` — the *definition* side of the same label [`ResolvedCall`]
    /// renders on the *call* side.
    ///
    /// This is the one entry here keyed by a declaration rather than an
    /// expression, and it is deliberate: the label has to be spelled from a
    /// fully-resolved [`QName`], and `mwl-ir` — which names the function it
    /// lowers — cannot compute one, because it does not depend on `mwl-hir`
    /// at all (see `mwl_ir::lower`'s module docs). Recording it here, at the
    /// point [`crate::check`] already holds the class's `QName`, is what
    /// makes a call's `target` and its callee's name agree *by construction*
    /// rather than by two crates spelling a namespace the same way.
    #[must_use]
    pub fn method_label(&self, span: Span) -> Option<&str> {
        self.methods.get(&span).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;

    use super::*;

    fn span(n: u32) -> Span {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", "");
        Span::new(file, n, n + 1)
    }

    #[test]
    fn recording_and_looking_up_an_entry_round_trips() {
        let mut table = ExprTypeTable::new();
        let mut interner = crate::ty::TypeInterner::new();
        let ty = interner.int();
        table.record(
            span(0),
            ExprInfo::New {
                class: QName::parse("Foo"),
                ctor: None,
                ty,
            },
        );
        assert!(matches!(
            table.lookup(span(0)),
            Some(ExprInfo::New { class, .. }) if class.to_string() == "Foo"
        ));
    }

    #[test]
    fn an_unrecorded_span_looks_up_to_nothing() {
        let table = ExprTypeTable::new();
        assert!(table.lookup(span(0)).is_none());
    }

    // The tests below drive the real `check_program` pipeline end to end,
    // rather than constructing an `ExprInfo` by hand — proving this module's
    // actual producer (`crate::expr::infer`) records the shape `mwl-ir`'s
    // consumer (next session's own widening) will read back.

    use mwl_diagnostics::Diagnostics;
    use mwl_hir::resolve_file;
    use mwl_syntax::ast::{ClassMemberKind, StmtKind};
    use mwl_syntax::parse_file;

    use crate::span_text;
    use crate::ty::TypeInterner;

    /// Parses `src`, checks it, and returns the table plus the span of `T`'s
    /// method `m`'s one and only statement's expression — the shape every
    /// fixture below uses to pin down exactly which `new`/call it means to
    /// inspect.
    fn check_and_find_expr_span(src: &str) -> (ExprTypeTable, Span) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        crate::check_program(
            &stmts,
            map.file(file),
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

        let src_file = map.file(file);
        let decl = stmts
            .iter()
            .find_map(|s| match &s.kind {
                StmtKind::ClassDecl(decl) if span_text(src_file, decl.name.span) == "T" => {
                    Some(decl)
                }
                _ => None,
            })
            .expect("fixture must declare a class `T`");
        let method = decl
            .members
            .iter()
            .find_map(|m| match &m.kind {
                ClassMemberKind::Method(method) if span_text(src_file, method.name) == "m" => {
                    Some(method)
                }
                _ => None,
            })
            .expect("`T` must declare a method `m`");
        let body = method.body.as_ref().expect("`m` must have a body");
        let stmt = body.stmts.first().expect("`m` must have one statement");
        let span = match &stmt.kind {
            StmtKind::Expr(e) => e.span,
            StmtKind::Return(Some(e)) => e.span,
            other => {
                panic!("fixture's statement must be a bare expression or return — got {other:?}")
            }
        };
        (exprs, span)
    }

    #[test]
    fn a_new_with_a_constructor_records_the_resolved_call() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function m(): void {\n    new Foo(1);\n  }\n}\n",
        );
        let Some(ExprInfo::New { class, ctor, .. }) = exprs.lookup(span) else {
            panic!("expected a recorded `New` entry");
        };
        assert_eq!(class.to_string(), "Foo");
        let ctor = ctor.as_ref().expect("Foo declares a constructor");
        assert_eq!(ctor.method, "constructor");
        assert_eq!(ctor.param_tys.len(), 1);
    }

    #[test]
    fn a_new_with_no_constructor_records_no_resolved_ctor() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    new Foo();\n  }\n}\n",
        );
        let Some(ExprInfo::New { class, ctor, .. }) = exprs.lookup(span) else {
            panic!("expected a recorded `New` entry");
        };
        assert_eq!(class.to_string(), "Foo");
        assert!(ctor.is_none());
    }

    #[test]
    fn a_static_call_records_the_resolved_call() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  static function make(): int { return 1; }\n  function m(): void {\n    self::make();\n  }\n}\n",
        );
        let Some(ExprInfo::Call(call)) = exprs.lookup(span) else {
            panic!("expected a recorded `Call` entry");
        };
        assert_eq!(call.class.to_string(), "T");
        assert_eq!(call.method, "make");
        assert!(call.param_tys.is_empty());
    }

    #[test]
    fn an_instance_method_call_records_the_resolved_call() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  function a(int $x): int { return $x; }\n  function m(): void {\n    $this->a(1);\n  }\n}\n",
        );
        let Some(ExprInfo::Call(call)) = exprs.lookup(span) else {
            panic!("expected a recorded `Call` entry");
        };
        assert_eq!(call.class.to_string(), "T");
        assert_eq!(call.method, "a");
        assert_eq!(call.param_tys.len(), 1);
    }

    #[test]
    fn a_property_access_through_a_known_class_records_the_resolved_property() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  public int $count = 0;\n  function m(): int {\n    return $this->count;\n  }\n}\n",
        );
        let Some(ExprInfo::Property { class, name, .. }) = exprs.lookup(span) else {
            panic!("expected a recorded `Property` entry");
        };
        assert_eq!(class.to_string(), "T");
        assert_eq!(name, "count");
    }

    /// A plain `object`-typed receiver erases per ADR 0036 § 4 — there is no
    /// declaring class to record, mirroring
    /// `crate::expr::check_property_access`'s own "nothing diagnosed, nothing
    /// resolved" treatment of that shape.
    #[test]
    fn a_property_access_through_a_plain_object_receiver_records_nothing() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  function m(object $o): mixed {\n    return $o->x;\n  }\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
    }

    #[test]
    fn an_array_index_through_a_known_element_type_records_the_element_type() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  function m(array<int> $a): int {\n    return $a[0];\n  }\n}\n",
        );
        // The recorded `elem_ty` is a `TypeId` from `check_and_find_expr_span`'s
        // own internal interner, which it doesn't hand back — same reason the
        // `Property` test just above doesn't inspect its own `ty` field
        // either, only `class`/`name`. Matching the variant at all is what
        // proves `check_expr`'s `Index` arm actually resolved and recorded
        // something, rather than falling through to the `mixed`-erased case.
        assert!(matches!(exprs.lookup(span), Some(ExprInfo::Index { .. })));
    }

    /// An array subscript through a `mixed`-erased base records nothing —
    /// mirroring [`ExprInfo::Property`]'s own "nothing compile-time-known to
    /// read" treatment of a shape/plain-`object` receiver.
    #[test]
    fn an_array_index_through_a_mixed_base_records_nothing() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  function m(): mixed {\n    return T::UNTYPED[0];\n  }\n  const UNTYPED = 1;\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
    }
}
