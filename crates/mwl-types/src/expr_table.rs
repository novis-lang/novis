//! The typed-expression table — the architecture decision `mwl-ir` widening
//! past scalars needed before it could lower a call, `new`, or a member
//! access: see `docs/agent/handoff.md`'s history for the two options this
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

use crate::defaults::ConstArg;
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
    /// Which parameters are declared `&$x`, positional —
    /// [`crate::signatures::MethodSig::by_ref`]. Recorded rather than left to
    /// `mwl-ir` because a call site's own syntax says nothing about it: PHP
    /// (and MWL) put the `&` on the *declaration*, so the argument
    /// `Adder::bump($n)` looks identical whether `$n` is passed by value or by
    /// reference. `mwl-ir` needs it to decide whether to stage a one-slot
    /// temporary and copy back — see `mwl_ir::ir::InstKind::RefSlot`.
    pub by_ref: Vec<bool>,
    /// Whether the last parameter is variadic — [`crate::signatures::MethodSig::variadic`].
    pub variadic: bool,
    /// Each parameter's evaluated default, positional —
    /// [`crate::signatures::MethodSig::defaults`]. Recorded for
    /// [`Self::by_ref`]'s reason, one step further: a call site's own syntax
    /// says nothing at all about a parameter it *omitted*, so `mwl-ir` has no
    /// way to know either that the callee has more parameters than there are
    /// arguments, or what to pass for them. It materializes one constant per
    /// missing trailing position from this list — see
    /// `mwl_types::defaults` for why the caller does that rather than the
    /// callee.
    pub defaults: Vec<Option<crate::defaults::ConstArg>>,
    /// Whether the resolved method is `static` —
    /// [`crate::signatures::MethodSig::is_static`], which owns the reason this
    /// has to be recorded rather than read off the call's own syntax.
    pub is_static: bool,
    /// The declared return type.
    pub return_ty: TypeId,
    /// Whether the resolved declaration has a body —
    /// [`crate::signatures::MethodSig::has_body`], which owns the reason this
    /// has to be recorded rather than read off the call's own syntax. A `false`
    /// here means [`Self::class`] names no compiled function at all, so the
    /// call has to dispatch on the receiver's runtime class.
    pub has_body: bool,
    /// The class the call site *named*, resolved — `Some` only for a static
    /// call written with an explicit class (`LeafRegistry::make()`), `None`
    /// for an instance call, for `self::`/`static::`/`parent::`, and for
    /// `new`'s own constructor invocation.
    ///
    /// Distinct from [`Self::class`], which is where the method is *declared*:
    /// `LeafRegistry::make()` resolves to `Registry::make`, and late static
    /// binding needs both — the declaring class to know which code to call,
    /// and the named class because that is what `static` means inside it.
    /// PHP's own rule, and the reason `self::`/`parent::`/`static::` record
    /// `None`: those three forward the caller's called class rather than
    /// setting a new one.
    ///
    /// Recorded rather than left to `mwl-ir` for [`ExprInfo::InstanceOf`]'s
    /// reason: resolving a bare `LeafRegistry` against the active namespace
    /// and imports needs context only this crate and `mwl-hir` have.
    pub static_class: Option<QName>,
    /// The class named by the **first written type argument**, for a member on
    /// `mwl_stdlib::registry::WRITTEN_CLASS_MEMBERS` — `Core\Json::decodeAs<User>`
    /// records `User`, and every other call records `None`.
    ///
    /// A type argument is erased like every other one
    /// ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md)), so this
    /// is deliberately not "what `T` bound to": it is the one fact a *native*
    /// member needs that erasure removes, namely which class's
    /// `mwl_runtime::ClassDesc` to build an instance of. `mwl-ir` turns it
    /// into an `InstKind::ClassDescConst` ahead of the call's own arguments;
    /// that roster's docs own the ABI half.
    pub written_class: Option<QName>,
    /// Whether some subtype of [`Self::class`] redeclares [`Self::method`],
    /// so a receiver's runtime class can answer it with different code than
    /// the label [`Self::class`] names —
    /// [`crate::signatures::ClassSignature::overridden_methods`], recorded
    /// here for the same reason [`Self::by_ref`] is: it is a whole-program
    /// question about declarations the call site cannot see, and `mwl-ir`
    /// has no class graph to ask.
    ///
    /// `false` is the common case and the fast one — the call binds to a
    /// compiled label. `true` sends it through
    /// `mwl_ir::ir::InstKind::CallVirtual` with that label as the fallback.
    pub overridden: bool,
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
    /// resolved to a known declaring class — a shape receiver records
    /// [`ExprInfo::ShapeProperty`] instead, and a plain-`object` receiver
    /// records nothing at all, since ADR 0036 § 4 erases it to `mixed` with
    /// no declaring class to name (see
    /// [`crate::expr::members::check_property_access`]'s own docs for that erasure).
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
    /// A resolved access to a property that declares an ADR 0014 § 1 hook
    /// block — recorded *instead of* [`ExprInfo::Property`] for exactly the
    /// same `PropertyAccess` spans, read and write alike, so a consumer that
    /// only knows `Property` cannot silently lower a hooked access as a plain
    /// field touch.
    ///
    /// A hooked access is a **call**, not a field access: the accessor's body
    /// is compiled as an ordinary function under
    /// [`crate::signatures::hook_label`]'s label, and reading or writing the
    /// property invokes it with the receiver as its implicit `$this`. The
    /// two labels are carried rather than re-spelled by the consumer for
    /// [`Self::method_label`]'s reason — they have to agree with the
    /// definition side by construction.
    ///
    /// **Not** recorded for an access inside that property's own hook bodies:
    /// there, `$this->thatProperty` is the backing slot, which is what lets a
    /// `get` hook read what a `set` hook stored without recursing. Such an
    /// access records an ordinary [`ExprInfo::Property`] entry.
    HookedProperty {
        /// The class the access was written on — the same value
        /// [`ExprInfo::Property`] carries, and for the same reason: a field
        /// slot is named by the receiver's own class.
        class: QName,
        /// The property's own name, `$`-sigil not included.
        name: String,
        /// The property's declared type — a `get` hook's return type and a
        /// `set` hook's parameter type both.
        ty: TypeId,
        /// The compiled `get` hook's label, or `None` if the property
        /// declares no `get` hook with a body — in which case a read is an
        /// ordinary slot read after all.
        get: Option<String>,
        /// The compiled `set` hook's label, or `None` if the property
        /// declares no `set` hook with a body — in which case a write is an
        /// ordinary slot write.
        set: Option<String>,
    },
    /// A resolved property access whose receiver is an ADR 0036 § 4 **shape**
    /// that names the field — recorded *instead of* [`ExprInfo::Property`],
    /// because a shape value has no class at all: it is anonymous and
    /// methodless, so there is no declaring name for a consumer to resolve a
    /// layout through.
    ///
    /// What is carried instead is the field's **slot index**, which is its
    /// position in the shape's own field list. That list is sorted by name
    /// when the type is interned ([`crate::ty::TypeInterner::shape`]), and
    /// every producer of a shape value lays its slots out in the same order
    /// (`mwl_stdlib::instance::shape`'s roster is that side of the
    /// agreement), so the index resolved here is the offset the read
    /// actually needs — no layout table is consulted at all.
    ///
    /// A name the shape does *not* list, and a plain `object` receiver, still
    /// record nothing: ADR 0036 § 4 erases both to `mixed` with nothing
    /// compile-time-known to read.
    ShapeProperty {
        /// The field's position in the shape's sorted field list.
        slot: u32,
        /// The field's own declared type.
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
    /// `$a ?? $b`, keyed by the whole binary expression's own span.
    ///
    /// Recorded rather than left to `mwl-ir` because both types it needs are
    /// answers only this crate has. The left operand's representation is
    /// `mwl_ir::Ty::Tagged` by then — a `?T` erases everything but the tag —
    /// so lowering the non-`null` arm has to be *told* which representation to
    /// narrow to; and the result type is `null`-stripped-lhs unioned with rhs,
    /// which is a canonicalization only [`crate::ty::TypeInterner`] performs.
    Coalesce {
        /// The left operand's type with `null` removed — what the value holds
        /// on the arm where the tag says it is not `null`.
        non_null: TypeId,
        /// The whole expression's type: [`Self::Coalesce::non_null`] unioned
        /// with the right operand's.
        result: TypeId,
    },
    /// `$x instanceof Name`, keyed by the *`instanceof` expression's* own
    /// span, whose right-hand side named a class or interface this program
    /// declares (or a reserved global one). Never recorded for the dynamic
    /// `$x instanceof $classNameExpr` form: there is no compile-time-known
    /// class to name, exactly the way [`ExprInfo::Property`] records nothing
    /// for an erased receiver.
    ///
    /// Recorded rather than left to the consumer because resolving a bare
    /// `Animal` to `Ns\Animal` needs the namespace and import context only
    /// this crate and `mwl-hir` have — `mwl-ir` deliberately depends on
    /// neither.
    InstanceOf {
        /// The class or interface tested against.
        class: QName,
    },
    /// `EnumName::CaseName`, keyed by the whole access's own span.
    ///
    /// ADR 0010 § 3 makes a case "an integer constant, inlined at every use
    /// site" — so this is the *value*, resolved once by [`crate::enums`] and
    /// read back by `mwl-ir` as a plain constant. Recorded rather than left to
    /// the consumer for [`ExprInfo::InstanceOf`]'s reason and one more: the
    /// enum's name needs namespace/import context only this crate has, and the
    /// auto-increment rule that gives an unwritten case its value needs the
    /// whole declaration in view.
    ///
    /// Never recorded for an ordinary `Class::CONST`, whose value is unmodeled
    /// (see [`crate::expr`]'s own known gaps).
    EnumCase {
        /// The case's constant value, in its enum's backing type.
        value: crate::enums::EnumValue,
    },
    /// `Core\Class::CONSTANT`, keyed by the whole access's own span.
    ///
    /// [ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)'s
    /// class constant, which `Core\Math::PI` is the first of. It carries the
    /// value for [`ExprInfo::EnumCase`]'s reason exactly: a constant is
    /// inlined at every use site, so there is no storage a consumer could read
    /// it back from, and the [`ConstArg`] here is the same shape a parameter
    /// default already lowers through.
    ///
    /// Never recorded for a **user-declared** class's constant, whose value is
    /// unmodeled (see [`crate::expr`]'s own known gaps) — only
    /// `mwl_stdlib::registry` states a constant's value today.
    CoreConst {
        /// The constant's value, in its declared type.
        value: ConstArg,
    },
    /// An [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
    /// `fn` closure literal, keyed by the literal's own span.
    ///
    /// A closure's *type* is [`crate::ty::Ty::Callable`] and says nothing
    /// about it — ADR 0031 § 4 keeps that type opaque, and ADR 0027 already
    /// fixed what may satisfy it. So everything lowering one needs is here
    /// instead: the class label `mwl-ir` synthesizes the closure's captured
    /// environment as, the outer bindings that environment holds, and the
    /// value the body produces.
    ///
    /// Recorded rather than re-derived for [`ExprInfo::InstanceOf`]'s reason
    /// twice over. The capture set is "exactly the outer variables its body
    /// reads" (§ 2), which is a fact only the checker's own scope walk knows;
    /// and an expression body's return type is inferred from that body, which
    /// is the checker's job by definition.
    Closure {
        /// The label of the class `mwl-ir` synthesizes for this closure's
        /// captured environment. Contains a `$`, which no MWL identifier may,
        /// so it can never collide with a declared class.
        class: String,
        /// Every outer binding the body reads or writes, in first-touch
        /// order — the field order of the class above. `$this` appears here
        /// under the name `this`, which is ADR 0008 § 4's "a closure binds
        /// `$this` only where the body uses it" falling straight out of § 2's
        /// capture rule rather than needing a rule of its own.
        captures: Vec<(String, TypeId)>,
        /// The value the body produces — the declared return type, or, for an
        /// expression body with none written, the type inferred from that
        /// expression.
        return_ty: TypeId,
    },
}

