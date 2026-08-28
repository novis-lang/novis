//! The typed-expression table — the architecture decision `nvs-ir` widening
//! past scalars needed before it could lower a call, `new`, or a member
//! access: see `docs/agent/handoff.md`'s history for the two options this
//! was weighed against (`nvs-ir` depending on `nvs-types` and duplicating its
//! resolution logic, versus this crate publishing a persisted result
//! `nvs-ir` reads back) and why the second was chosen.
//!
//! # What this is, and why it's shaped this way
//!
//! [`expr::infer`](crate::expr::infer) already resolves a call's or `new`'s
//! target — walking [`crate::signatures::SignatureTable`] and
//! [`nvs_hir::ClassGraph`] to find the actual declaring class and its
//! [`crate::signatures::MethodSig`] — but until now threw that resolution
//! away the moment it returned a [`crate::ty::TypeId`]. [`ExprTypeTable`] is
//! where [`crate::check::check_program`] persists it instead: one
//! [`ExprInfo`] entry per expression whose *resolved identity* (not just its
//! type) a later pass needs and cannot cheaply re-derive from the AST alone.
//!
//! This is a narrow, deliberately incomplete table — it exists to answer
//! exactly the questions `nvs-ir`'s widening needed answered, not to become a
//! second, general-purpose typed-AST. [`ExprInfo::Property`] is the first
//! instance of the pattern this module's docs originally predicted: widening
//! `nvs-ir` further (array access, `match`/ternary result identity, ...) is
//! expected to keep growing [`ExprInfo`] with one new variant per question,
//! each populated at its own `expr::infer`/`check_property_access`-style call
//! site — not to replace this shape. See the crate's own known-gaps list for
//! exactly which expression shapes have no entry here yet.
//!
//! # Why a lookup is keyed by [`nvs_diagnostics::Span`], not assignment order
//!
//! [`ExprId`] is a real, stable id — assigned once, in the order
//! [`crate::check::check_program`]'s single left-to-right AST walk first
//! records each entry, and never reused. But a consumer in another crate
//! (`nvs-ir`) cannot re-derive *that* order for itself: its own lowering walk
//! is a second, independently-shaped traversal of the same AST (for example,
//! it may skip a dynamic member-name sub-expression this crate's checker
//! still visits), so "the Nth entry this crate recorded" and "the Nth
//! call-shaped node `nvs-ir` visits" are not guaranteed to line up. The one
//! thing both crates *do* agree on without coordinating their walk order is
//! the source [`nvs_diagnostics::Span`] each AST node already carries — so
//! [`ExprTypeTable::lookup`] takes a span, not an id, and [`ExprId`] itself is
//! never constructed outside this module. This mirrors why `nvs-ir`'s own
//! [`ids`](../nvs_ir/ids/index.html) module numbers `StmtId`/`EdgeId` from a
//! *single* deterministic walk rather than letting two passes agree on
//! numbering independently — the same hazard, resolved the other way because
//! here the two walks unavoidably live in two different crates.
//!
//! # What it costs
//!
//! One [`ExprInfo`] (a resolved [`nvs_hir::QName`] plus a handful of already-
//! interned [`crate::ty::TypeId`]s) and one span-keyed hash-map entry per
//! recorded call/`new` in the compiled file — attributable to the request
//! that compiled it, freed with the rest of the check run's tables, and paid
//! once per compile rather than per request the compiled code later serves
//! (ADR 0017's cache makes a compile a rare event, not a per-request cost).

