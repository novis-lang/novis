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
//! (`rule:config/an-edit-reaches-the-next-request-without-a-restart`'s cache makes a compile a rare event, not a per-request cost).

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

/// The inline shape a call site wrote as the type argument of a member on
/// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` — `Core\Arr::shapeAs<{n: int}>`.
///
/// Two facts rather than one, because a shape's class and a shape's wire
/// contract are keyed differently and have to be: [`Self::label`] names the
/// class `nvs-ir` synthesizes, which is keyed on the sorted field *names*
/// alone, so `{n: int}` and `{n: string}` are one class and one label;
/// [`Self::codec`] is the span the type argument was written at, which is what
/// [`ExprTypeTable::shape_codec`] answers the per-field wire types under —
/// exactly the fact the label cannot tell apart.
#[derive(Clone, Debug)]
pub struct WrittenShape {
    /// `$shape{n}` — [`crate::derive::shape_class_label`] over the shape's
    /// sorted field names, which is the label `nvs-ir` registers the
    /// synthesized class under and an `InstKind::ClassDescConst` resolves.
    pub label: String,
    /// The type argument's own span, the key
    /// [`ExprTypeTable::shape_codec`] reads this shape's
    /// [`crate::derive::DerivedCodec`] back out under.
    pub codec: Span,
}

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
    /// Each parameter's own name without the `$`, positional and one entry per
    /// [`Self::param_tys`] entry — [`crate::signatures::MethodSig::param_names`].
    ///
    /// Recorded here for the same reason [`Self::param_tys`] is: the names are
    /// a property of the *declaration* this call resolved to, and a consumer
    /// downstream of checking holds the call site rather than the signature
    /// table. ADR 0006 § *Decision*'s `args:` binding is the consumer — a
    /// method entry's child calls its target with the map's entries bound by
    /// name, and `nvs_ir::lower`'s `spawn_method_label` reads the names off
    /// this entry to emit them beside the label. `nvs_runtime::MethodRow` is
    /// not a second source: it carries arity and parameter tags, never names.
    pub param_names: Vec<String>,
    /// Which parameters are declared `inout $x`, positional —
    /// [`crate::signatures::MethodSig::inout`]. `rule:statements/inout-is-written-at-the-call` puts the word at
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
    /// (`rule:types/declaration`), so this
    /// is deliberately not "what `T` bound to": it is the one fact a *native*
    /// member needs that erasure removes, namely which class's
    /// `nvs_runtime::ClassDesc` to build an instance of. `nvs-ir` turns it
    /// into an `InstKind::ClassDescConst` ahead of the call's own arguments;
    /// that roster's docs own the ABI half.
    ///
    /// `Core\Json::decodeAs<array<User>>` records `User` too — the class is the
    /// *element's*, and [`Self::written_class_is_list`] is what tells the two
    /// apart.
    pub written_class: Option<QName>,
    /// Whether what was written — [`Self::written_class`] or
    /// [`Self::written_shape`] — was wrapped in an `array<...>`, so the member
    /// decodes a JSON array into one instance per element rather than the
    /// document into one instance.
    ///
    /// A separate field rather than a second `QName` because the native member
    /// needs a descriptor either way: erasure removes which class, and this
    /// removes nothing further — `array` has no descriptor to build. `nvs-ir`
    /// emits it as an `InstKind::ConstBool` in the slot after the descriptor.
    pub written_class_is_list: bool,
    /// The **inline shape** written in that same first type argument, where
    /// what was written is a shape rather than a class —
    /// `Core\Arr::shapeAs<{n: int}>` records `$shape{n}` and the span its wire
    /// contract is filed under.
    ///
    /// A field of its own rather than a second spelling of
    /// [`Self::written_class`], which is a [`QName`]: a shape's class is
    /// synthesized by `nvs-ir` and labelled `$shape{…}`, which no name can be.
    /// At most one of the two is `Some`, and a member off that roster records
    /// neither.
    pub written_shape: Option<WrittenShape>,
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
/// The two labels `rule:classes/property-observer-pipeline`'s observer step dispatches through, recorded on
/// a property access whose receiver's class implements `PropertyObserver`.
///
/// § 3 makes the observer a *second* step over every read and write of every
/// declared property, hooked or not, so it hangs off the access rather than
/// off the property: one entry answers both `ExprInfo::Property` and
/// `ExprInfo::HookedProperty`, and the consumer picks the half its direction
/// names.
///
/// Each label is the **statically resolved** `"Owner::onPropertyGet"`, present
/// only when that resolution has a body — exactly `ExprInfo::Call`'s
/// `has_body` convention, and for its reason: the call dispatches on the
/// receiver's runtime class (a subclass may override the observer), so the
/// label is `InstKind::CallVirtual`'s fallback and never its target.
/// [`crate::conformance`] is what makes both `Some` in practice; a class that
/// implements the interface and declares neither body is refused there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverCalls {
    /// `onPropertyGet`'s fallback label — the read half of § 3's pipeline.
    pub get: Option<String>,
    /// `onPropertySet`'s fallback label — the write half.
    pub set: Option<String>,
}

/// One piece of `rule:routing/link-name-and-params-are-checked`'s resolved link, in path order and each carrying
/// its own leading `/`: concatenating them left to right rebuilds the route's
/// declared path with every capture substituted.
///
/// The route's `path` is **not** carried beside this. § 2's grammar is read
/// once, by [`crate::routes::link_pieces`], and what crosses to `nvs-ir` is its
/// answer — `docs/agent/loop-goal.md` § *Standing decisions* makes a fold and
/// its runtime path one implementation, and a template the runtime re-parsed
/// would be the second copy that rule exists to refuse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlPiece {
    /// A literal segment, `/` included — compared byte for byte at match time
    /// (`rule:classes/names-resolve-case-sensitively`),
    /// so it is copied out exactly as declared.
    Literal(String),
    /// `{name}`: `/` and then `$params[name]`, percent-encoded — § 4's launder
    /// for the URL-path sink.
    Required(String),
    /// `{name?}`: [`Self::Required`], or nothing at all — its `/` included —
    /// where `$params` holds no such key.
    Optional(String),
    /// `{name...}`: `/` and then `$params[name]`, whose own `/`s are the one
    /// thing not percent-encoded, because § 2 gives this form every remaining
    /// segment rather than one.
    Rest(String),
}

/// One enum-spelled capture's conversion table: what run time writes for a case
/// that arrives as its backing integer.
///
/// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
/// decides the spelling while compiling, and a **name**-spelled subset is the
/// half that leaves work over. A case is indistinguishable from its integer by
/// the time it is a value, so a link would otherwise build `/s/1` for a route
/// whose segment is `/s/Sale`. A value-spelled subset gets no entry at all: the
/// decimal `nvs_runtime::value_to_string` already writes *is* its segment.
///
/// Not a check, and that is what makes it affordable at all — the set a link is
/// refused against is `crate::links`' and stays there, and these rows answer
/// only what a segment *is*. `nvs_stdlib::router`'s `substitute` owns the other
/// half of that distinction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumSpelling {
    /// The capture's name, as the piece that substitutes it names it.
    pub capture: String,
    /// Every admitted case: its backing value in decimal — the text a `$params`
    /// entry holding the case arrives as — and the segment spelling it is
    /// written into the link as.
    pub cases: Vec<(String, String)>,
}

impl UrlPiece {
    /// A resolved link written out in the one format
    /// [`nvs_stdlib::router::link`] reads, which is that module's to define —
    /// this is the writing end of it and holds no second opinion about the
    /// spelling.
    ///
    /// A string rather than a structure because what carries it is an ordinary
    /// [`nvs_ir::InstKind::ConstStr`](../nvs_ir/ir/enum.InstKind.html) argument
    /// into the same `CoreCall` any other `Core` member takes, so the prepared
    /// artifact costs the instruction a literal would have cost anyway.
    ///
    /// **Every spelling row is written in front of the first path piece**, and
    /// the reader is why: `substitute` builds its answer as it walks, so a
    /// conversion table arriving after the capture it converts would arrive too
    /// late. One row per admitted case, each holding the capture it belongs to,
    /// so reading a row back stays a `split` of a fixed three fields.
    #[must_use]
    pub fn prepared(pieces: &[Self], spellings: &[EnumSpelling]) -> String {
        let mut out = String::new();
        for spelling in spellings {
            for (value, segment) in &spelling.cases {
                if !out.is_empty() {
                    out.push(nvs_stdlib::router::link::PIECE_SEPARATOR);
                }
                out.push(char::from(nvs_stdlib::router::link::SPELLING));
                out.push_str(&spelling.capture);
                out.push(nvs_stdlib::router::link::FIELD_SEPARATOR);
                out.push_str(value);
                out.push(nvs_stdlib::router::link::FIELD_SEPARATOR);
                out.push_str(segment);
            }
        }
        for piece in pieces {
            if !out.is_empty() {
                out.push(nvs_stdlib::router::link::PIECE_SEPARATOR);
            }
            let (tag, text) = match piece {
                Self::Literal(text) => (nvs_stdlib::router::link::LITERAL, text),
                Self::Required(name) => (nvs_stdlib::router::link::REQUIRED, name),
                Self::Optional(name) => (nvs_stdlib::router::link::OPTIONAL, name),
                Self::Rest(name) => (nvs_stdlib::router::link::REST, name),
            };
            out.push(char::from(tag));
            out.push_str(text);
        }
        out
    }
}