/// Which of ADR 0053 § 3's three subject shapes a `foreach` is walking, and
/// therefore which loop `mwl-ir` emits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ForeachDrive {
    /// An `array<T>`: walked slot by slot with no interface call and no
    /// allocation. The only shape with keys.
    Array,
    /// An `Iterable<T>`: `iterate()` is called once and the cursor it returns
    /// is driven. Wins over [`Self::Cursor`] when a class reaches both — see
    /// [`crate::signatures::resolve_iteration_element`] for why.
    Iterable,
    /// An `Iterator<T>`: driven directly, `advance()` then `current()`, with
    /// no `iterate()` call at all.
    Cursor,
}

/// Every [`ExprInfo`] [`crate::check::check_program`] recorded this run,
/// looked up by the source span of the expression it describes. See the
/// module docs for the full design and why a span is the lookup key.
#[derive(Debug, Default)]
pub struct ExprTypeTable {
    entries: Vec<ExprInfo>,
    by_span: FxHashMap<Span, ExprId>,
    methods: FxHashMap<Span, String>,
    types: FxHashMap<Span, TypeId>,
    foreach: FxHashMap<Span, ForeachDrive>,
    codecs: FxHashMap<String, crate::derive::DerivedCodec>,
    property_defaults: FxHashMap<String, Vec<(String, crate::defaults::ConstArg)>>,
    to_string: FxHashMap<Span, ResolvedCall>,
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