use nvs_diagnostics::Span;
use nvs_hir::QName;
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
/// call site: that is a property of which [`nvs_syntax::ast::ExprKind`]
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
    /// Which parameters are declared `inout $x`, positional —
    /// [`crate::signatures::MethodSig::inout`]. ADR 0107 § 2 puts the word at
    /// the call site too, so `Adder::bump(inout $n)` does say which arguments
    /// these are — but it says it in the *source*, and `nvs-ir` lowers a
    /// resolved call rather than re-resolving one, so the agreement this
    /// checker enforced (`E0713`/`E0714`) is recorded here rather than
    /// re-derived from a signature `nvs-ir` no longer holds. It is what
    /// decides whether to stage a one-slot temporary and copy back — see
    /// `nvs_ir::ir::InstKind::RefSlot`.
    pub inout: Vec<bool>,
    /// Whether the last parameter is variadic — [`crate::signatures::MethodSig::variadic`].
    pub variadic: bool,
    /// Each parameter's evaluated default, positional —
    /// [`crate::signatures::MethodSig::defaults`]. Recorded for
    /// [`Self::inout`]'s reason, one step further: a call site's own syntax
    /// says nothing at all about a parameter it *omitted*, so `nvs-ir` has no
    /// way to know either that the callee has more parameters than there are
    /// arguments, or what to pass for them. It materializes one constant per
    /// missing trailing position from this list — see
    /// `nvs_types::defaults` for why the caller does that rather than the
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
    /// Recorded rather than left to `nvs-ir` for [`ExprInfo::InstanceOf`]'s
    /// reason: resolving a bare `LeafRegistry` against the active namespace
    /// and imports needs context only this crate and `nvs-hir` have.
    pub static_class: Option<QName>,
    /// The class named by the **first written type argument**, for a member on
    /// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` — `Core\Json::decodeAs<User>`
    /// records `User`, and every other call records `None`.
    ///
    /// A type argument is erased like every other one
    /// ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md)), so this
    /// is deliberately not "what `T` bound to": it is the one fact a *native*
    /// member needs that erasure removes, namely which class's
    /// `nvs_runtime::ClassDesc` to build an instance of. `nvs-ir` turns it
    /// into an `InstKind::ClassDescConst` ahead of the call's own arguments;
    /// that roster's docs own the ABI half.
    pub written_class: Option<QName>,
    /// Whether some subtype of [`Self::class`] redeclares [`Self::method`],
    /// so a receiver's runtime class can answer it with different code than
    /// the label [`Self::class`] names —
    /// [`crate::signatures::ClassSignature::overridden_methods`], recorded
    /// here for the same reason [`Self::inout`] is: it is a whole-program
    /// question about declarations the call site cannot see, and `nvs-ir`
    /// has no class graph to ask.
    ///
    /// `false` is the common case and the fast one — the call binds to a
    /// compiled label. `true` sends it through
    /// `nvs_ir::ir::InstKind::CallVirtual` with that label as the fallback.
    pub overridden: bool,
    /// Which parameter each **written** argument fills, in the order the call
    /// site wrote them — one entry per [`nvs_syntax::ast::Arg`].
    ///
    /// For a plain positional list this is `[Param(0), Param(1), …]` and says
    /// nothing new. It exists for the two shapes where an argument's position
    /// is not its parameter's: `name: value` fills the parameter its name
    /// resolved to, and `...$rest` fills the variadic tail with the subject's
    /// own entries. `nvs-ir` cannot re-derive either — a name resolves against
    /// [`crate::signatures::MethodSig::param_names`], which the IR has no
    /// access to — so the mapping is settled once, here, by the checker that
    /// already had to do it to type the arguments at all.
    pub arg_slots: Vec<ArgSlot>,
}

/// Which parameter one written argument fills — [`ResolvedCall::arg_slots`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArgSlot {
    /// The parameter at this index takes this argument's own value: a
    /// positional argument at its own position, or a named one at the
    /// position its name resolved to.
    Param(usize),
    /// `...$rest`: the argument is an `array<T>` and *its entries* are
    /// appended to the variadic parameter at this index, which is always the
    /// last one.
    Spread(usize),
    /// The argument fills no parameter, and a diagnostic already said why.
    /// A consumer never sees one: `nvs-ir` runs only on a program that
    /// reported nothing.
    Unresolved,
}