/// `nvs-ir` widens past what this first slice needed — see the module docs.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ExprInfo {
    /// A resolved instance or static method call. Carried by an
    /// [`nvs_syntax::ast::ExprKind::MethodCall`] or
    /// [`nvs_syntax::ast::ExprKind::StaticCall`] whose receiver/class side
    /// resolved to a known signature.
    Call(ResolvedCall),
    /// `Foo::bar(...)` / `$obj->method(...)` — `rule:types/callable-is-a-closure`'s first-class
    /// callable syntax, which *names* the resolved member rather than calling
    /// it, and whose value is a closure over it.
    ///
    /// Recorded *instead of* [`ExprInfo::Call`] for the same span, and
    /// carrying the same [`ResolvedCall`], because the two ends need exactly
    /// the same facts about the target and differ only in what they do with
    /// them: a call emits one, a reference captures one. A consumer tells them
    /// apart by the variant rather than by re-reading
    /// [`nvs_syntax::ast::CallArgs`] out of the AST, so the checker's reading
    /// of the sentinel is the only one.
    ///
    /// Only ever recorded when the member resolved. The three shapes that
    /// name no member are refused where they are written and record nothing:
    /// a `mixed` receiver (`E_FIRST_CLASS_CALLABLE_ERASED_RECEIVER`, since a
    /// closure carries its callee with it and `rule:types/erased-member-access`'s run-time
    /// dispatch has no callee to carry), an erased `object`/shape receiver
    /// (`E_METHOD_ON_ERASED_RECEIVER`), and an unresolved class expression,
    /// which `nvs_hir::members` has already reported. So a consumer that
    /// finds no entry on a first-class-callable span is looking at a program
    /// that did not compile.
    ///
    /// `ResolvedCall::arg_slots` is empty here and means nothing: `(...)` is a
    /// sentinel, not an argument list, so there are no written arguments to
    /// map. Every other field is the one an ordinary call would carry,
    /// [`ResolvedCall::static_class`] included — `rule:types/callable-is-a-closure` makes
    /// `static::helper(...)` late-bound exactly as `static::helper()` is.
    CallableRef(ResolvedCall),
    /// A bare name in callee position that is
    /// `rule:types/closure-self-name`'s self-name — `fact` inside `fn fact(int $n): int => … fact($n - 1)`.
    ///
    /// Recorded on the **callee's** span, not the call's, because it is the
    /// resolution of that name and nothing else: the call around it is the
    /// ordinary call through a `callable`, and the enclosing
    /// [`nvs_syntax::ast::ExprKind::Call`] carries an entry of its own only
    /// where the callee's type names its parameters
    /// ([`Self::CallThroughSignature`]), which a self-name does not: the name
    /// resolves to the closure being written, not to a declared `callable`.
    ///
    /// Carries nothing. The value the name resolves to is the invoke's own
    /// receiver, which the consumer already holds — `nvs_ir::lower::closure`'s
    /// `FN_SELF` — so a field naming the closure's class would be a second copy
    /// of a fact the frame being lowered *is*. A consumer that finds this on a
    /// span it is not lowering a closure body for is looking at a program that
    /// did not compile: the checker binds the name for one body only
    /// ([`crate::expr::calls::check_fn_literal`]).
    ClosureSelf,
    /// `$f(...)` where `$f`'s type is `rule:types/callable-signature`'s written
    /// signature — the call site whose arguments are proven where they are
    /// written, recorded on the **call's** own span.
    ///
    /// A call through bare `callable` carries no entry, and that absence is the
    /// signal: nothing is proven there, so `nvs-ir` keeps the dynamic path and
    /// its per-argument tag check. Recorded only for a plain positional list,
    /// since a `...` makes the argument count a run-time fact and a `name:`
    /// argument is refused outright (`E0712`).
    ///
    /// Both halves are what the consumer cannot recompute. `params` is the
    /// representation each argument must reach the callee in, because
    /// `nvs_runtime::closure`'s `check_param_tags` — which is what widened an
    /// `int` into a `float` parameter — is exactly what this entry removes.
    /// `ret` is what the call answers, which `nvs-ir` otherwise reads as
    /// `mixed` for want of a resolved target to name.
    CallThroughSignature {
        /// The callee's declared parameter types, left to right.
        params: Vec<TypeId>,
        /// The callee's declared return type.
        ret: TypeId,
    },
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
    /// `new $cls(...)` over a `class<T>` operand —
    /// `rule:types/class-reference-sites`'s dynamic form, and a variant of its own precisely because it
    /// cannot answer the question [`ExprInfo::New`] is built around. A `New`
    /// entry names **the class a layout comes from**, and here that is
    /// whichever implementor the descriptor in hand holds — a fact no compile
    /// has. Recording the bound as a `New` would lower an allocation of the
    /// base, silently and only for the one site where the base is the wrong
    /// answer.
    ///
    /// So the class allocated is not in this entry at all: it is the operand,
    /// which a consumer lowers to a class-descriptor value and hands to
    /// `nvs_ir::ir::InstKind::NewDynamic` as its `desc`, exactly as
    /// `new static()` hands that instruction late static binding's own
    /// descriptor. What is recorded here is the bound, which is what the site
    /// *checked* against.
    NewDynamic {
        /// The `class<T>` operand's bound — `T`, never the class allocated.
        bound: QName,
        /// `T`'s resolved `constructor`, if its chain declares one, and
        /// [`ExprInfo::New`]'s `ctor` in every other respect. Sound to check a
        /// call against even though the allocated class may declare its own,
        /// because `rule:classes/constructor-compatibility` refuses at this very site any implementor of
        /// `T` whose constructor is not compatible with it (`E0794`).
        ctor: Option<ResolvedCall>,
        /// The expression's own result type — always `Ty::Class(bound)`, which
        /// is § 4's "resolves against `T`" as a consumer sees it.
        ty: TypeId,
    },
    /// `$cls::f(...)` over a `class<T>` class side — `rule:types/class-reference-sites`'s second
    /// site, and an [`ExprInfo::Call`] in every field it carries.
    ///
    /// A variant of its own for [`ExprInfo::NewDynamic`]'s reason, one step
    /// on: a `Call`'s target is a label a consumer binds straight to, and
    /// binding to it here would run `T`'s body wherever the descriptor holds
    /// an implementor that overrides it. So the resolved call inside is `T`'s
    /// declaration used as the *fallback*, and the call itself dispatches on
    /// the descriptor the class side evaluates to.
    ///
    /// **The target is always `static`.** A class reference names a class and
    /// never an object, so there is no receiver an instance member could be
    /// reached through — including `$this`, which belongs to a class the
    /// reference has nothing to do with. [`crate::expr::calls`]'s
    /// static-call refusal is where that is reported.
    ///
    /// Known gap: `$cls::f(...)` written as `rule:types/callable-is-a-closure`'s first-class callable
    /// records [`ExprInfo::CallableRef`] like any other class side, so the
    /// closure it names is `T`'s method rather than the implementor's. The
    /// same fallback-versus-override question as above, at a site that has no
    /// descriptor to dispatch on once the closure has escaped.
    ClassRefCall(ResolvedCall),
    /// `$m->method(...)` on a **`mixed`** receiver — the one method call that
    /// resolves to no signature and is not refused where it is written.
    ///
    /// `rule:types/conversion` makes `mixed` the one unchecked position, so `rule:types/erased-member-access`'s deferral covers a call as well as a property access: which class
    /// is behind the handle, and whether there is one at all, is a run-time
    /// question, and `docs/adr/README.md` § *Decisions taken at project start*
    /// owns the convention that answers it — the receiver's own descriptor
    /// marshals the call, its method row carrying the callee's arity and
    /// parameter tags. Every *other* receiver naming no class is
    /// `E_METHOD_ON_ERASED_RECEIVER` instead, so an entry here is the checker
    /// saying "dispatch on the value" rather than "I could not tell".
    ///
    /// The **name** is all this carries, and it is all the dispatch reads:
    /// there is no signature to record an argument mapping from, which is why
    /// a `name:` argument is refused at the site
    /// (`E_NAMED_ARG_THROUGH_CALLABLE`) and every written argument fills its
    /// own position. Recorded rather than read back out of the source for
    /// [`Self::ShapeProperty`]'s reason: the fetch is keyed on the name, and
    /// one home for what that name is keeps the two ends from disagreeing.
    ErasedCall {
        /// The member name written at the call site.
        name: String,
    },
    /// A resolved property access (`$obj->prop`) whose receiver statically
    /// resolved to a known declaring class — a shape receiver and a
    /// plain-`object` one both record [`ExprInfo::ShapeProperty`] instead,
    /// since `rule:types/erased-member-access` gives neither a declaring class to name (see
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
        /// `rule:classes/property-observer-pipeline`'s second step, or `None` when the receiver's class
        /// implements no `PropertyObserver` — see [`ObserverCalls`].
        observer: Option<ObserverCalls>,
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
    /// A resolved access to a property that declares an `rule:classes/property-hooks` hook
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
    /// [`ExprTypeTable::method_label`]'s reason — they have to agree with the
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
        /// `rule:classes/property-observer-pipeline`'s second step, or `None` when the receiver's class
        /// implements no `PropertyObserver`. A hooked property is **not**
        /// exempt from it — see [`ObserverCalls`].
        observer: Option<ObserverCalls>,
    },
    /// A resolved property access whose receiver is an `rule:types/erased-member-access` **shape**
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
        /// fetch is keyed on: `rule:types/erased-member-access` makes the read name-keyed, because
        /// [`Self::ShapeProperty::slot`] is only the layout of the receiver's
        /// *static* shape and a widened view's is not the value's own.
        name: String,
        /// The field's position in the shape's sorted field list — a hint the
        /// runtime tries first, not the answer. See `nvs_ir::InstKind::SlotGet`.
        slot: u32,
        /// The field's own declared type. Not the type the *expression* has:
        /// a guarded read of an optional field answers `?ty`, exactly as
        /// [`Self::Index`] records `elem_ty` and leaves the `null` to the
        /// consumer.
        ty: TypeId,
        /// Whether an absent field must answer `null` here rather than taking
        /// `rule:types/erased-member-access`'s throw — this access is under a
        /// `??`, an `isset` or an `empty`, and the name is one the receiver's
        /// shape marks optional or one an erased receiver cannot promise at
        /// all. [`Self::Index::guarded`] is the same bit one storage kind
        /// along, and carries the reasoning for both.
        ///
        /// A **required** field is never guarded whatever it is written
        /// under: `rule:types/shape-type` proves it present, so there is no
        /// absence for the mark to answer for and `$p->b ?? 0` on one is the
        /// short-circuit it already was.
        guarded: bool,
    },
    /// A resolved array-element access (`$arr[$expr]`, read or write) whose
    /// base statically resolved to a known `Ty::Array` element type, **or**
    /// erased to `mixed` in a read position, where `rule:types/erased-member-access`'s deferral
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
        /// the guarded read is the one place `rule:php-migration/every-divergence-is-deliberate-and-listed` row 11's divergence
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
        /// What the test proved — the declared union with `null` dropped, or
        /// the class an `instanceof` named, and in either case exactly the
        /// type [`crate::locals::LocalScope::declared_ty`] answered this read
        /// with.
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
    /// `rule:security/secret-comparison-is-constant-time`
    /// : that pair lowers to a constant-time comparison rather than the
    /// short-circuiting one every other operand pair uses.
    ///
    /// Carries nothing, because there is nothing to carry: the question
    /// `nvs-ir` asks is a single bit, and it cannot ask it for itself.
    /// [`crate::ty::Ty`]'s qualifier lives in the checker's type and
    /// `nvs_ir::ty::Ty` has no room for it — a `secret string` and a `string`
    /// are one representation, which is exactly what `rule:security/secret-qualifier` promises
    /// and why the erasure is right. So the *presence of this entry* is the
    /// whole message, the way [`Self::EnumCase`] carries a value the AST
    /// alone does not hold.
    ///
    /// Recorded for a `secret` operand on **either** side, per § 5's own
    /// reading: where exactly one is `secret`, § 2 has already poisoned the
    /// value that reached the other, so the pair is `secret`.
    SecretEquality,
    /// `$a + $b` over two sink carriers of the same kind, keyed by the `+`
    /// expression's own span —
    /// `rule:core-classes/html-auto-escape`'s `Markup + Markup` and
    /// `rule:tooling/styling-is-a-value-not-a-grammar`'s
    /// `Text + Text`, which [`crate::expr::operators`] admits as one rule.
    ///
    /// Carries the symbol rather than the class for [`Self::SecretEquality`]'s
    /// reason, one step further along: `nvs_ir::ty::Ty` erases a class to
    /// `Ty::Object`, so both carriers arrive at the lowering as the same pair
    /// of representations and the *presence* of an entry is no longer enough —
    /// there are two answers now, and which one is the checker's to say.
    CarrierComposition {
        /// The `nvs_ir::ir::InstKind::CoreCall` symbol the composition lowers
        /// to: [`crate::CORE_HTML_MARKUP_CONCAT`] or
        /// [`crate::CORE_CLI_TEXT_CONCAT`].
        symbol: &'static str,
    },
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
    /// `$x is T`, keyed by the *`is` expression's* own span: the type its
    /// right-hand side lowered to.
    ///
    /// Recorded only where the test does real work. A settled test folds to
    /// `true` or `false` and carries [`Self::SettledTypeTest`] instead, and a
    /// refused right-hand side has no type to carry
    /// (`crate::expr::type_test`) — so **this** variant on a span means the
    /// answer is a run-time `bool`, which is exactly the case a consumer has
    /// something to do about: narrowing on the true edge
    /// (`rule:types/narrowing`) and, for `nvs-ir`, a test to emit.
    ///
    /// Recorded rather than left to the consumer for [`ExprInfo::InstanceOf`]'s
    /// reason, one step wider than a class name: the right-hand side is a
    /// written *type*, and lowering one places every name in it by the
    /// namespace and the imports of the site that wrote it and interns the
    /// result — context `nvs-ir` has neither, holding no
    /// [`crate::ty::TypeInterner`] at all.
    TypeTest {
        /// The type tested against, as [`crate::lower::lower_type`] interned
        /// it. Never a `void`, a `never` or a qualified atom: those are the
        /// right-hand sides `is` refuses.
        tested: TypeId,
    },
    /// `$x is T` the checker **settled**, carrying the constant it folded to —
    /// the other half of [`Self::TypeTest`], and never recorded beside one.
    ///
    /// Which of the two variants an `is` expression carries is the whole of
    /// what a consumer needs to know about it: this one means there is no
    /// run-time test, so [`crate::locals::narrow`] narrows nothing (a target
    /// that would widen a binding folded to `true` and is here) and `nvs-ir`
    /// emits the constant rather than a comparison.
    ///
    /// The constant is recorded rather than re-derived because nothing below
    /// this crate can re-derive it: the fold is a question about the
    /// *checker's* types — `mixed` accepting everything, a union disjoint from
    /// a subject — and two settled tests whose subject and target erase to the
    /// same pair of representations can fold opposite ways
    /// (`?int $x; $x is int|null` against `$x is string|float`).
    SettledTypeTest {
        /// What the test answers, at every execution.
        answer: bool,
    },
    /// `EnumName::CaseName`, keyed by the whole access's own span.
    ///
    /// `rule:enums/no-class-machinery` makes a case "an integer constant, inlined at every use
    /// site" — so this is the *value*, resolved once by [`crate::enums`] and
    /// read back by `nvs-ir` as a plain constant. Recorded rather than left to
    /// the consumer for [`ExprInfo::InstanceOf`]'s reason and one more: the
    /// enum's name needs namespace/import context only this crate has, and the
    /// auto-increment rule that gives an unwritten case its value needs the
    /// whole declaration in view.
    ///
    /// Never recorded for an ordinary `Class::CONST`, whose value travels in
    /// [`ExprInfo::ClassConst`] instead.
    /// The enum and the case are carried beside the value for
    /// [`ExprInfo::InstanceOf`]'s reason a second time: `rule:types/literal-types`'s guard
    /// row narrows a local to the case's own `Ty::EnumCase`, and *which* case
    /// a written `Mode::Read` names is a question about the namespace and the
    /// imports of the site that wrote it — context
    /// [`crate::locals::literal_residue`] does not carry. The value alone
    /// cannot answer it: two cases of two enums may share one integer.
    EnumCase {
        /// The case's constant value, in its enum's backing type.
        value: crate::enums::EnumValue,
        /// The enum the case belongs to, fully resolved.
        enum_: QName,
        /// The case's own name.
        case: String,
    },
    /// A value the checker folded at a site that named **no constant of its
    /// own**: a `Foo::class`, an attribute retrieval
    /// (`rule:attributes/structural-retrieval`), a static call settled while
    /// checking. It carries the value for [`ExprInfo::EnumCase`]'s reason
    /// exactly: the answer is inlined at the use site, so there is no storage
    /// a consumer could read it back from, and the [`ConstArg`] here is the
    /// same shape a parameter default already lowers through.
    ///
    /// A written `Class::CONST` is [`Self::ClassConst`] instead, which carries
    /// the same value beside the two names the site wrote.
    CoreConst {
        /// The folded value, in the type the site was checked at.
        value: ConstArg,
    },
    /// `Class::CONST` as a program wrote it, keyed by the whole access's own
    /// span.
    ///
    /// `rule:classes/no-free-functions-or-constants`'s class constant, on a
    /// `Core` class and a user-declared one alike — `Core\Math::PI` and
    /// `Limits::MAX` are one shape here, since what the value came from is not
    /// a question a consumer of this entry asks. `crate::signatures::ConstSig`
    /// is where a user constant's value is placed in its declared type; the
    /// one shape that records nothing at all is a value with no constant form
    /// (`public const array<int> ROWS = [1, 2];`).
    ///
    /// The class and the name travel beside the value because the site that
    /// wrote them is the only place they survive: the constant is inlined, so
    /// a consumer reading this back has no declaration in hand to recover them
    /// from. `nvs_lsp::index` is who asks — a read is an occurrence of the
    /// constant it names, and a name is what an occurrence is keyed by. What
    /// it spends is one [`QName`] and one [`String`] per class-constant read,
    /// in the checked unit's table, released with the unit and nothing per
    /// request (`rule:programs/memory-priority`).
    ClassConst {
        /// The class that **declares** the constant, which is where an
        /// inherited one is reached from rather than the class the read wrote
        /// — [`crate::signatures::resolve_const_owned`]'s answer, and
        /// [`ExprInfo::Property`]'s rule for the same question.
        class: QName,
        /// The constant's own name, with no sigil, which is how `C::NAME` is
        /// told from the property `C::$name`.
        name: String,
        /// The constant's value, in its declared type.
        value: ConstArg,
    },
    /// `static::class` and `$obj::class` — a `::class` whose answer is not
    /// known until the call runs, and the one shape that is *not* folded into
    /// an [`Self::CoreConst`] beside it.
    ///
    /// Both spellings are the same question asked of a class descriptor the
    /// frame already holds, which is why they share one entry: `static::class`
    /// reads the frame's late-static-binding class
    /// ([`nvs_ir::lower`'s `Lowering::lsb`](/crates/nvs-ir/src/lower/mod.rs)),
    /// and `$obj::class` reads the receiver's own
    /// ([`nvs_ir::ir::InstKind::ClassDescOf`](/crates/nvs-ir/src/ir.rs)).
    /// `nvs-ir` picks which by matching the class side's own `ExprKind`, so
    /// nothing about *which* descriptor needs recording here — what this entry
    /// carries is the fact that the checker accepted a run-time `::class` at
    /// all, which keeps `nvs-ir`'s folded arm a checker invariant rather than
    /// a guess.
    ///
    /// A `Foo::class`/`self::class`/`parent::class` never reaches this: those
    /// name a class the compiler resolves, so `rule:classes/no-free-functions-or-constants`'s "inlined at every use
    /// site" still holds for them and they stay [`Self::CoreConst`].
    ClassNameOf,
    /// `as property<T>` / `as ?property<T>` over an operand that is **not** a
    /// written-out string —
    /// `rule:types/property-key`'s `string` and `property<U>` rows, with `T`'s public roster already
    /// resolved.
    ///
    /// Recorded because the roster is the one thing the erasure loses. A key's
    /// values are `T`'s public declared property names; `property<T>` erases to
    /// [`nvs_ir`'s `Ty::Str`](/crates/nvs-ir/src/ty.rs) because a key
    /// *is* a name, and `nvs-ir` holds no class table to re-derive the set
    /// from. So the set travels here and § 2's two checked rows lower to the
    /// same compile-time-known membership chain `rule:types/enum-case-type`'s literal union
    /// already does — one `BinOp::Eq` per name, and a throw where every one of
    /// them missed.
    ///
    /// Keyed by the **annotation's** span rather than by the conversion
    /// expression's, the way [`ExprTypeTable::declared_ty`] already is and for
    /// its reason: `nvs-ir` reaches the target through the `Type` node it holds
    /// (`nvs_ir::lower::Lowering::lower_conversion` takes `ty`, not the
    /// expression around it), and the operand's own span keeps whatever entry
    /// the operand itself earned. The two tables are separate maps, so a
    /// declared type and this entry share the span without either shadowing the
    /// other.
    ///
    /// A **written-out** operand records nothing at all: § 2 decides `"email"
    /// as property<User>` where it is written, so it pays nothing at run time
    /// and there is no set for it to be tested against.
    PropertyKey {
        /// The class the key is bounded by, rendered — the throw names it, and
        /// nothing below the erasure could.
        class: String,
        /// `T`'s public declared property names, inherited ones included, in
        /// `crate::expr::members::public_property_names`' own order.
        names: Vec<String>,
    },
    /// `$obj->$key` / `$obj->{$expr}` —
    /// `rule:types/property-key-access`'s keyed access, read or write, whose member name arrives as a
    /// **value** when the statement runs.
    ///
    /// The dynamic-name counterpart of [`ExprInfo::Property`], recorded for
    /// that variant's reason: `nvs_ir::lower` reads a `PropertyAccess` back out
    /// of this table and has no fallback for a span with no entry. Keyed by the
    /// access expression's own span, like every other property entry.
    ///
    /// **Both directions lower to `rule:types/erased-member-access`'s erased access** — § 5's own
    /// decision, recorded in that section: the name is resolved against the
    /// receiver's concrete descriptor at run time, so nothing here carries a
    /// slot to hint with. A key names one of `T`'s public properties and the
    /// receiver satisfies `T`, but *which* name it holds is exactly what is not
    /// known until the access runs. That is why the alternative — a closed-set
    /// chain over the roster, one comparison per name — buys nothing a
    /// descriptor lookup does not already do in one call.
    KeyedProperty {
        /// The class the key is bounded by, rendered: `T` of the operand's
        /// `property<T>`, which the receiver was checked to satisfy.
        class: String,
        /// § 5's union of `T`'s public declared property types — what a read
        /// answers, and what a write must satisfy at least one member of.
        /// [`crate::ty::TypeInterner::mixed`] where `T` declares no
        /// public property at all, a class whose keys have no value.
        ty: TypeId,
    },
    /// `Core\Program::implementing<T>()`, keyed by the call's own span —
    /// `rule:programs/implementing`'s enumeration, already answered.
    ///
    /// The sibling of [`ExprInfo::CoreConst`] for a fold whose answer is not a
    /// constant: § 3 expands the call "to an array literal of `new`
    /// expressions", and a `new` allocates. So what is recorded is the *list*
    /// rather than a value, and `nvs-ir` emits one `InstKind::New` per entry
    /// into one `InstKind::ArrayNew` — exactly the instructions the array
    /// literal a program could have written by hand lowers to, which is what
    /// makes the instances per-request like any other object
    /// (`rule:security/isolate-shares-nothing`).
    ///
    /// Recorded *instead of* [`ExprInfo::Call`] for the same span, for
    /// [`crate::retrieval`]'s reason: one span carries one entry, and
    /// `nvs-ir` would otherwise lower the call it was told to replace.
    /// `nvs_stdlib::program` registers a body that aborts if one ever does.
    ProgramInstances {
        /// Every non-abstract class implementing the written interface, sorted
        /// by fully-qualified name — `nvs_hir::implementors`' answer verbatim,
        /// so the order is the ADR's rather than the filesystem's.
        classes: Vec<QName>,
        /// Each entry's resolved `constructor` label
        /// (`"Owner::constructor"`), or `None` where the class declares none
        /// — the same convention [`ExprInfo::New`]'s `ctor` carries, and
        /// resolved here for its reason: `nvs-ir` cannot re-walk the hierarchy
        /// to find the declaring class. One entry per `classes` entry, in the
        /// same order.
        ctors: Vec<Option<String>>,
    },
    /// `Core\Router::url`/`urlAbsolute`/`urlSigned` over a **literal** name
    /// that resolved to a declared route —
    /// `rule:routing/link-name-and-params-are-checked`'s link,
    /// with the lookup already made.
    ///
    /// Recorded *over* the [`ExprInfo::Call`] the same span already carries,
    /// rather than instead of it, and that is the difference from
    /// [`ExprInfo::ProgramInstances`]: the fold cannot run where the call is
    /// typed, because the route it names may be declared in a file § 5's scan
    /// has not reached yet. So [`crate::links`] records the site during the
    /// walk and resolves it against the finished table afterwards, and the
    /// later recording wins the span (see [`ExprTypeTable::record`]). A
    /// *computed* name records nothing here and keeps its `Call`, which is how
    /// § 4's "a computed `$name` throws" stays true.
    RouteLink {
        /// The named route's declared path, already split by § 2's grammar.
        pieces: Vec<UrlPiece>,
        /// `true` for `urlAbsolute`, which prepends `rule:routing/an-absolute-link-takes-a-configured-origin`'s configured
        /// origin in front of everything `url` builds.
        absolute: bool,
        /// The route's name as the call wrote it, for `urlSigned` alone, and
        /// `None` for the two members that sign nothing.
        ///
        /// The name reaches run time as a value of its own because it is what
        /// the signature is taken over: `rule:core-classes/router-signed-url`
        /// signs a route's *identity* — this name and the parameters — rather
        /// than the path beside it, since one compiled table serves at more
        /// than one mount and the path is the half a remount changes.
        signed: Option<String>,
        /// Every capture whose parameter narrows to a **name**-spelled enum
        /// subset, as the table run time converts its value with — empty for
        /// every other link, which is most of them.
        ///
        /// Here rather than inside a [`UrlPiece`] because it is not a property
        /// of the *path*: [`crate::routes::link_pieces`] reads § 2's grammar and
        /// the spelling comes off the handler's signature, so joining the two in
        /// one variant would make the only reading of that grammar answer a
        /// second question. [`crate::links`]' `spellings` is where they meet.
        spellings: Vec<EnumSpelling>,
    },
    /// An `rule:types/closure-literal`
    /// `fn` closure literal, keyed by the literal's own span.
    ///
    /// A closure's *type* is [`crate::ty::Ty::Callable`] and says nothing
    /// about it — `rule:types/callable-absorbs-closure` keeps that type opaque, and `rule:types/callable-is-a-closure` already
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
        /// under the name `this`, which is `rule:statements/a-closure-binds-this-only-where-it-uses-it`'s "a closure binds
        /// `$this` only where the body uses it" falling straight out of § 2's
        /// capture rule rather than needing a rule of its own.
        captures: Vec<(String, TypeId)>,
        /// The value the body produces — the declared return type, or, for an
        /// expression body with none written, the type inferred from that
        /// expression.
        return_ty: TypeId,
    },
}