    /// Every [`ExprInfo::Closure`] recorded this run, in the order the
    /// checker met each `fn` literal — its environment class label, its
    /// capture list and its return type.
    ///
    /// The one accessor here that iterates rather than looks a span up:
    /// `mwl-ir` reaches a closure through the literal it is lowering, but a
    /// test (and, later, anything that has to enumerate the synthesized
    /// classes) has no span to start from.
    pub fn closures(&self) -> impl Iterator<Item = (&str, &Vec<(String, TypeId)>, TypeId)> {
        self.entries.iter().filter_map(|info| match info {
            ExprInfo::Closure {
                class,
                captures,
                return_ty,
            } => Some((class.as_str(), captures, *return_ty)),
            _ => None,
        })
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

    /// Records the class labelled `label` as carrying ADR 0071's
    /// `#[Json\Derive]`, with the field list [`crate::derive`] read off its
    /// declaration.
    pub(crate) fn record_codec(&mut self, label: String, codec: crate::derive::DerivedCodec) {
        self.codecs.insert(label, codec);
    }

    /// The derived JSON codec of the class labelled `label`, or `None` when it
    /// carries no `#[Json\Derive]`.
    ///
    /// Keyed by a class label rather than a span, for [`Self::method_label`]'s
    /// reason one step further: the fact is about a *declaration*, and the one
    /// consumer (`mwl_ir::lower::lower_file`, joining it against the slot order
    /// in `mwl_types::layout`) reaches it by label, never by AST node.
    #[must_use]
    pub fn codec(&self, label: &str) -> Option<&crate::derive::DerivedCodec> {
        self.codecs.get(label)
    }

    /// Records the class labelled `label`'s **own** evaluated property
    /// defaults — [`crate::signatures::ClassSignature::property_defaults`],
    /// copied across at check time.
    ///
    /// Copied rather than read straight out of the signature table because
    /// `mwl-ir` is handed this table and not that one, and threading a second
    /// one through `lower_program` would change every caller for a fact that
    /// already has a home here beside [`Self::record_codec`].
    pub(crate) fn record_property_defaults(
        &mut self,
        label: String,
        defaults: Vec<(String, crate::defaults::ConstArg)>,
    ) {
        self.property_defaults.insert(label, defaults);
    }

    /// The class labelled `label`'s own property defaults, in declaration
    /// order — empty for a class that declares none, and for every class in a
    /// program that writes no `= expr` on a property.
    ///
    /// **Own only**: an inherited property's default is recorded under the
    /// class that declared it, so `mwl_ir::lower` walks a class's own entry
    /// and then its ancestors' — see [`Self::codec`] for why this is keyed by
    /// label.
    #[must_use]
    pub fn property_defaults(&self, label: &str) -> &[(String, crate::defaults::ConstArg)] {
        self.property_defaults.get(label).map_or(&[], Vec::as_slice)
    }

    /// Records how the `foreach` whose subject sits at `span` reaches its
    /// elements. See [`Self::foreach_drive`].
    pub(crate) fn record_foreach(&mut self, span: Span, drive: ForeachDrive) {
        self.foreach.insert(span, drive);
    }

    /// Which of ADR 0053 § 3's three shapes the `foreach` subject at `span`
    /// turned out to be — `None` for a subject that erased to
    /// `mixed`/`iterable` or was already diagnosed as none of the three,
    /// which is the same "nothing compile-time-known" answer
    /// [`ExprInfo::Property`] gives an erased receiver.
    ///
    /// Its own map rather than an [`ExprInfo`] variant, and deliberately: a
    /// subject is an ordinary expression that has usually already recorded an
    /// entry of its own under exactly this span (`foreach (new Nums(5) as …)`
    /// records an [`ExprInfo::New`] there), and [`Self::record`] repoints
    /// `by_span` at the most recent entry. Two independent facts about one
    /// span need two maps; this is the same reason [`Self::record_method`]
    /// and [`Self::record_type`] have theirs.
    ///
    /// Recorded rather than left to `mwl-ir` for [`ExprInfo::InstanceOf`]'s
    /// reason: reaching `Iterable` through a base class is a
    /// [`crate::signatures::resolve_iteration_element`] walk over
    /// [`mwl_hir::ClassGraph`], which `mwl-ir` does not depend on.
    #[must_use]
    pub fn foreach_drive(&self, span: Span) -> Option<ForeachDrive> {
        self.foreach.get(&span).copied()
    }

    /// Records the resolved `toString()` the operand at `span` is implicitly
    /// converted through. See [`Self::to_string_call`].
    pub(crate) fn record_to_string(&mut self, span: Span, call: ResolvedCall) {
        self.to_string.insert(span, call);
    }

    /// The `Stringable::toString()` the object-typed operand at `span`
    /// stringifies through — ADR 0028 § 1's implicit conversion, resolved
    /// against the operand's own class, or `None` when the operand was not an
    /// object at all (every scalar row, which needs no call) or was already
    /// diagnosed as not implementing `Stringable`.
    ///
    /// Its own map rather than an [`ExprInfo`] variant, for
    /// [`Self::foreach_drive`]'s reason: the operand is an ordinary
    /// expression that usually already records an entry under exactly this
    /// span — `echo $registry->name()` records an [`ExprInfo::Call`] for the
    /// call *it* is, and the `toString` it then stringifies through is a
    /// second, independent fact about the same span.
    ///
    /// Recorded rather than left to `mwl-ir` for [`ExprInfo::InstanceOf`]'s
    /// reason: an implicit conversion site is not a call expression, so the
    /// consumer has no call node to resolve, and walking
    /// [`mwl_hir::ClassGraph`] for the declaring class is not something that
    /// crate can do at all.
    #[must_use]
    pub fn to_string_call(&self, span: Span) -> Option<&ResolvedCall> {
        self.to_string.get(&span)
    }

    /// Records the [`TypeId`] a *declared* type annotation at `span` resolved
    /// to. See [`Self::declared_ty`] for why.
    pub(crate) fn record_type(&mut self, span: Span, ty: TypeId) {
        self.types.insert(span, ty);
    }

    /// The resolved type of the annotation whose [`mwl_syntax::ast::Type`]
    /// node sits at `span` — a parameter's, a local declaration's, a `catch`
    /// clause's, or an `as` conversion's target.
    ///
    /// The second entry here keyed by a declaration rather than an expression,
    /// and for [`Self::method_label`]'s reason: `mwl-ir` lowers a declared
    /// type straight off the AST (`mwl_ir::lower::lower_decl_type`), which
    /// works for every atom that *is* its own answer — `int`, `array<T>`, a
    /// plain class name — but not for one whose meaning depends on
    /// resolution. An enum name is the first such atom: ADR 0010 makes
    /// `Rank $r` an integer binding, and telling that apart from `Dog $d`
    /// needs the symbol table, which `mwl-ir` does not have. So the
    /// resolution happens once, here, at the same
    /// [`crate::lower::lower_type`] call the checker already makes.
    #[must_use]
    pub fn declared_ty(&self, span: Span) -> Option<TypeId> {
        self.types.get(&span).copied()
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
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
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
    fn an_inherited_constructor_records_the_class_that_declares_it() {
        // `new Bar()` allocates a `Bar` and calls `Foo::constructor`. Naming
        // `Bar::constructor` instead leaves `mwl-codegen` looking for a
        // function the unit never compiled.
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass Foo {\n  function constructor() {}\n}\nclass Bar extends Foo {}\nclass T {\n  function m(): void {\n    new Bar();\n  }\n}\n",
        );
        let Some(ExprInfo::New { class, ctor, .. }) = exprs.lookup(span) else {
            panic!("expected a recorded `New` entry");
        };
        assert_eq!(class.to_string(), "Bar");
        let ctor = ctor.as_ref().expect("Bar inherits a constructor");
        assert_eq!(ctor.class.to_string(), "Foo");
    }

    #[test]
    fn an_instanceof_records_the_resolved_class() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(Foo $f): bool {\n    return $f instanceof Foo;\n  }\n}\n",
        );
        let Some(ExprInfo::InstanceOf { class }) = exprs.lookup(span) else {
            panic!("expected a recorded `InstanceOf` entry");
        };
        assert_eq!(class.to_string(), "Foo");
    }

    #[test]
    fn a_dynamic_instanceof_records_nothing() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(Foo $f, string $n): bool {\n    return $f instanceof $n;\n  }\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
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

    /// ADR 0014 § 1: a property that declares a hook records
    /// [`ExprInfo::HookedProperty`] *instead of* [`ExprInfo::Property`], so a
    /// consumer that only knows the latter cannot lower a hooked access as a
    /// plain field touch by accident. Both accessor labels ride along, since
    /// the same entry answers a read and a write.
    #[test]
    fn a_hooked_property_access_records_its_accessors_instead_of_the_slot() {
        let (exprs, span) = check_and_find_expr_span(concat!(
            "<?mwl\nclass T {\n",
            "  public int $hits;\n",
            "  public int $doubled { get => $this->hits * 2; set(int $v) { $this->hits = $v; } }\n",
            "  function constructor(int $hits) { $this->hits = $hits; }\n",
            "  function m(): int { return $this->doubled; }\n}\n",
        ));
        let Some(ExprInfo::HookedProperty {
            class,
            name,
            get,
            set,
            ..
        }) = exprs.lookup(span)
        else {
            panic!("expected a recorded `HookedProperty` entry");
        };
        assert_eq!(class.to_string(), "T");
        assert_eq!(name, "doubled");
        assert_eq!(get.as_deref(), Some("T::$doubled::get"));
        assert_eq!(set.as_deref(), Some("T::$doubled::set"));
    }

    /// The one place a hooked property is *not* a call: inside its own hooks,
    /// where `$this->p` is the backing slot. Without this the `get` hook
    /// below would call itself forever.
    #[test]
    fn a_hooked_property_read_inside_its_own_hook_records_the_plain_slot() {
        let (exprs, _span) = check_and_find_expr_span(concat!(
            "<?mwl\nclass T {\n",
            "  public int $n { get => $this->n + 1; }\n",
            "  function constructor() { }\n",
            "  function m(): int { return 0; }\n}\n",
        ));
        // The hook body's own `$this->n` is the only access in the file.
        let hooked = exprs
            .entries
            .iter()
            .filter(|e| matches!(e, ExprInfo::HookedProperty { .. }))
            .count();
        let plain = exprs
            .entries
            .iter()
            .filter(|e| matches!(e, ExprInfo::Property { name, .. } if name == "n"))
            .count();
        assert_eq!((hooked, plain), (0, 1));
    }

    /// A shape receiver has no declaring class either, but it does have a
    /// layout: ADR 0036 § 4's field read resolves to the field's position in
    /// the shape's *sorted* list, which is why `path` is slot 1 of
    /// `{path, message}` rather than slot 0.
    #[test]
    fn a_property_access_through_a_shape_receiver_records_its_slot() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  function m({path: string, message: string} $i): string {\n    return $i->path;\n  }\n}\n",
        );
        assert!(matches!(
            exprs.lookup(span),
            Some(ExprInfo::ShapeProperty { slot: 1, .. })
        ));
    }

    /// A name the shape does not list is erased exactly like a plain `object`
    /// receiver: ADR 0036 § 4 answers `mixed` and records nothing, since
    /// whether it is there at all is a runtime question.
    #[test]
    fn a_property_access_naming_a_field_the_shape_lacks_records_nothing() {
        let (exprs, span) = check_and_find_expr_span(
            "<?mwl\nclass T {\n  function m({path: string} $i): mixed {\n    return $i->nope;\n  }\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
    }

    /// A plain `object`-typed receiver erases per ADR 0036 § 4 — there is no
    /// declaring class to record, mirroring
    /// `crate::expr::members::check_property_access`'s own "nothing diagnosed, nothing
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