/// One resolved expression a later pass (today, only `nvs-ir`) needs more
/// than just a [`TypeId`] for. `#[non_exhaustive]`: expect new variants as
/// `nvs-ir` widens past what this first slice needed — see the module docs.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ExprInfo {
    /// A resolved instance or static method call. Carried by an
    /// [`nvs_syntax::ast::ExprKind::MethodCall`] or
    /// [`nvs_syntax::ast::ExprKind::StaticCall`] whose receiver/class side
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
    /// resolved to a known declaring class — a shape receiver and a
    /// plain-`object` one both record [`ExprInfo::ShapeProperty`] instead,
    /// since ADR 0036 § 4 gives neither a declaring class to name (see
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
    /// A resolved `Class::$prop` access, read or write — the static
    /// counterpart of [`ExprInfo::Property`], and recorded for the same reason:
    /// `nvs-ir` has no way of its own to turn the written class expression
    /// (`self`, `static`, `parent` or a name) into the label the storage is
    /// keyed on.
    ///
    /// `class` is the class that actually **declares** the property, not the
    /// one written at the access. That is what makes `Sub::$count` and
    /// `Base::$count` the one slot PHP makes them, with no flattening step
    /// anywhere below: the checker has already resolved the name through the
    /// class graph ([`crate::signatures::resolve_property_owned`]), so the
    /// label recorded here *is* the storage's identity.
    StaticProperty {
        /// The class that declares the property.
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
    /// (`nvs_stdlib::instance::shape`'s roster is that side of the
    /// agreement), so the index resolved here is the offset the read
    /// actually needs — no layout table is consulted at all.
    ///
    /// A name the shape does *not* list, and a plain `object` receiver, are
    /// § 4's fully **erased** half and record this same variant — the name is
    /// all an erased access has, and the name is what the fetch is keyed on
    /// either way. Both carry `slot: 0` and a `ty` of `mixed`: there is no
    /// static layout to hint from, so the runtime's by-name search answers or
    /// § 4's catchable missing-name throw fires. A consumer therefore cannot
    /// read `slot` as a proven offset or `ty` as a proven type — only the
    /// *shape-listed* case is either, which is why neither is worth
    /// distinguishing at the consumer.
    ShapeProperty {
        /// The field's own name, `$`-sigil not included. What the runtime
        /// fetch is keyed on: ADR 0036 § 4 makes the read name-keyed, because
        /// [`Self::ShapeProperty::slot`] is only the layout of the receiver's
        /// *static* shape and a widened view's is not the value's own.
        name: String,
        /// The field's position in the shape's sorted field list — a hint the
        /// runtime tries first, not the answer. See `nvs_ir::InstKind::SlotGet`.
        slot: u32,
        /// The field's own declared type.
        ty: TypeId,
    },
    /// A resolved array-element access (`$arr[$expr]`, read or write) whose
    /// base statically resolved to a known `Ty::Array` element type, **or**
    /// erased to `mixed` in a read position, where ADR 0036 § 4's deferral
    /// applies one storage kind along from a member access: the element type
    /// is `mixed` too, and `nvs-ir` picks
    /// [`nvs_ir::Helper::ValueIndexGet`](../../nvs_ir/ir/enum.Helper.html)
    /// over `InstKind::ArrayGet` off the base's own representation rather
    /// than off anything recorded here. Every other base — a scalar, an
    /// untested `?array<T>`, a union naming no array — is `E0482` where it
    /// is written and records nothing, and so is a `mixed` in a **write**
    /// target, whose copy-on-write separation has no holder to write back
    /// through. There is no element-access counterpart to
    /// [`ExprInfo::ShapeProperty`]'s erased half: a property has a written
    /// name to key a runtime fetch on and a subscript has only a value.
    /// Recorded
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
        /// Whether this read is the left operand of a `??`, and so must
        /// answer an absent key with `null` rather than throwing.
        ///
        /// PHP's `??` is exactly *"absent or `null`, without the warning"*, so
        /// the guarded read is the one place ADR 0007 § 7 row 11's divergence
        /// is carved back out — a `$a["k"] ?? "d"` that threw would refuse the
        /// very spelling PHP offers for the safe read. Recorded here because
        /// the question is about the *expression tree*, which only this crate
        /// walks: `nvs-ir` sees one subscript at a time and would have to
        /// re-derive its parent to ask it. A guarded read's type is
        /// `?elem_ty`, which is what puts the `null` this promises inside the
        /// left operand's static type and stops
        /// [`ExprInfo::Coalesce`]'s lowering short-circuiting the whole `??`
        /// away.
        guarded: bool,
    },
    /// A read of a local a dominating `!= null` test **narrowed** — `$m`
    /// inside `if ($m != null) { … }`, where [`crate::locals`]' `narrow`
    /// proved the binding cannot be `null` on this path.
    ///
    /// Keyed by the [`nvs_syntax::ast::ExprKind::Variable`] read's own span,
    /// which carries no other entry, and it is the whole of what makes a
    /// narrowing usable below the checker. `nvs-ir` gives a `?T` local one
    /// `nvs_ir::ty::Ty::Tagged` slot whatever a condition later proves about
    /// it, so every *consumer* of such a read — a call receiver, a subscript
    /// base, a `foreach` subject, an argument — would otherwise have to narrow
    /// for itself, and one forgotten site is a cranelift rejection rather than
    /// a panic. Recording the fact at the read means it is discharged **once,
    /// where the value is produced**, leaving no site to forget:
    /// `nvs_ir::lower::Lowering::lower_expr`'s `Variable` arm is that one site.
    ///
    /// Recorded only where the narrowing actually changed the answer, so a
    /// read of an ordinary non-nullable binding carries no entry at all.
    NarrowedRead {
        /// What the test proved — the declared union with `null` dropped, and
        /// exactly the type [`crate::locals::LocalScope::declared_ty`]
        /// answered this read with.
        to: TypeId,
    },
    /// `$a ?? $b`, keyed by the whole binary expression's own span.
    ///
    /// Recorded rather than left to `nvs-ir` because both types it needs are
    /// answers only this crate has. The left operand's representation is
    /// `nvs_ir::Ty::Tagged` by then — a `?T` erases everything but the tag —
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
    /// `$a == $b` — or `!=` — where at least one operand is statically
    /// `secret`, keyed by the *comparison's* own span.
    /// [ADR 0033](../../../docs/adr/0033-secret-qualifier-for-confidential-values.md)
    /// § 5: that pair lowers to a constant-time comparison rather than the
    /// short-circuiting one every other operand pair uses.
    ///
    /// Carries nothing, because there is nothing to carry: the question
    /// `nvs-ir` asks is a single bit, and it cannot ask it for itself.
    /// [`crate::ty::Ty`]'s qualifier lives in the checker's type and
    /// `nvs_ir::ty::Ty` has no room for it — a `secret string` and a `string`
    /// are one representation, which is exactly what ADR 0033 § 1 promises
    /// and why the erasure is right. So the *presence of this entry* is the
    /// whole message, the way [`Self::EnumCase`] carries a value the AST
    /// alone does not hold.
    ///
    /// Recorded for a `secret` operand on **either** side, per § 5's own
    /// reading: where exactly one is `secret`, § 2 has already poisoned the
    /// value that reached the other, so the pair is `secret`.
    SecretEquality,
    /// `$x instanceof Name`, keyed by the *`instanceof` expression's* own
    /// span, whose right-hand side named a class or interface this program
    /// declares (or a reserved global one). Never recorded for the dynamic
    /// `$x instanceof $classNameExpr` form: there is no compile-time-known
    /// class to name, exactly the way [`ExprInfo::Property`] records nothing
    /// for an erased receiver — but unlike that receiver, the form is refused
    /// where it is written (`E0496`), so no program `nvs-ir` sees reaches an
    /// unrecorded entry. An enum, a `Core` class and a name resolving to
    /// nothing are refused on the same pass, the last as the ordinary `E0303`.
    ///
    /// Recorded rather than left to the consumer because resolving a bare
    /// `Animal` to `Ns\Animal` needs the namespace and import context only
    /// this crate and `nvs-hir` have — `nvs-ir` deliberately depends on
    /// neither.
    InstanceOf {
        /// The class or interface tested against.
        class: QName,
    },
    /// `EnumName::CaseName`, keyed by the whole access's own span.
    ///
    /// ADR 0010 § 3 makes a case "an integer constant, inlined at every use
    /// site" — so this is the *value*, resolved once by [`crate::enums`] and
    /// read back by `nvs-ir` as a plain constant. Recorded rather than left to
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
    /// `nvs_stdlib::registry` states a constant's value today.
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
    /// instead: the class label `nvs-ir` synthesizes the closure's captured
    /// environment as, the outer bindings that environment holds, and the
    /// value the body produces.
    ///
    /// Recorded rather than re-derived for [`ExprInfo::InstanceOf`]'s reason
    /// twice over. The capture set is "exactly the outer variables its body
    /// reads" (§ 2), which is a fact only the checker's own scope walk knows;
    /// and an expression body's return type is inferred from that body, which
    /// is the checker's job by definition.
    Closure {
        /// The label of the class `nvs-ir` synthesizes for this closure's
        /// captured environment. Contains a `$`, which no Novis identifier may,
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
/// therefore which loop `nvs-ir` emits.
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
    property_types: FxHashMap<String, Vec<(String, TypeId)>>,
    static_properties: FxHashMap<String, Vec<(String, Option<crate::defaults::ConstArg>)>>,
    to_string: FxHashMap<Span, ResolvedCall>,
    require_targets: FxHashMap<Span, nvs_diagnostics::SourceId>,
}