/// Which of `rule:iteration/foreach-subjects`'s three subject shapes a `foreach` is walking, and
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

/// What [`crate::intrinsics`]'s fold prepared at one call site, and where the
/// literal it read was written.
///
/// The two halves answer two different readers. `nvs-ir` reads [`Self::fact`]
/// and hands it to the member as `nvs_stdlib::registry::PREPARED_MEMBERS`'
/// constant; a test or a record reads [`Self::literal`] and slices the source
/// at it to name the pattern. Keeping the second span here rather than making
/// it the table's key is what lets the first reader look the entry up by the
/// only address it holds, which is the call's own span.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Prepared {
    /// The span of the literal preparation read — the pattern as it was
    /// written, quotes and escapes included.
    pub literal: Span,
    /// What reading it produced.
    pub fact: PreparedFact,
}

/// One preparation's durable answer — the closed set of facts a call site can
/// carry into the runtime.
///
/// A variant per *answer*, not per grammar row: what a member is handed is the
/// result of reading its literal, and two members reading one pattern language
/// hand over the same kind of thing. The set is closed for
/// `rule:expressions/intrinsic-list-is-closed`'s reason — an answer no roster
/// row produces is an answer no helper can be handed — and
/// `nvs_ir::ir::Prepared` is its twin on the other side of the channel.
///
/// **A fact, never an object.** A compiled regex is an `Rc` on one core's
/// thread-local cache, so what crosses is the tier it settled in and the
/// runtime spends what is left: `nvs_stdlib::registry::PREPARED_MEMBERS` owns
/// that bound in full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreparedFact {
    /// Which of `rule:core-classes/regex-two-tiers`'s two engines a literal
    /// pattern compiles on, settled by the same `nvs_stdlib::regex::validate`
    /// the refusal above it came from.
    RegexTier(nvs_stdlib::regex::Tier),
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
    db_codecs: FxHashMap<String, crate::derive::DerivedCodec>,
    shape_codecs: FxHashMap<Span, crate::derive::DerivedCodec>,
    tests: FxHashMap<String, Vec<crate::testing::TestCase>>,
    fixtures: FxHashMap<String, Vec<crate::testing::Fixture>>,
    inline_snapshots: Vec<crate::testing::InlineSnapshot>,
    property_defaults: FxHashMap<String, Vec<(String, crate::defaults::ConstArg)>>,
    property_default_types: FxHashMap<Span, TypeId>,
    property_types: FxHashMap<String, Vec<(String, TypeId)>>,
    lateinit_properties: FxHashMap<String, Vec<String>>,
    static_properties: FxHashMap<String, Vec<(String, Option<crate::defaults::ConstArg>)>>,
    to_string: FxHashMap<Span, ResolvedCall>,
    require_targets: FxHashMap<Span, nvs_diagnostics::SourceId>,
    prepared: FxHashMap<Span, Prepared>,
    delegations: Vec<Delegation>,
    locals: Vec<(Span, Vec<LocalBinding>)>,
    routes: crate::routes::RouteTable,
    commands: crate::commands::CommandTable,
    callable_values: FxHashMap<Span, TypeId>,
    callable_markers: Vec<(TypeId, String)>,
    callable_conformance: FxHashMap<Span, Vec<String>>,
}