impl ExprTypeTable {
    /// An empty table — what a fresh [`crate::check::check_program`] run
    /// starts from.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `info` for the expression at `span`, returning its freshly
    /// assigned [`ExprId`]. [`crate::expr`] calls this at the same point it
    /// already resolved `info` for its own type-checking purposes — see the
    /// module docs for why nothing outside this crate ever constructs an
    /// entry directly. [`crate::locals`] has the one other call site, for
    /// the one thing that resolves an element type at a span holding no
    /// expression: a destructuring leaf, which gets the same
    /// [`ExprInfo::Index`] entry the subscript it is spelled out of would
    /// (see `walk_destructure_target`).
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
    /// `nvs-ir` reaches a closure through the literal it is lowering, but a
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
    /// fully-resolved [`QName`], and `nvs-ir` — which names the function it
    /// lowers — cannot compute one, because it does not depend on `nvs-hir`
    /// at all (see `nvs_ir::lower`'s module docs). Recording it here, at the
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
    /// consumer (`nvs_ir::lower::lower_file`, joining it against the slot order
    /// in `nvs_types::layout`) reaches it by label, never by AST node.
    #[must_use]
    pub fn codec(&self, label: &str) -> Option<&crate::derive::DerivedCodec> {
        self.codecs.get(label)
    }