/// One local variable, as the body that declared it left it.
///
/// A local's type is the one thing this crate resolves that it used to keep
/// nowhere a later pass could read: [`crate::locals::LocalScope`] is built per
/// body and dropped with it, and a *read* of a plain variable records no entry
/// of its own. `nvs_lsp::completion` is what needs it — the receiver of
/// `$u->` is a plain read, so without this there is nothing to resolve the
/// members off.
///
/// The alternative was an entry per variable read, which is the same fact
/// keyed the other way and costs a table row at every *occurrence* rather than
/// at every *declaration*; this crate's priority ordering spends
/// simplicity to protect compile latency, not the reverse. Nothing is copied
/// to build one: a scope is moved out of the frame that is being dropped
/// anyway, so what a compile pays is the memory held to the end of the check
/// run instead of to the end of the body — freed with the rest of this table,
/// and attributable to the compile that allocated it.
#[derive(Clone, Debug)]
pub struct LocalBinding {
    /// The name, `$`-sigil not included — [`crate::locals::LocalScope`] keys
    /// it that way and `ExprInfo::Property`'s `name` does too.
    pub name: String,
    /// The type it was declared at, never a narrowing: a narrowing is a fact
    /// about one path and this is the binding.
    pub ty: TypeId,
    /// Where the declaration was written.
    pub declared: Span,
}

/// One synthesized `implements I by $field;` forward —
/// `rule:classes/delegation-by-field`'s "the compiler synthesizes, for every method the interface requires, a
/// one-line forward", resolved here and emitted in `nvs_ir::lower`.
///
/// It is resolved in this crate for the reason every other entry in this table
/// is: which members an interface requires, and which of them the class
/// already answers with a body, are questions about the signature table and
/// the class graph, neither of which `nvs-ir` holds. What rides across is the
/// finished decision — one record per method that needs a forward — so the
/// lowering is a shape with no resolution left in it.
///
/// It is a **method**, not a rewrite at the call site: a receiver typed as the
/// interface (`Timestamped $t = $post;`) dispatches on the runtime class, so
/// the forward has to be a real row in that class's method table or the
/// polymorphism delegation exists for does not work.
#[derive(Clone, Debug)]
pub struct Delegation {
    /// The delegating class's label, rendered as
    /// [`ExprTypeTable::method_label`]'s class half is.
    pub class: String,
    /// The forwarded method's own name.
    pub method: String,
    /// The property the forward reads its receiver out of, `$`-sigil not
    /// included — `by $field`'s own name.
    pub field: String,
    /// The forwarded method's parameter types, positionally. The forward
    /// declares exactly these and passes them straight on, so a call site's
    /// own defaults have already been filled by the time one arrives.
    pub params: Vec<TypeId>,
    /// The forwarded method's declared return type.
    pub return_ty: TypeId,
    /// The `Class::method() at <file>:<line>` backtrace label the forward's
    /// error edge propagates under, rendered here because this is where the
    /// `by $field` clause's span can still be resolved to a line.
    pub frame: String,
    /// The `by $field` clause's own span — the nearest thing a synthesized
    /// forward has to a written position, and all it needs one for is the
    /// conditional edges of the `rule:classes/an-unwritten-property-read-throws` guard
    /// (`nvs_ir::lower::call::delegation_forward`).
    pub span: Span,
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

    /// Takes one finished body's local scope, keyed by the body's own span.
    ///
    /// Called by [`crate::check`] as each frame ends — that is the one moment
    /// the scope is complete, since declaration is function-scoped
    /// (`rule:types/declaration`) and a name declared in the last statement is
    /// a name the whole body has.
    pub fn record_locals(&mut self, body: Span, locals: Vec<LocalBinding>) {
        self.locals.push((body, locals));
    }

    /// Every body's locals, with the span of the body that declared them.
    ///
    /// Iterated rather than looked up: which of the bodies covering an offset
    /// a name belongs to is the consumer's question — a closure's body is
    /// inside a method's and shares none of its bindings
    /// (`rule:types/closure-literal`'s capture is by value), so a reader walks
    /// from the innermost outward and stops at the first body that declares
    /// the name it is after.
    pub fn local_scopes(&self) -> impl Iterator<Item = (Span, &[LocalBinding])> {
        self.locals
            .iter()
            .map(|(body, locals)| (*body, locals.as_slice()))
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

    /// Records that the expression at `span` evaluates to a closure whose
    /// signature the checker interned as `sig` — a `fn` literal, or one of
    /// `rule:types/callable-is-a-closure`'s first-class-callable spellings,
    /// those being the only expressions that make one.
    ///
    /// Keyed by the literal rather than by the class `nvs-ir` synthesizes for
    /// it, because a first-class callable's class is that crate's own name for
    /// a site and never reaches this one. `nvs-ir` is lowering the literal
    /// when it builds the class, so a span is the key both sides hold.
    pub(crate) fn record_callable_value(&mut self, span: Span, sig: TypeId) {
        self.callable_values.insert(span, sig);
    }

    /// Every literal that makes a closure, beside the signature it makes one
    /// of — [`crate::callables`]'s input, and the counterpart of
    /// [`Self::tested_types`].
    pub(crate) fn callable_values(&self) -> impl Iterator<Item = (Span, TypeId)> + '_ {
        self.callable_values.iter().map(|(span, sig)| (*span, *sig))
    }

    /// Every type an `is` in this program asks a value to hold. Iterates for
    /// [`Self::closures`]' reason: the question is about the program rather
    /// than about one site, so there is no span to look up.
    pub(crate) fn tested_types(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.entries.iter().filter_map(|info| match info {
            ExprInfo::TypeTest { tested } => Some(*tested),
            _ => None,
        })
    }