    /// Records the class labelled `label`'s **own** evaluated property
    /// defaults — [`crate::signatures::ClassSignature::property_defaults`],
    /// copied across at check time.
    ///
    /// Copied rather than read straight out of the signature table because
    /// `nvs-ir` is handed this table and not that one, and threading a second
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
    /// class that declared it, so `nvs_ir::lower` walks a class's own entry
    /// and then its ancestors' — see [`Self::codec`] for why this is keyed by
    /// label.
    #[must_use]
    pub fn property_defaults(&self, label: &str) -> &[(String, crate::defaults::ConstArg)] {
        self.property_defaults.get(label).map_or(&[], Vec::as_slice)
    }

    /// Records which file the `require` whose path expression is at `span`
    /// resolved to — `nvs_hir::Loaded::requires`, copied across by whoever
    /// ran both phases.
    ///
    /// This crate neither produces nor reads the fact: the `require` graph
    /// walk is the only place a written path is joined to a base directory,
    /// canonicalized and checked, and `nvs-ir` is the only consumer, needing
    /// it to call that file's own script frame at the site. It rides here
    /// for [`Self::record_property_defaults`]'s reason exactly — `nvs-ir` is
    /// handed this table and not `nvs-hir`'s output, and threading a second
    /// one through `lower_program` would change every caller (and every
    /// frame below it, a `require` being writable inside any body) for a
    /// fact that has a home beside the others here.
    pub fn record_require_target(&mut self, span: Span, target: nvs_diagnostics::SourceId) {
        self.require_targets.insert(span, target);
    }

    /// The file the `require` at `span` resolved to, or `None` for a path
    /// that is not a literal, names nothing loadable, or closes a cycle —
    /// each of which is already a diagnostic or ADR 0021's dynamic fallback,
    /// so the site has nothing to call.
    #[must_use]
    pub fn require_target(&self, span: Span) -> Option<nvs_diagnostics::SourceId> {
        self.require_targets.get(&span).copied()
    }

    /// Records the class labelled `label`'s **own** `static` properties and
    /// their evaluated initializers —
    /// [`crate::signatures::ClassSignature::static_properties`], copied across
    /// at check time for [`Self::record_property_defaults`]'s reason.
    pub(crate) fn record_static_properties(
        &mut self,
        label: String,
        statics: Vec<(String, Option<crate::defaults::ConstArg>)>,
    ) {
        self.static_properties.insert(label, statics);
    }

    /// The class labelled `label`'s own `static` properties, in declaration
    /// order — empty for a class that declares none.
    ///
    /// **Own only**, and unlike [`Self::property_defaults`] never joined with
    /// an ancestor's: `Sub::$count` and the `Base::$count` it inherits are one
    /// storage, held under `Base`'s label, so `nvs_ir::lower` enumerates each
    /// class's own entry and stops there.
    #[must_use]
    pub fn static_properties(&self, label: &str) -> &[(String, Option<crate::defaults::ConstArg>)] {
        self.static_properties.get(label).map_or(&[], Vec::as_slice)
    }

    /// Records the class labelled `label`'s **own** declared property types,
    /// sorted by name — [`crate::signatures::ClassSignature::properties`],
    /// copied across at check time for [`Self::record_property_defaults`]'s
    /// reason, and sorted because the signature's own map has no order a
    /// build can reproduce.
    pub(crate) fn record_property_types(&mut self, label: String, types: Vec<(String, TypeId)>) {
        self.property_types.insert(label, types);
    }