    /// Records [`crate::callables`]' answer for one written signature: the
    /// marker class a test against it walks for, and the literals whose
    /// closures conform to that marker.
    pub(crate) fn record_callable_conformance(
        &mut self,
        sig: TypeId,
        marker: String,
        conformers: &[Span],
    ) {
        for span in conformers {
            self.callable_conformance
                .entry(*span)
                .or_default()
                .push(marker.clone());
        }
        self.callable_markers.push((sig, marker));
    }

    /// The marker class a test against the written signature `sig` walks for,
    /// or `None` for a signature no `is` in this program tested — which is
    /// every signature in a program that writes no such test, the markers
    /// being emitted for the tests that need them and not for every type the
    /// interner holds.
    #[must_use]
    pub fn callable_sig_marker(&self, sig: TypeId) -> Option<&str> {
        self.callable_markers
            .iter()
            .find(|(tested, _)| *tested == sig)
            .map(|(_, marker)| marker.as_str())
    }

    /// Every marker class this program needs a descriptor for, in the order
    /// [`crate::callables`] resolved them.
    pub fn callable_sig_markers(&self) -> impl Iterator<Item = &str> {
        self.callable_markers
            .iter()
            .map(|(_, marker)| marker.as_str())
    }

    /// The marker classes the closure made at `span` conforms to — the
    /// supertypes `nvs-ir` gives that literal's synthesized class, beside the
    /// one every closure carries.
    #[must_use]
    pub fn callable_markers_at(&self, span: Span) -> &[String] {
        self.callable_conformance
            .get(&span)
            .map_or(&[], Vec::as_slice)
    }