    /// The class labelled `label`'s own declared property types, by name.
    ///
    /// **Own only**, joined against the flattened slot order the same way
    /// [`Self::property_defaults`] is — see `nvs_ir::lower`'s `field_reprs`,
    /// which is what ADR 0036 § 4's erased *write* check is built out of: a
    /// name this list does not carry leaves that slot unchecked rather than
    /// mistyped.
    #[must_use]
    pub fn property_types(&self, label: &str) -> &[(String, TypeId)] {
        self.property_types.get(label).map_or(&[], Vec::as_slice)
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
    /// Recorded rather than left to `nvs-ir` for [`ExprInfo::InstanceOf`]'s
    /// reason: reaching `Iterable` through a base class is a
    /// [`crate::signatures::resolve_iteration_element`] walk over
    /// [`nvs_hir::ClassGraph`], which `nvs-ir` does not depend on.
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
    /// Recorded rather than left to `nvs-ir` for [`ExprInfo::InstanceOf`]'s
    /// reason: an implicit conversion site is not a call expression, so the
    /// consumer has no call node to resolve, and walking
    /// [`nvs_hir::ClassGraph`] for the declaring class is not something that
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

    /// The resolved type of the annotation whose [`nvs_syntax::ast::Type`]
    /// node sits at `span` — a parameter's, a local declaration's, a `catch`
    /// clause's, or an `as` conversion's target.
    ///
    /// The second entry here keyed by a declaration rather than an expression,
    /// and for [`Self::method_label`]'s reason: `nvs-ir` lowers a declared
    /// type straight off the AST (`nvs_ir::lower::lower_decl_type`), which
    /// works for every atom that *is* its own answer — `int`, `array<T>`, a
    /// plain class name — but not for one whose meaning depends on
    /// resolution. An enum name is the first such atom: ADR 0010 makes
    /// `Rank $r` an integer binding, and telling that apart from `Dog $d`
    /// needs the symbol table, which `nvs-ir` does not have. So the
    /// resolution happens once, here, at the same
    /// [`crate::lower::lower_type`] call the checker already makes.
    #[must_use]
    pub fn declared_ty(&self, span: Span) -> Option<TypeId> {
        self.types.get(&span).copied()
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;

    use super::*;

    fn span(n: u32) -> Span {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", "");
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
    // actual producer (`crate::expr::infer`) records the shape `nvs-ir`'s
    // consumer (next session's own widening) will read back.

    use nvs_diagnostics::Diagnostics;
    use nvs_hir::resolve_file;
    use nvs_syntax::ast::{ClassMemberKind, StmtKind};
    use nvs_syntax::parse_file;

    use crate::span_text;
    use crate::ty::TypeInterner;

    /// Parses `src`, checks it, and returns the table plus the span of `T`'s
    /// method `m`'s one and only statement's expression — the shape every
    /// fixture below uses to pin down exactly which `new`/call it means to
    /// inspect.
    fn check_and_find_expr_span(src: &str) -> (ExprTypeTable, Span) {
        let (exprs, span, diags) = check_fixture(src);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        (exprs, span)
    }

    /// [`check_and_find_expr_span`] without its "and it checked cleanly"
    /// assertion, for the one fixture whose whole point is the diagnostic.
    fn check_fixture(src: &str) -> (ExprTypeTable, Span, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
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
        (exprs, span, diags)
    }

    #[test]
    fn a_new_with_a_constructor_records_the_resolved_call() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function m(): void {\n    new Foo(1);\n  }\n}\n",
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
            "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    new Foo();\n  }\n}\n",
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
        // `Bar::constructor` instead leaves `nvs-codegen` looking for a
        // function the unit never compiled.
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass Foo {\n  function constructor() {}\n}\nclass Bar extends Foo {}\nclass T {\n  function m(): void {\n    new Bar();\n  }\n}\n",
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
            "<?nvs\nclass Foo {}\nclass T {\n  function m(Foo $f): bool {\n    return $f instanceof Foo;\n  }\n}\n",
        );
        let Some(ExprInfo::InstanceOf { class }) = exprs.lookup(span) else {
            panic!("expected a recorded `InstanceOf` entry");
        };
        assert_eq!(class.to_string(), "Foo");
    }

    /// The dynamic form records nothing *and* is refused where it is written:
    /// ADR 0007 § 2 has no dynamic class names, so there is no entry for
    /// `nvs-ir` to read and no program that reaches it.
    #[test]
    fn a_dynamic_instanceof_records_nothing_and_is_refused() {
        let (exprs, span, diags) = check_fixture(
            "<?nvs\nclass Foo {}\nclass T {\n  function m(Foo $f, string $n): bool {\n    return $f instanceof $n;\n  }\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(nvs_diagnostics::code::E_INSTANCEOF_NOT_A_CLASS)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_static_call_records_the_resolved_call() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  static function make(): int { return 1; }\n  function m(): void {\n    self::make();\n  }\n}\n",
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
            "<?nvs\nclass T {\n  function a(int $x): int { return $x; }\n  function m(): void {\n    $this->a(1);\n  }\n}\n",
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
            "<?nvs\nclass T {\n  public int $count = 0;\n  function m(): int {\n    return $this->count;\n  }\n}\n",
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
            "<?nvs\nclass T {\n",
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
            "<?nvs\nclass T {\n",
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
            "<?nvs\nclass T {\n  function m({path: string, message: string} $i): string {\n    return $i->path;\n  }\n}\n",
        );
        assert!(matches!(
            exprs.lookup(span),
            Some(ExprInfo::ShapeProperty { slot: 1, .. })
        ));
    }