    /// Every literal that conforms to at least one marker, beside the markers
    /// it conforms to — the enumerating counterpart of
    /// [`Self::callable_markers_at`], for a consumer with no span to start
    /// from.
    pub fn callable_conformance(&self) -> impl Iterator<Item = (Span, &[String])> {
        self.callable_conformance
            .iter()
            .map(|(span, markers)| (*span, markers.as_slice()))
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

    /// Records the class labelled `label` as carrying `rule:core-classes/derive-attribute`'s derive
    /// attribute for `format`, with the field list [`crate::derive`] read off
    /// its declaration.
    ///
    /// One table per format rather than one keyed by both, because the two are
    /// read by different consumers for different reasons — the JSON half by
    /// `nvs_ir::lower::lower_file`, the row half by the driver work — and a
    /// class carrying both attributes has two independent contracts (§ 3).
    pub(crate) fn record_codec(
        &mut self,
        format: crate::derive::Format,
        label: String,
        codec: crate::derive::DerivedCodec,
    ) {
        match format {
            crate::derive::Format::Json => self.codecs.insert(label, codec),
            crate::derive::Format::Db => self.db_codecs.insert(label, codec),
        };
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

    /// The derived row mapping of the class labelled `label`, or `None` when
    /// it carries no `#[Db\Derive]` — [`Self::codec`]'s counterpart, keyed the
    /// same way and for the same reason.
    ///
    /// Nothing generates `fromRow` from it yet; `crate::derive`'s gap 2 owns
    /// that half.
    #[must_use]
    pub fn db_codec(&self, label: &str) -> Option<&crate::derive::DerivedCodec> {
        self.db_codecs.get(label)
    }

    /// Records the wire contract of the inline shape written at `span`, read
    /// off the type itself by [`crate::derive::shape_codec`].
    ///
    /// Keyed by the span the shape was **written** at rather than by the class
    /// label its fields produce, which is the one thing
    /// [`Self::record_codec`] cannot do here: a shape class is keyed on sorted
    /// field names alone, so `{n: int}` and `{n: string}` share `$shape{n}`
    /// while sharing no wire contract at all. So this is a fact about a call
    /// site, exactly as [`ResolvedCall::written_class`] is, and never one about
    /// a declaration.
    pub(crate) fn record_shape_codec(&mut self, span: Span, codec: crate::derive::DerivedCodec) {
        self.shape_codecs.insert(span, codec);
    }

    /// Every inline shape written on that roster this run, in no particular
    /// order — what [`Self::shape_codec`] answers one call site of.
    ///
    /// `nvs_ir::lower` builds the unit's table of wire contracts off this
    /// rather than off the call sites it lowered: a contract is program-global
    /// and a lowering is per function, so collecting here is what makes two
    /// call sites writing one shape share one table without a merge step. The
    /// order is a hash map's, so the caller that needs a stable one sorts.
    pub fn shape_codecs(&self) -> impl Iterator<Item = &crate::derive::DerivedCodec> {
        self.shape_codecs.values()
    }

    /// The wire contract of the inline shape written at `span`, or `None` where
    /// no call site on `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` wrote one
    /// there — [`ResolvedCall::written_shape`] carries the span to ask with.
    #[must_use]
    pub fn shape_codec(&self, span: Span) -> Option<&crate::derive::DerivedCodec> {
        self.shape_codecs.get(&span)
    }

    /// Records `rule:testing/test-attribute`'s `#[Test]` table for the class labelled `label`,
    /// in declaration order — [`crate::testing::check_class_tests`] read it
    /// off that class's members.
    pub(crate) fn record_tests(&mut self, label: String, cases: Vec<crate::testing::TestCase>) {
        self.tests.insert(label, cases);
    }

    /// The `#[Test]` methods of the class labelled `label`, or `None` when it
    /// declares none.
    ///
    /// Keyed by a class label rather than a span, for [`Self::codec`]'s
    /// reason: the fact is about a *declaration*, and what consumes it is a
    /// test runner that asks "which methods does this class offer", never an
    /// AST node.
    #[must_use]
    pub fn tests(&self, label: &str) -> Option<&[crate::testing::TestCase]> {
        self.tests.get(label).map(Vec::as_slice)
    }

    /// Records one written `Core\Test::assertMatchesInline` — `rule:testing/inline-snapshots`'s
    /// updater material, appended as [`crate::testing::note_inline_snapshot`]
    /// reaches the call.
    ///
    /// A `Vec` and not a map: the key a consumer joins on is the snapshot's
    /// *text*, which is not unique by design — § 14's workflow starts every
    /// snapshot at `""` — so the table keeps every row and the runner is the
    /// one place that decides an ambiguous join is not rewritable.
    pub(crate) fn record_inline_snapshot(&mut self, row: crate::testing::InlineSnapshot) {
        self.inline_snapshots.push(row);
    }

    /// How many § 14 rows have been recorded so far — the mark
    /// [`Self::own_inline_snapshots`] stamps from.
    pub(crate) fn inline_snapshot_mark(&self) -> usize {
        self.inline_snapshots.len()
    }

    /// Names `label` as the owner of every § 14 row recorded at or after
    /// `mark` — `crate::check::check_method`'s one call, made once the body
    /// has been walked.
    ///
    /// Stamped afterwards rather than passed down: the expression walk that
    /// records a row is threaded a `Ctx` that knows the enclosing *class* and
    /// not the enclosing method, and a field for it would be one more thing
    /// every one of that struct's thirteen construction sites has to answer for
    /// a fact only this table wants.
    pub(crate) fn own_inline_snapshots(&mut self, mark: usize, label: String) {
        for row in &mut self.inline_snapshots[mark..] {
            row.owner = Some(label.clone());
        }
    }

    /// Every written `Core\Test::assertMatchesInline` in the program, in the
    /// order the walk reached them — `rule:testing/inline-snapshots`'s `nvs test --update` is the
    /// one consumer, and a program that writes no snapshot has none.
    #[must_use]
    pub fn inline_snapshots(&self) -> &[crate::testing::InlineSnapshot] {
        &self.inline_snapshots
    }

    /// Records `rule:testing/fixtures`'s `#[Fixture]` roster for the class labelled
    /// `label`, in declaration order — [`crate::testing::check_class_tests`]
    /// read it off that class's members, in the same walk that read the
    /// `#[Test]` table above, because the two are one question about one
    /// class and two walks could disagree about which members it has.
    pub(crate) fn record_fixtures(
        &mut self,
        label: String,
        fixtures: Vec<crate::testing::Fixture>,
    ) {
        self.fixtures.insert(label, fixtures);
    }

    /// The `#[Fixture]` methods of the class labelled `label`, or `None` when
    /// it declares none.
    ///
    /// Keyed by a class label for [`Self::tests`]' reason, and separate from
    /// that table because the two rosters answer different questions: which
    /// members are run, and which members supply a value to run them with. A
    /// class may declare either without the other.
    #[must_use]
    pub fn fixtures(&self, label: &str) -> Option<&[crate::testing::Fixture]> {
        self.fixtures.get(label).map(Vec::as_slice)
    }

    /// Records `rule:routing/table-is-opt-in`'s finished route table — every `#[Route]` in the
    /// program, collected across its files and already held to §§ 1-3's
    /// compile errors by [`crate::routes::check_table`].
    ///
    /// Written once, at the end of [`crate::check::check_program`], rather than
    /// row by row as the walk reaches each class: the two errors that are
    /// questions about the whole enumeration are reported over the collected
    /// rows, and a table that could be read back half-built would let a
    /// consumer see a program state no program is ever in.
    pub(crate) fn record_routes(&mut self, routes: crate::routes::RouteTable) {
        self.routes = routes;
    }

    /// `rule:routing/table-is-opt-in`'s route table, empty for a program declaring no `#[Route]`.
    ///
    /// This is the channel the table crosses to `nvs-ir` by, rather than a
    /// second return value on [`crate::check::check_program`], for the reason
    /// every other whole-program fact here crosses the same way: which member a
    /// name resolves to is a question about this crate's tables, and what rides
    /// across is the answer.
    #[must_use]
    pub fn routes(&self) -> &crate::routes::RouteTable {
        &self.routes
    }

    /// Whether this program builds an absolute link at all — `true` exactly
    /// when some `Core\Router::urlAbsolute` over a **literal** route name
    /// resolved against the table above.
    ///
    /// The question is `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s,
    /// asked of a mount's unit before the server binds anything: a program that
    /// links absolutely under a mount that resolves no origin would throw at
    /// the call, and a deployment learns that from a start that fails rather
    /// than from a sent message.
    ///
    /// A **computed** name is not one of these, and cannot be: it records no
    /// [`ExprInfo::RouteLink`] at all (that variant's doc owns why), so its
    /// origin is still a throw at the call. Which is § 3's own wording — the
    /// check is over a literal call — rather than a limit of this walk.
    #[must_use]
    pub fn links_absolutely(&self) -> bool {
        self.entries
            .iter()
            .any(|info| matches!(info, ExprInfo::RouteLink { absolute: true, .. }))
    }

    /// Records `rule:tooling/commands-are-compiled`'s finished command table — every `#[Command]` in
    /// the program, collected across its files and already held to § 6's
    /// duplicate-name error by [`crate::commands::check_table`].
    ///
    /// [`Self::record_routes`]'s arrangement, for its reason: the error that is
    /// a question about the whole enumeration is reported over the collected
    /// rows, so a half-built table must not be readable at all.
    pub(crate) fn record_commands(&mut self, commands: crate::commands::CommandTable) {
        self.commands = commands;
    }

    /// `rule:tooling/commands-are-compiled`'s command table, empty for a program declaring no
    /// `#[Command]` — which is that section's "a program with no `#[Command]`
    /// builds no table" as a consumer sees it.
    #[must_use]
    pub fn commands(&self) -> &crate::commands::CommandTable {
        &self.commands
    }

    /// Every class that declares at least one `#[Test]`, sorted, so that a
    /// consumer walking the whole table does so reproducibly for an unchanged
    /// program rather than in hash order.
    #[must_use]
    pub fn test_classes(&self) -> Vec<&str> {
        let mut labels: Vec<&str> = self.tests.keys().map(String::as_str).collect();
        labels.sort_unstable();
        labels
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

    /// Records the class labelled `label`'s **own** `lateinit` properties
    /// (`rule:classes/lateinit-restrictions`) — [`crate::signatures::ClassSignature::lateinit_properties`],
    /// copied across at check time for [`Self::record_property_defaults`]'
    /// reason exactly.
    ///
    /// `nvs-ir` is the consumer and needs it twice, both for `rule:classes/an-unwritten-property-read-throws`'s
    /// never-written storage state: to arm such a slot with the marker at
    /// construction, and to guard the compiled read of one. Sorted here so
    /// that a lowered program is reproducible for an unchanged file, the
    /// signature's own set being a hash set.
    pub(crate) fn record_lateinit_properties(&mut self, label: String, mut names: Vec<String>) {
        names.sort();
        self.lateinit_properties.insert(label, names);
    }

    /// The class labelled `label`'s own `lateinit` properties, sorted —
    /// empty for the overwhelming majority of classes, which declare none.
    ///
    /// **Own only**, exactly as [`Self::property_defaults`] is: an inherited
    /// one is recorded under the class that declared it.
    #[must_use]
    pub fn lateinit_properties(&self, label: &str) -> &[String] {
        self.lateinit_properties
            .get(label)
            .map_or(&[], Vec::as_slice)
    }

    /// Whether the class labelled `label` declares `$name` `lateinit` itself
    /// — the per-property question [`Self::lateinit_properties`] answers per
    /// class, asked at a resolved property access, whose recorded class is
    /// already the *declaring* one.
    #[must_use]
    pub fn is_lateinit_property(&self, label: &str, name: &str) -> bool {
        self.lateinit_properties(label).iter().any(|p| p == name)
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

    /// Records what [`crate::intrinsics`]'s fold prepared out of the literal a
    /// call at `call` was written with — § 3's second effect settled while
    /// checking, rather than left for the first call to discover.
    ///
    /// [`Self::record_require_target`]'s reason for riding here, with one
    /// addition of its own: what preparation answers is a property of the text
    /// and not of the machine, so recording it once is not a cache that can go
    /// stale between this run and the run that reads it. `nvs-ir` is the only
    /// consumer, and a call with no entry is one whose argument was not a
    /// literal — § 2's rule that nothing is refused for being dynamic applies
    /// to what is *recorded* just as it does to what is refused.
    ///
    /// **Filed under the call, and naming the literal.** The address is the
    /// call's own span because that is what `nvs-ir` holds when it lowers one
    /// (`nvs_stdlib::registry::PREPARED_MEMBERS`); [`Prepared::literal`] is the
    /// pattern's own span, which is what a reader of the record slices the
    /// source at.
    pub(crate) fn record_prepared(&mut self, call: Span, prepared: Prepared) {
        self.prepared.insert(call, prepared);
    }

    /// What the fold prepared for the call at `call`, or `None` for one it did
    /// not read — a dynamic argument, an argument moved out of position by a
    /// `...`, or a literal already refused.
    #[must_use]
    pub fn prepared(&self, call: Span) -> Option<Prepared> {
        self.prepared.get(&call).copied()
    }

    /// Every pattern this run settled a tier for, in no particular order —
    /// for a caller counting them rather than asking about one site.
    pub fn regex_tiers(&self) -> impl Iterator<Item = nvs_stdlib::regex::Tier> + '_ {
        self.prepared.values().map(|prepared| match prepared.fact {
            PreparedFact::RegexTier(tier) => tier,
        })
    }

    /// The same settlements carrying the span each was read from, for a caller
    /// holding the source and wanting to *name* the patterns rather than count
    /// them — the span is the literal's own, so slicing the source at it gives
    /// the pattern back as it was written.
    pub fn regex_tier_sites(&self) -> impl Iterator<Item = (Span, nvs_stdlib::regex::Tier)> + '_ {
        self.prepared.values().map(|prepared| match prepared.fact {
            PreparedFact::RegexTier(tier) => (prepared.literal, tier),
        })
    }

    /// Records one synthesized `by $field` forward — see [`Delegation`], whose
    /// doc comment owns why the decision is taken here and emitted there.
    ///
    /// Keyed by nothing: a forward is a fact about a *declaration* rather than
    /// about an expression, and `nvs-ir` reads the whole list once while it
    /// builds the class table, so a map would only be a slower `Vec`.
    pub(crate) fn record_delegation(&mut self, delegation: Delegation) {
        self.delegations.push(delegation);
    }

    /// Every synthesized `by $field` forward the program owes, in the order
    /// the delegating classes were checked.
    #[must_use]
    pub fn delegations(&self) -> &[Delegation] {
        &self.delegations
    }

    /// The file the `require` at `span` resolved to, or `None` for a path
    /// that is not a literal, names nothing loadable, or closes a cycle —
    /// each of which is already a diagnostic or `rule:statements/require-is-the-only-inclusion-construct`'s dynamic fallback,
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
    /// [`Self::property_defaults`] is — see `nvs_ir::lower`'s `field_slots`,
    /// which is what `rule:types/erased-member-access`'s erased *write* check is built out of: a
    /// name this list does not carry leaves that slot unchecked rather than
    /// mistyped.
    #[must_use]
    pub fn property_types(&self, label: &str) -> &[(String, TypeId)] {
        self.property_types.get(label).map_or(&[], Vec::as_slice)
    }

    /// Records that the property initializer at `span` is written into a
    /// binding declared `ty`. See [`Self::property_default_ty`].
    pub(crate) fn record_property_default_ty(&mut self, span: Span, ty: TypeId) {
        self.property_default_types.insert(span, ty);
    }

    /// The declared type the property initializer at `span` is written into.
    ///
    /// The declaration-side answer to the question [`Self::lookup`] answers at
    /// an *access*: a property declaration is not an access, so it records no
    /// [`ExprInfo`] and there is no entry there to read a qualifier off. An
    /// editor needs one anyway — a `secret` property's default is bytes to
    /// conceal (`rule:ide/redaction-covers-bytes-only`), and it is
    /// written at the one place in a class body that has no expression entry.
    ///
    /// Keyed by the initializer and **not** by the annotation, which is what
    /// keeps it separate from [`Self::declared_ty`]. That map is
    /// `nvs_ir::lower::lower_decl_type`'s first choice, so putting property
    /// annotations into it would change which lowering path a property's type
    /// takes; see [`crate::signatures::SignatureTable::property_default_types`],
    /// which is where these are collected and why they cross the seam on their
    /// own.
    #[must_use]
    pub fn property_default_ty(&self, span: Span) -> Option<TypeId> {
        self.property_default_types.get(&span).copied()
    }

    /// Records how the `foreach` whose subject sits at `span` reaches its
    /// elements. See [`Self::foreach_drive`].
    pub(crate) fn record_foreach(&mut self, span: Span, drive: ForeachDrive) {
        self.foreach.insert(span, drive);
    }

    /// Which of `rule:iteration/foreach-subjects`'s three shapes the `foreach` subject at `span`
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
    /// stringifies through — `rule:classes/stringable`'s implicit conversion, resolved
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
    /// resolution. An enum name is the first such atom: `rule:enums/closed-integer-type` makes
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

    /// A `Core` class a value can be an instance of is a written class name
    /// like any other: it resolves through the same namespace and import rules
    /// and is recorded in the same entry, so `nvs-ir` reads one shape and
    /// `nvs-codegen` relocates against the descriptor
    /// `nvs_stdlib::class_descriptors` publishes.
    #[test]
    fn an_instanceof_records_a_core_class_the_same_way() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m(mixed $v): bool {\n    return $v instanceof Core\\Time\\Date;\n  }\n}\n",
        );
        let Some(ExprInfo::InstanceOf { class }) = exprs.lookup(span) else {
            panic!("expected a recorded `InstanceOf` entry");
        };
        assert_eq!(class.to_string(), r"Core\Time\Date");
    }

    /// The dynamic form records nothing *and* is refused where it is written:
    /// `rule:types/conversion` has no dynamic class names, so there is no entry for
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

    /// `rule:types/callable-is-a-closure`'s first-class callable syntax names the member rather than
    /// calling it, so the same resolved facts are recorded under a variant a
    /// consumer cannot mistake for a call — see [`ExprInfo::CallableRef`].
    #[test]
    fn a_static_first_class_callable_records_the_resolved_target() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  static function make(int $x): int { return $x; }\n  function m(): callable {\n    return self::make(...);\n  }\n}\n",
        );
        let Some(ExprInfo::CallableRef(call)) = exprs.lookup(span) else {
            panic!("expected a recorded `CallableRef` entry");
        };
        assert_eq!(call.class.to_string(), "T");
        assert_eq!(call.method, "make");
        assert_eq!(call.param_tys.len(), 1);
        assert!(call.is_static);
        // `self::` forwards the caller's called class rather than setting one,
        // exactly as `self::make()` does — see `ResolvedCall::static_class`.
        assert!(call.static_class.is_none());
        // `(...)` is a sentinel, not an argument list: there is nothing to map.
        assert!(call.arg_slots.is_empty());
    }

    /// The other half of the same rule: an explicitly named class *sets* the
    /// called class, so `rule:types/callable-is-a-closure`'s "late-bound, exactly like
    /// `static::class`" survives the reference.
    #[test]
    fn a_named_class_first_class_callable_records_its_static_class() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  static function make(): int { return 1; }\n  function m(): callable {\n    return T::make(...);\n  }\n}\n",
        );
        let Some(ExprInfo::CallableRef(call)) = exprs.lookup(span) else {
            panic!("expected a recorded `CallableRef` entry");
        };
        assert_eq!(
            call.static_class.as_ref().map(ToString::to_string),
            Some("T".to_owned())
        );
    }

    #[test]
    fn an_instance_first_class_callable_records_the_resolved_target() {
        let (exprs, span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function a(int $x): int { return $x; }\n  function m(): callable {\n    return $this->a(...);\n  }\n}\n",
        );
        let Some(ExprInfo::CallableRef(call)) = exprs.lookup(span) else {
            panic!("expected a recorded `CallableRef` entry");
        };
        assert_eq!(call.class.to_string(), "T");
        assert_eq!(call.method, "a");
        assert!(!call.is_static);
    }

    /// `rule:types/type-test`'s `is callable(int): string` is answered by a
    /// marker class the tested signature names and the closures that conform
    /// to it, and the relation is `crate::expr::is_assignable`: this literal
    /// declares a *wider* parameter than the test asks for, which is the
    /// direction a callable conversion makes sound, so it conforms.
    #[test]
    fn a_closure_conforms_to_the_marker_of_every_signature_it_satisfies() {
        let (exprs, _span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m(mixed $v): bool {\n    return $v is callable(int): string;\n  }\n  function f(): callable {\n    return fn (mixed $n): string => \"x\";\n  }\n}\n",
        );
        assert_eq!(
            exprs.callable_sig_markers().collect::<Vec<_>>(),
            ["$callable(int): string"]
        );
        let conformance = exprs.callable_conformance().collect::<Vec<_>>();
        assert_eq!(conformance.len(), 1);
        assert_eq!(conformance[0].1, ["$callable(int): string"]);
    }

    /// The other half of the same relation, and why the marker is recorded
    /// whether or not anything satisfies it: a closure returning the wrong
    /// type conforms to nothing, and the test still needs a descriptor to walk
    /// before it can answer `false`.
    #[test]
    fn a_closure_whose_return_type_differs_conforms_to_no_marker() {
        let (exprs, _span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m(mixed $v): bool {\n    return $v is callable(int): string;\n  }\n  function f(): callable {\n    return fn (int $n): int => $n;\n  }\n}\n",
        );
        assert_eq!(
            exprs.callable_sig_markers().collect::<Vec<_>>(),
            ["$callable(int): string"]
        );
        assert_eq!(exprs.callable_conformance().count(), 0);
    }

    /// A closure carries its callee with it, so the one receiver `rule:types/erased-member-access`
    /// defers to run time has nothing to defer *to* — it is refused where it is
    /// written and records nothing, which is what makes "no entry on a
    /// first-class-callable span" mean "this program did not compile".
    #[test]
    fn a_first_class_callable_on_a_mixed_receiver_records_nothing_and_is_refused() {
        let (exprs, span, diags) = check_fixture(
            "<?nvs\nclass T {\n  function m(mixed $o): callable {\n    return $o->a(...);\n  }\n}\n",
        );
        assert!(exprs.lookup(span).is_none());
        assert!(
            diags
                .iter()
                .any(|d| d.code
                    == Some(nvs_diagnostics::code::E_FIRST_CLASS_CALLABLE_ERASED_RECEIVER)),
            "{diags:?}"
        );
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

    /// `rule:classes/property-hooks`: a property that declares a hook records
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
    /// layout: `rule:types/erased-member-access`'s field read resolves to the field's position in
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

    /// `rule:types/shape-type`'s optional field under a `??`: absence has an
    /// answer there, so the read is marked guarded and the entry says so. The
    /// span the helper hands back is the whole `??`, so the access is found by
    /// its variant instead.
    #[test]
    fn a_guarded_read_of_an_optional_field_records_it_as_guarded() {
        let (exprs, _span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m({a?: int} $p): int {\n    return $p->a ?? 0;\n  }\n}\n",
        );
        let entry = exprs
            .entries
            .iter()
            .find(|e| matches!(e, ExprInfo::ShapeProperty { .. }))
            .expect("expected a recorded `ShapeProperty` entry");
        let ExprInfo::ShapeProperty { name, guarded, .. } = entry else {
            unreachable!("filtered to that variant")
        };
        assert_eq!((name.as_str(), *guarded), ("a", true));
    }

    /// The other half, and the one that says the bit is about *optionality*
    /// rather than about the `??`: a required field is proven present, so the
    /// same spelling over one leaves the read unguarded and `nvs-ir` keeps the
    /// throwing fetch.
    #[test]
    fn a_read_of_a_required_field_under_a_coalesce_is_not_guarded() {
        let (exprs, _span) = check_and_find_expr_span(
            "<?nvs\nclass T {\n  function m({a: int} $p): int {\n    return $p->a ?? 0;\n  }\n}\n",
        );
        assert!(
            exprs
                .entries
                .iter()
                .any(|e| matches!(e, ExprInfo::ShapeProperty { guarded: false, .. })),
            "a required field's read must not be marked guarded"
        );
    }

    /// A name the shape does not list is erased exactly like a plain `object`
    /// receiver: `rule:types/erased-member-access` answers `mixed` and hints no slot, since whether
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

    /// A plain `object`-typed receiver erases per `rule:types/erased-member-access` — there is no
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

    /// An array subscript through a `mixed`-erased base is `rule:types/erased-member-access`'s
    /// deferral rather than a refusal: the entry is recorded with a `mixed`
    /// element type, and `nvs-ir` reads the base's *representation* to pick
    /// the tag-asking read (`nvs_ir::Helper::ValueIndexGet`) over the
    /// statically typed one. Nothing is diagnosed, because there is nothing
    /// the declared type could have answered.
    ///
    /// The base is a `mixed` parameter rather than the unannotated class
    /// constant this fixture used to reach for: a constant with no annotation
    /// now reads at the type of the value it folded to
    /// ([`crate::signatures::ConstSig`]), so `T::UNTYPED[0]` over `= 1` is an
    /// `int` subscript and belongs to the test below instead.
    #[test]
    fn an_array_index_through_a_mixed_base_defers_to_the_tag() {
        let (exprs, span, diags) = check_fixture(
            "<?nvs\nclass T {\n  function m(mixed $m): mixed {\n    return $m[0];\n  }\n}\n",
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