    /// A name the shape does not list is erased exactly like a plain `object`
    /// receiver: ADR 0036 § 4 answers `mixed` and hints no slot, since whether
    /// the field is there at all is a runtime question. The name is still
    /// recorded — it is what the fetch is keyed on.
    #[test]
    fn a_property_access_naming_a_field_the_shape_lacks_records_the_name_alone() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m({path: string} $i): mixed {\n    return $i->nope;\n  }\n}\n",
        );
        let Some(ExprInfo::ShapeProperty { name, slot, .. }) = exprs.lookup(span) else {
            panic!("expected a recorded `ShapeProperty` entry");
        };
        assert_eq!((name.as_str(), *slot), ("nope", 0));
    }

    /// A plain `object`-typed receiver erases per ADR 0036 § 4 — there is no
    /// declaring class to record and no layout to hint from, so what is
    /// recorded is the written name and nothing else, which is exactly what
    /// the runtime fetch needs.
    #[test]
    fn a_property_access_through_a_plain_object_receiver_records_the_name_alone() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m(object $o): mixed {\n    return $o->x;\n  }\n}\n",
        );
        let Some(ExprInfo::ShapeProperty { name, slot, .. }) = exprs.lookup(span) else {
            panic!("expected a recorded `ShapeProperty` entry");
        };
        assert_eq!((name.as_str(), *slot), ("x", 0));
    }

    #[test]
    fn an_array_index_through_a_known_element_type_records_the_element_type() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m(array<int> $a): int {\n    return $a[0];\n  }\n}\n",
        );
        // The recorded `elem_ty` is a `TypeId` from `check_and_find_expr_span`'s
        // own internal interner, which it doesn't hand back — same reason the
        // `Property` test just above doesn't inspect its own `ty` field
        // either, only `class`/`name`. Matching the variant at all is what
        // proves `check_expr`'s `Index` arm actually resolved and recorded
        // something, rather than falling through to the `mixed`-erased case.
        assert!(matches!(exprs.lookup(span), Some(ExprInfo::Index { .. })));
    }

    /// An array subscript through a `mixed`-erased base is ADR 0036 § 4's
    /// deferral rather than a refusal: the entry is recorded with a `mixed`
    /// element type, and `nvs-ir` reads the base's *representation* to pick
    /// the tag-asking read (`nvs_ir::Helper::ValueIndexGet`) over the
    /// statically typed one. Nothing is diagnosed, because there is nothing
    /// the declared type could have answered.
    #[test]
    fn an_array_index_through_a_mixed_base_defers_to_the_tag() {
        let (exprs, span, diags) = check_fixture(
            "<?nvs\nclass T {\n  function m(): mixed {\n    return T::UNTYPED[0];\n  }\n  const UNTYPED = 1;\n}\n",
        );
        assert!(matches!(exprs.lookup(span), Some(ExprInfo::Index { .. })));
        assert_eq!(diags.iter().count(), 0);
    }

    /// The other side of that bound: a base whose *declared* type already
    /// answers "there is no array here" keeps `E0482` where it is written,
    /// because the deferral is what `mixed` is for and a type that has
    /// answered the question does not get to ask it again at run time.
    #[test]
    fn an_array_index_through_a_scalar_base_is_still_refused() {
        let (exprs, span, diags) = check_fixture(
            "<?nvs\nclass T {\n  function m(int $n): mixed {\n    return $n[0];\n  }\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
        assert_eq!(diags.iter().count(), 1);
        assert_eq!(
            diags.iter().next().and_then(|d| d.code.as_ref()),
            Some(&nvs_diagnostics::code::E_SUBSCRIPT_ON_NON_ARRAY)
        );
    }
}
