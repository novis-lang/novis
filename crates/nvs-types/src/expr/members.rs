//! A member reference that is not a call: a property access, a class
//! constant, an enum case — and which receiver shapes this checker diagnoses
//! rather than `mwl_hir`.
//!
//! **Diagnosing a missing member is split by receiver, not duplicated.** A
//! `self::`/`static::`/`parent::`/explicit-class-name static call, static
//! property, or class constant is already checked for existence by
//! `mwl_hir::members`, so this module only recovers its *type* there and adds
//! no second diagnostic. A `$this->prop` property access is the same story
//! (`mwl_hir::members` already reports `E_UNDEFINED_PROPERTY` for it). Every
//! other receiver shape — an instance method call regardless of receiver, and
//! a property access on anything but `$this` — has never been checked by
//! `mwl_hir` at all (it has no static type to check against), so this module
//! reports `E_UNKNOWN_MEMBER` for those directly.
//!
//! **A member that exists is then checked for being reachable.** ADR 0094's
//! `private`/`protected` levels are applied by [`check_member_visibility`],
//! keyed on the accessing class rather than on the receiver — every property
//! access lands there, whatever its receiver's spelling, and a method call
//! does not yet.
//!
//! [`check_property_access`]'s shape/`object`/`mixed` arms are ADR 0036 § 4
//! whole: a field a shape names types cleanly with no diagnostic either way,
//! and records the slot `mwl-ir` reads it at
//! ([`crate::expr_table::ExprInfo::ShapeProperty`]); a name it doesn't list,
//! a plain `object` receiver, and a `mixed` one are silently `mixed` rather
//! than `E_UNKNOWN_MEMBER`, and record the same entry carrying the written
//! name alone — which is what ADR 0014 § 5's runtime-checked fallback is
//! keyed on, and it throws now rather than being deferred. A `mixed` is there
//! for ADR 0007 § 2's reason rather than § 4's: it is the one unchecked
//! position, so even "is this an object at all" is deferred to that throw.
//! Every *other* receiver — a scalar, an `array<T>`, a union naming no single
//! class — is `E_RECEIVER_HAS_NO_PROPERTIES` where it is written (ADR 0007
//! § 7 row 13). `unset()`'s operand is narrowed to one shape by
//! [`check_unset_target`], which is that rule's only home: a declared
//! property, static or instance, is refused regardless of nullability
//! (ADR 0028 § 3), and so is every operand that is not an array element of a
//! named holder.
//!
//! [`infer_instanceof`] draws the same line one type earlier, and both sides
//! of the operator are checked. A **subject** whose declared type can hold no
//! object already answered the question, so the test is `E0497` where it is
//! written (ADR 0007 § 7 row 14) while `mixed`, `object`, a shape and any
//! union holding a class keep the run-time test. A **right-hand side** must be
//! a written name the program declares: the dynamic form is ADR 0007 § 2's
//! no-computed-names rule, an enum is a value type (ADR 0010) and a `Core`
//! class has no descriptor laid out for the test to walk, so all three are
//! `E0496` — while a name resolving to nothing is the ordinary `E0303`,
//! exactly as `new Undeclared()` reports it.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `Class::CONST` — [`super::infer`]'s `ExprKind::ClassConstAccess` arm.
///
/// Two shapes are typed precisely, and they split by what the left-hand side
/// names. `EnumName::CaseName` is ADR 0010 § 4's case, recovered as `Ty::Enum`
/// — or, where the position names that one case, as ADR 0047 § 3's narrower
/// `Ty::EnumCase`, the same take-your-type-from-the-position rule
/// `crate::expr::literals` states in full;
/// `Core\Math::PI` is ADR 0011's class constant, recovered as the declared type
/// of the `mwl_stdlib::registry::CoreConst` row. A **user-declared** class's
/// constant is still unmodeled (`mixed`) — see the crate docs' known gaps —
/// because nothing collects one into a signature table to look it up in.
/// `mwl_hir::members` has already checked that every one of the three exists,
/// so this only recovers the type.
#[expect(
    clippy::too_many_arguments,
    reason = "the five-parameter checking context every expression walker in 
              this module carries, plus the constant reference's own two 
              spans and the expectation ADR 0047 § 3 places its case against"
)]
pub(super) fn infer_class_const(
    expr: &Expr,
    class: &Expr,
    name: Span,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_expr(class, None, live, scope, ctx, env);
    let qname = resolve_class_expr(class, ctx, env);
    // A `Core`-owned enum has no `SymbolKind::Enum` entry — nothing declared it
    // — but it is in the same enum table, seeded from
    // `mwl_stdlib::registry::ENUMS`, so asking that table is the one question
    // that answers both. `crate::enums::seed_core` owns why there is one table
    // rather than two.
    let is_enum = qname.as_ref().is_some_and(|qname| {
        matches!(env.symbols.get(qname), Some(sym) if sym.kind == SymbolKind::Enum)
            || (qname.is_core() && env.enums.get(qname).is_some())
    });
    match qname {
        Some(qname) if is_enum => {
            // ADR 0010 § 3: the case *is* its integer constant, so `mwl-ir`
            // needs the value, not just the type — see `ExprInfo::EnumCase`. A
            // name `mwl_hir::members` already reported as undeclared records
            // nothing.
            let case = span_text(env.src, name).to_owned();
            if let Some(value) = env.enums.case(&qname, &case) {
                env.exprs.record(expr.span, ExprInfo::EnumCase { value });
            } else if qname.is_core() {
                // One of the three places `Core`'s blanket trust is *narrowed*
                // rather than relied on — [`super::calls::infer_static_call`]
                // does the same for a member name: `mwl_hir::members` waves a
                // `Core\…::Anything` through because nothing declares it, but
                // `mwl_stdlib::registry::ENUMS` states every case a `Core` enum
                // has, so a name that is not one is knowably wrong here. Without
                // this the mistake reaches `mwl-ir` as a `Class::CONST` with no
                // value recorded, which panics.
                report_unknown_member(class.span, &qname, &case, "case", env);
            }
            let backing = env.enums.backing_of(&qname);
            // ADR 0047 § 3: the case's own narrowed type where the position
            // names it, the whole enum everywhere else — the placement rule
            // `crate::expr::literals` applies to a `string`/`int` literal,
            // reached here because § 3's atom is a *case*, not a literal of
            // its backing value. `ExprInfo::EnumCase` is recorded either way,
            // so `mwl-ir` sees the same integer constant it always did.
            let placed = placed_literal(
                expected,
                env.interner,
                |ty| matches!(ty, Ty::EnumCase(q, _, c) if *q == qname && *c == case),
            );
            placed.unwrap_or_else(|| env.interner.enum_(qname, backing))
        }
        // ADR 0011's class constant, on a `Core` class the registry states. The
        // *value* is recorded, not just the type, for exactly ADR 0010 § 3's
        // reason one line above: a constant is inlined at every use site, so
        // `mwl-ir` needs the constant itself and there is no storage to read it
        // from at run time.
        Some(qname) if qname.is_core() => {
            let constant = span_text(env.src, name).to_owned();
            match crate::core_lib::constant(&qname, &constant, env.interner) {
                Some((ty, value)) => {
                    env.exprs.record(expr.span, ExprInfo::CoreConst { value });
                    ty
                }
                None => {
                    // The third narrowing of `Core`'s blanket trust, on the same
                    // terms as the two above: a class the registry *states* is
                    // checked like any other, while one it does not yet know
                    // stays trusted so the rest of the spec can be written in a
                    // fixture before it is implemented (`crate::core_lib`'s own
                    // docs own that rule).
                    if crate::core_lib::is_registered(&qname) {
                        report_unknown_member(class.span, &qname, &constant, "constant", env);
                    }
                    env.interner.mixed()
                }
            }
        }
        _ => env.interner.mixed(),
    }
}

/// `expr instanceof ClassOrExpr` — [`super::infer`]'s `ExprKind::InstanceOf`
/// arm.
///
/// A bare `Foo` on the right is a class name, not a constant read — recorded
/// here so `mwl-ir` never has to resolve one (see
/// `crate::expr_table::ExprInfo::InstanceOf`). Anything else is the dynamic
/// form, which still checks as an ordinary expression and records nothing.
///
/// The three names worth recording are a declared symbol, one of the reserved
/// global exception classes, and one of the reserved global interfaces — the
/// last two have no declaration to find in `env.symbols`, and
/// `crate::layout::build_class_layouts` seeds a descriptor for each so
/// `mwl-codegen` has something to test against.
///
/// Every other spelling is refused where it is written rather than left for
/// `mwl-ir` to find nothing recorded and panic. There are four, and they split
/// by whose rule they break: a name resolving to nothing is the ordinary
/// `E_UNDEFINED_CLASS` (`new Undeclared()` reports exactly that);
/// a `Core` class, an enum and the dynamic `$x instanceof $name` form are
/// `E_INSTANCEOF_NOT_A_CLASS`, whose own doc comment says why each has no test
/// to run; and a left-hand side whose declared type can hold no object is
/// `E_INSTANCEOF_SUBJECT_NOT_OBJECT` (ADR 0007 § 7 row 14).
pub(super) fn infer_instanceof(
    expr: &Expr,
    inner: &Expr,
    class: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let subject = check_expr(inner, None, live, scope, ctx, env);
    if !can_hold_an_object(subject, env.interner) {
        let described = env.interner.describe(subject);
        env.diags.report(
            Diagnostic::error(
                code::E_INSTANCEOF_SUBJECT_NOT_OBJECT,
                format!("`{described}` can never be an object, so `instanceof` cannot ask"),
            )
            .with_primary(inner.span, "this value's type already answers")
            .with_help(
                "declare the subject `mixed`, `object`, or the base class you expect — a \
                 declared scalar, `array<T>` or enum is not a class and never becomes one \
                 (ADR 0007 § 7 row 14)",
            ),
        );
    }
    let ExprKind::ConstFetch(name) = &class.kind else {
        // ADR 0007 § 2: a class name is written, never computed — the line
        // `$$var` and `eval` are already on. Nothing below could resolve one
        // either: `mwl-codegen` bakes a descriptor address in as a constant.
        check_expr(class, None, live, scope, ctx, env);
        env.diags.report(
            Diagnostic::error(
                code::E_INSTANCEOF_NOT_A_CLASS,
                "the right-hand side of `instanceof` must be a written class name",
            )
            .with_primary(class.span, "not a class name")
            .with_help(
                "MWL has no dynamic class names (ADR 0007 § 2, the rule that rejects `$$var` \
                 and `eval`) — write the class, or branch on the names you accept",
            ),
        );
        return env.interner.bool_ty();
    };
    let text = span_text(env.src, name.span);
    let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);
    let declared = env.symbols.get(&qname);
    if matches!(declared, Some(sym) if sym.kind == SymbolKind::Enum) {
        env.diags.report(
            Diagnostic::error(
                code::E_INSTANCEOF_NOT_A_CLASS,
                format!("`{qname}` is an enum, and no value is ever an instance of one"),
            )
            .with_primary(name.span, "an enum is a value type")
            .with_help(
                "ADR 0010 makes an enum case a named integer rather than an object — compare \
                 it with `==`, or `match` on it",
            ),
        );
    } else if declared.is_some()
        || qname.is_reserved_global_class()
        || qname.is_reserved_global_interface()
    {
        env.exprs
            .record(expr.span, ExprInfo::InstanceOf { class: qname });
    } else if qname.is_core() {
        env.diags.report(
            Diagnostic::error(
                code::E_INSTANCEOF_NOT_A_CLASS,
                format!("`{qname}` is a `Core` class and has no descriptor to test against"),
            )
            .with_primary(name.span, "no descriptor for this class")
            .with_help(
                "a `Core` class is a signature in the stdlib registry rather than a declared \
                 class, so nothing has laid one out for `instanceof` to walk",
            ),
        );
    } else {
        // Same wording `check_new_target` gives `new Undeclared()`: one
        // mistake, one code, wherever the name is written.
        env.diags.report(
            Diagnostic::error(
                code::E_UNDEFINED_CLASS,
                format!("`{qname}` is not declared"),
            )
            .with_primary(name.span, "no matching declaration"),
        );
    }
    env.interner.bool_ty()
}

/// Whether a checked type admits an object at run time — `instanceof`'s
/// left-hand side question, and the whole of what
/// `E_INSTANCEOF_SUBJECT_NOT_OBJECT` refuses.
///
/// Deliberately answered by listing the types that *cannot*: a scalar, an
/// `array<T>`, an enum and the literal types that erase to one. Everything
/// else — `mixed`, `object`, a class, a shape, a `callable`, an `Iterable`, a
/// type variable, an intersection — keeps the run-time test, so a type this
/// pass has not thought about is never refused by accident.
fn can_hold_an_object(ty: TypeId, interner: &TypeInterner) -> bool {
    match interner.get(ty) {
        Ty::Null
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::Bytes
        | Ty::TaintedString
        | Ty::TaintedBytes
        | Ty::SecretString
        | Ty::SecretBytes
        | Ty::SecretTaintedString
        | Ty::SecretTaintedBytes
        | Ty::StringLiteral(_)
        | Ty::IntLiteral(_)
        | Ty::Array(_)
        | Ty::Enum(..)
        | Ty::EnumCase(..) => false,
        // `?Box` is `Union([Null, Class])`, so one member admitting an object
        // is what keeps the whole union testable — and `int|string` is the
        // union this refuses.
        Ty::Union(members) => members
            .iter()
            .any(|member| can_hold_an_object(*member, interner)),
        _ => true,
    }
}

/// The class or enum a resolved type names, if it names one at all — the
/// receiver-type question every member-access/call arm below needs answered
/// before it can look anything up in a [`crate::signatures::SignatureTable`].
pub(super) fn class_qname_of(ty: TypeId, interner: &TypeInterner) -> Option<QName> {
    match interner.get(ty) {
        Ty::Class(q, _) | Ty::Enum(q, _) => Some(q.clone()),
        _ => None,
    }
}

/// Whether `object` is exactly the `$this` variable — the one receiver shape
/// `mwl_hir::members` already diagnoses a missing property on, so
/// [`infer`]'s `PropertyAccess` arm must not diagnose it a second time.
pub(crate) fn is_this_receiver(object: &Expr, src: &mwl_diagnostics::SourceFile) -> bool {
    matches!(&object.kind, ExprKind::Variable(span) if span_text(src, *span) == "$this")
}

/// Resolves a `Class::…`-side expression to the class it names, the same way
/// `mwl_hir::members::check_member_ref` does for existence checking:
/// `self`/`static` against the enclosing class, `parent` against its first
/// `extends` link, an explicit name via the same unqualified/qualified/
/// fully-qualified lookup every resolver in this codebase shares. A dynamic
/// class side (a variable, a parenthesized expression, ...) has no statically
/// knowable class and resolves to `None` — callers fall back to `mixed` with
/// no diagnostic, matching `mwl_hir::members`'s own silent skip for the same
/// shape.
pub(super) fn resolve_class_expr(class_expr: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
    match &class_expr.kind {
        ExprKind::SelfExpr | ExprKind::StaticExpr => ctx.current_class.cloned(),
        ExprKind::ParentExpr => {
            let current = ctx.current_class?;
            env.graph.get(current)?.extends.first().cloned()
        }
        ExprKind::ConstFetch(name) => {
            let text = span_text(env.src, name.span);
            Some(mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports))
        }
        _ => None,
    }
}

/// `Foo::class` — [`super::infer`]'s `ExprKind::ClassNameConst` arm.
///
/// The whole construct is a compile-time constant `string`: PHP resolves the
/// written name against the file's imports and namespace and hands back the
/// fully qualified name, with no runtime step and no requirement that the
/// class be loaded. [`resolve_class_expr`] is the same resolution a static
/// call's class side already gets, and [`mwl_hir::QName`]'s `Display` renders
/// it the way PHP does — `App\User`, no leading `\` — so the value is recorded
/// here rather than left for `mwl-ir` to re-derive from a name it cannot even
/// spell (`mwl-hir` is a dev-dependency there).
///
/// Recording it as [`ExprInfo::CoreConst`] is deliberate reuse rather than a
/// near-miss: that variant means "an ADR 0011 constant, inlined at its use
/// site, whose value is here because there is no storage to read it back
/// from," which is exactly what this is. `mwl-ir` materializes it through the
/// same `emit_const_arg` a parameter default already goes through.
///
/// The class side is **not** checked as a value. Doing so would report
/// `E0319`/`E0321` on every `Foo::class` in the program, for the same reason
/// `mwl_hir::members::walk_class_side` skips the four name-shaped expressions.
/// A side that resolves to nothing is a *dynamic* one — `$obj::class`,
/// `($e)::class` — and is [`code::E_CLASS_NAME_CONST_NOT_STATIC`]: an object
/// carries no name a program can read back. The type stays `string` either
/// way, so a refused site does not then also mismatch its binding.
pub(super) fn check_class_name_const(
    expr: &Expr,
    class: &Expr,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    // `static::class` is the one class side that resolves *and* is wrong to
    // fold. ADR 0008's late static binding makes `static` whichever class the
    // call was made on, so an inherited method's `static::class` is the
    // subclass in PHP and would be the declaring class here — a silently
    // different string rather than a refusal. The name is reachable at run
    // time (the frame carries a `Ty::ClassDesc`), so this is a lowering that
    // does not exist yet rather than a thing the language lacks; until it
    // does, `self::class` is the spelling that means what this folds to.
    if matches!(class.kind, ExprKind::StaticExpr) {
        env.diags.report(
            Diagnostic::error(
                code::E_CLASS_NAME_CONST_NOT_STATIC,
                "`::class` needs a class named at compile time",
            )
            .with_primary(class.span, "`static` is not known until the call runs")
            .with_help(
                "ADR 0008 binds `static` to whichever class the call was made on, so folding \
                 it here would answer the declaring class instead — write `self::class` if \
                 that is what was meant",
            ),
        );
        return env.interner.string();
    }
    match resolve_class_expr(class, ctx, env) {
        Some(qname) => {
            // A *written* name is checked for existing, which is where MWL
            // parts company with PHP: PHP folds `Bogus::class` to `"Bogus"`
            // with no complaint at all, because the string is on its way to
            // `new $name` or `$name::m()` and the question is answered there.
            // MWL has neither spelling (ADR 0011), so a name that resolves to
            // nothing is a typo with nowhere left to be caught — the same
            // mistake and the same code `new Undeclared()` already takes.
            // `self`/`static`/`parent` resolve through the enclosing
            // declaration and are real by construction.
            if matches!(class.kind, ExprKind::ConstFetch(_))
                && env.symbols.get(&qname).is_none()
                && !qname.is_core()
                && !qname.is_reserved_global_class()
                && !qname.is_reserved_global_interface()
            {
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNDEFINED_CLASS,
                        format!("`{qname}` is not declared"),
                    )
                    .with_primary(class.span, "no matching declaration"),
                );
            }
            let value = crate::defaults::ConstArg::Str(qname.to_string());
            env.exprs.record(expr.span, ExprInfo::CoreConst { value });
        }
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CLASS_NAME_CONST_NOT_STATIC,
                    "`::class` needs a class named at compile time",
                )
                .with_primary(class.span, "this names no class the compiler can resolve")
                .with_help(
                    "write the class itself — `Foo::class`, `self::class` — or take the \
                     question to the type system. An object carries no name a program can \
                     read back: ADR 0011 puts every reflective question on `Core\\Reflect`",
                ),
            );
        }
    }
    env.interner.string()
}

/// Shared body for a property access, whether it appears as an ordinary
/// expression (`$obj->prop`, `is_unset` false) or as `unset()`'s operand
/// (`is_unset` true) — the receiver/member resolution is identical either
/// way; only what happens once a *declared* property is found differs (ADR
/// 0028 § 3: `unset()` on one is refused outright, per ADR 0022's guarantee
/// that a declared property can never become uninitialized again).
#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`check_property_member`] states, with the \
              nullsafe flag in place of the receiver type it computes"
)]
pub(super) fn check_property_access(
    object: &Expr,
    property: &MemberName,
    nullsafe: bool,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let object_ty = check_expr(object, None, live, scope, ctx, env);
    // `?->` resolves the property against the receiver's non-`null` half and
    // adds `null` back to the whole access's type — see [`nullsafe_result`],
    // which the method-call arm of [`infer`] shares.
    let receiver_ty = strip_nullsafe_receiver(nullsafe, object_ty, object.span, env);
    let member_ty = check_property_member(
        object,
        receiver_ty,
        property,
        is_unset,
        live,
        scope,
        ctx,
        env,
    );
    nullsafe_result(nullsafe, object_ty, member_ty, env)
}

/// The type a `?->` yields once the member itself has one: the member's own
/// type, plus the `null` the short-circuiting arm answers with.
///
/// A receiver that is not nullable in the first place gains nothing — `?->`
/// on it is exactly `->`, which is also what `mwl-ir` lowers it to. Neither
/// does a `void` member: there is no `?void`, the value is unusable either
/// way, and unioning one would make every `$obj?->doThing();` statement carry
/// a type nothing can consume.
pub(super) fn nullsafe_result(
    nullsafe: bool,
    receiver_ty: TypeId,
    member_ty: TypeId,
    env: &mut Env<'_>,
) -> TypeId {
    if !nullsafe
        || !env.interner.is_nullable(receiver_ty)
        || matches!(env.interner.get(member_ty), Ty::Void)
    {
        return member_ty;
    }
    let null = env.interner.null();
    env.interner.make_union([member_ty, null])
}

/// The half of a `?->` receiver's type that actually reaches the member —
/// everything but `null`. Left alone for `->`, whose receiver reaches the
/// member whole.
pub(super) fn strip_nullsafe_receiver(
    nullsafe: bool,
    object_ty: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    if nullsafe {
        return env.interner.without_null(object_ty);
    }
    // A plain `->` on a receiver that may be `null` is refused rather than
    // resolved against its non-`null` half. Two reasons, and the second is the
    // load-bearing one: PHP throws at run time for exactly this, and
    // `mwl-ir` has no lowering for it at all — `class_qname_of` answers
    // nothing for a union, so no target is recorded and lowering panics naming
    // the span.
    //
    // `if ($m !== null) { $m->text(); }` — which every PHP program writes —
    // does not land here: `crate::locals`' narrowing gives the receiver the
    // class type inside that block, so this sees a resolved class rather than
    // a union. What still lands here is a receiver nothing tested, and one a
    // write inside the block widened again.
    if env.interner.is_nullable(object_ty) && !matches!(env.interner.get(object_ty), Ty::Null) {
        let described = env.interner.describe(object_ty);
        env.diags.report(
            Diagnostic::error(
                code::E_NULLABLE_RECEIVER,
                format!("`{described}` may be `null`, so `->` cannot reach a member of it"),
            )
            .with_primary(span, "this receiver is nullable")
            .with_help(
                "test it first — inside `if ($x != null) { … }` the receiver is no longer \
                 nullable — or use `?->`, which answers `null` instead of reaching the member",
            ),
        );
    }
    object_ty
}

/// [`check_property_access`]'s member half: everything after the receiver's
/// own type is known, so that `?->` and `->` reach it identically.
///
/// # Every access this returns from records an entry, or is refused
///
/// `mwl_ir::lower` reads a `PropertyAccess` back out of
/// [`crate::expr_table`] and has no fallback for a span with no entry — its
/// read arm (`lower_property_access`) and its write arm (`lower_store`'s
/// `PropertyAccess`) both panic there. This is the only function that decides,
/// so the proof that neither panic has a reachable target is here and nowhere
/// else. The split is exhaustive over the two questions an access asks:
///
/// - **The member name.** A computed one (`->$name`, `->{expr}`) never reaches
///   lowering: `mwl_syntax`'s `Parser::parse_member_name` refuses the spelling
///   itself as `E0235`, so the [`MemberName::Ident`] arm below is the only one
///   a compiled program takes. ADR 0014 § 5 owns why.
/// - **The receiver's type.** A [`Ty::Shape`] records [`ExprInfo::ShapeProperty`]
///   with the field's slot; [`Ty::Object`] and [`Ty::Mixed`] record the same
///   variant erased, ADR 0036 § 4's name-keyed half. A type naming a class
///   records [`ExprInfo::Property`] or [`ExprInfo::HookedProperty`] when the
///   name resolves, and is `E_UNKNOWN_MEMBER` when it does not — on **every**
///   class kind, the `Core` namespace and the reserved exception tree
///   included, which is the hole this paragraph was written for. Everything
///   else — a scalar, an `array<T>`, an enum, a union naming no single class —
///   is `E_RECEIVER_HAS_NO_PROPERTIES`, and a nullable receiver is
///   `E_NULLABLE_RECEIVER` on the way in.
///
/// The one return with no entry and no diagnostic of its own is `$this->name`
/// for a name the class does not declare, which `mwl_hir::members` already
/// refused as `E_UNDEFINED_PROPERTY` before this ran.
#[expect(
    clippy::too_many_arguments,
    reason = "the four-part checking context every function in this module \
              threads — live set, scope, ctx, env — plus the receiver, its \
              already-computed type, the member and `unset()`'s flag"
)]
pub(super) fn check_property_member(
    object: &Expr,
    object_ty: TypeId,
    property: &MemberName,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_member_name(property, live, scope, ctx, env);
    let MemberName::Ident(name_span) = property else {
        return env.interner.mixed();
    };
    let name = span_text(env.src, *name_span).to_owned();

    // ADR 0036 § 4, extending ADR 0014 § 5's "a dynamically computed property
    // name is a checked runtime throw, never a fallback" rule to a second
    // trigger: an *erased receiver type*. A field a shape type names is
    // proven present at compile time — reading it never throws, so this just
    // recovers its type, same as any other statically-known access. A name
    // the shape doesn't list, or a plain `object` receiver, is fully erased;
    // whether it exists at runtime isn't a question this compile-time
    // checker can answer either way, so — unlike an ordinary class receiver's
    // `E_UNKNOWN_MEMBER` below — nothing is diagnosed here, and the access
    // answers `mixed`.
    //
    // All four record an `ExprInfo::ShapeProperty`, because § 4 keys the
    // fetch on the **name** in every one of them and the name is the only
    // thing an erased access has. What differs is what rides along: a field
    // the receiver's own shape lists contributes its slot as the hint the
    // runtime tries first and its declared type as the result; an erased one
    // hints slot 0 and answers `mixed`, leaving `mwl_runtime::ClassDesc::
    // field_slot`'s by-name search — and § 4's catchable missing-name throw —
    // as the whole of the resolution.
    //
    // A `mixed` receiver is the fourth, and it is ADR 0007 § 2's one
    // unchecked position rather than a fourth kind of erasure: PHP accepts
    // `$m->name` and so does this, deferring the whole question — is it even
    // an object, and does that object carry this name — to the same run-time,
    // catchable throw. What a `mixed` receiver does *not* share with the
    // other three is a proven tag, so the fetch sees the tagged value itself
    // (`mwl_ir::ir::InstKind::SlotGet`, whose receiver operand is therefore
    // not always a `Ty::Object`).
    match env.interner.get(object_ty).clone() {
        Ty::Shape(fields) => {
            // A field the shape names is proven present, so reading it never
            // throws (ADR 0036 § 4) — but it is *not* proven to be at one
            // slot. The interner sorted this list by name and every producer
            // of a shape value lays its slots out in that same order, so the
            // position resolved here is right exactly where this shape is the
            // value's own exact type. Through a *widened view* — § 3's width
            // subtyping, which is the one way a value's shape and its
            // receiver's differ — it is not, which is why § 4 keys the fetch
            // on the **name** and this records one; the slot rides along as
            // the hint the runtime tries first (`mwl_ir::InstKind::SlotGet`).
            let (slot, ty) = fields
                .iter()
                .position(|(n, _)| *n == name)
                .and_then(|slot| Some((u32::try_from(slot).ok()?, fields[slot].1)))
                .unwrap_or_else(|| (0, env.interner.mixed()));
            env.exprs.record(
                object.span.to(*name_span),
                ExprInfo::ShapeProperty {
                    name: name.clone(),
                    slot,
                    ty,
                },
            );
            return ty;
        }
        Ty::Object | Ty::Mixed => {
            let ty = env.interner.mixed();
            env.exprs.record(
                object.span.to(*name_span),
                ExprInfo::ShapeProperty {
                    name: name.clone(),
                    slot: 0,
                    ty,
                },
            );
            return ty;
        }
        _ => {}
    }

    match class_qname_of(object_ty, env.interner) {
        Some(qname) => match crate::signatures::resolve_property_owned(
            &qname,
            &name,
            env.signatures,
            env.graph,
        ) {
            Some((owner, ty)) => {
                if is_unset {
                    report_unset_on_property(object.span.to(*name_span), &qname, &name, env);
                }
                check_member_visibility(
                    object.span.to(*name_span),
                    &owner,
                    &format!("${name}"),
                    crate::signatures::property_visibility(&owner, &name, env.signatures),
                    ctx,
                    env,
                );
                // ADR 0014 § 1: a hooked property's access is a call to its
                // accessor, not a field touch — except inside that property's
                // own hooks, where `$this->p` is the backing slot (see
                // `Ctx::current_hook`). `is_unset` never reaches here with a
                // hook in play without also having been refused above, so
                // there is no third case.
                let hooks = crate::signatures::hooks_of(&owner, &name, env.signatures);
                let inside_own_hook =
                    ctx.current_hook == Some(name.as_str()) && is_this_receiver(object, env.src);
                if hooks != crate::signatures::PropertyHooks::default() && !inside_own_hook {
                    env.exprs.record(
                        object.span.to(*name_span),
                        ExprInfo::HookedProperty {
                            class: qname.clone(),
                            name: name.clone(),
                            ty,
                            get: hooks.get.then(|| {
                                crate::signatures::hook_label(
                                    &owner,
                                    &name,
                                    mwl_syntax::ast::PropertyHookKind::Get,
                                )
                            }),
                            set: hooks.set.then(|| {
                                crate::signatures::hook_label(
                                    &owner,
                                    &name,
                                    mwl_syntax::ast::PropertyHookKind::Set,
                                )
                            }),
                        },
                    );
                    return ty;
                }
                // `mwl-ir` needs this access's resolved declaring class to
                // lower an eventual field-read instruction — see
                // `crate::expr_table`'s own module docs. The key must match
                // `mwl-ir`'s lookup exactly: `object.span.to(*name_span)` is
                // precisely how the parser built the enclosing
                // `PropertyAccess` expression's own span (see
                // `Parser::parse_new_target_expr`'s `?->`/`->` arm), so
                // there's no need to thread that span through as a separate
                // parameter.
                env.exprs.record(
                    object.span.to(*name_span),
                    ExprInfo::Property {
                        class: qname.clone(),
                        name: name.clone(),
                        ty,
                    },
                );
                ty
            }
            None => {
                // `$this->missing` is already `E_UNDEFINED_PROPERTY`
                // from `mwl_hir::members` — every other receiver
                // shape has never been checked before this.
                //
                // A `Core` class and the reserved exception tree used to be
                // excused here alongside it, and that was the one shape a
                // property write had no refusal in front of: nothing was
                // diagnosed and nothing was recorded, so `mwl_ir::lower`
                // reached a `PropertyAccess` with no table entry and panicked.
                // Neither excuse survives inspection — the exception tree's
                // own properties *are* in `env.signatures` (`$e->message`
                // resolves through this same call), and no `Core` class
                // declares an instance property at all
                // (`mwl_stdlib::registry` is the whole surface, and it is
                // members-only), so `None` on either means exactly what it
                // means on a user class: the name is not declared.
                if !is_this_receiver(object, env.src) {
                    report_unknown_member(object.span, &qname, &name, "property", env);
                }
                env.interner.mixed()
            }
        },
        // Every receiver that neither names a class nor erases to one of the
        // three shapes above: a scalar, an `array<T>`, an enum, a `callable`,
        // or a union naming no single class. PHP warns and yields `null` for
        // the first family and this refuses it instead — ADR 0007 § 7 row 13,
        // which is row 8's rule ("nothing makes an absent thing read as a
        // zero value") at the one storage kind a *declared* type already
        // answers before the program runs. `mixed` is not here: it took the
        // erased arm above, because deferring is what ADR 0007 § 2 makes it
        // for.
        None => {
            // A nullable receiver whose non-`null` half *is* a class already
            // took `E_NULLABLE_RECEIVER` on the way in
            // ([`strip_nullsafe_receiver`]) and is one mistake, not two.
            let non_null = env.interner.without_null(object_ty);
            if class_qname_of(non_null, env.interner).is_none() {
                let described = env.interner.describe(object_ty);
                env.diags.report(
                    Diagnostic::error(
                        code::E_RECEIVER_HAS_NO_PROPERTIES,
                        format!("a property cannot be reached through `{described}`"),
                    )
                    .with_primary(
                        object.span.to(*name_span),
                        "this receiver's type declares no properties",
                    )
                    .with_help(
                        "only an object has properties — convert the receiver to the class you \
                         expect (`$x as Box`), or declare it `mixed`, which is the one unchecked \
                         position (ADR 0007 § 2) and defers the whole question to a catchable \
                         throw at run time",
                    ),
                );
            }
            env.interner.mixed()
        }
    }
}

/// `unset()`'s operand, which ADR 0028 § 3 narrows to exactly one shape:
/// **an array element of a named holder**, `$holder[key]`, where the holder is
/// a local, a property or a static property. This function is that rule's only
/// home, and it splits three ways:
///
/// - A **declared property**, instance or static, is refused as
///   `E0413` ([`report_unset_on_property`]) — ADR 0022 guarantees such a
///   property is definitely initialized for good, and there is no honouring
///   both ADRs at once. The nullsafe spelling is passed through to
///   [`check_property_access`] so the refusal fires on `unset($a?->b)` too,
///   rather than silently resolving to nothing because the receiver's type
///   still carried `null`.
/// - A **subscript** is checked like any other expression, and then through
///   [`check_write_target`], because removing an entry separates a shared
///   array exactly the way writing one does — so `unset($obj->hooked[0])` and
///   `unset($shape->rows[0])` have precisely as little to write the separated
///   copy back into as the assignments those two functions already refuse.
/// - **Everything else** — a bare local, a subscript of a temporary, a literal
///   — is `E0234` ([`report_unset_not_an_element`]).
///
/// Nothing is left over: `mwl_ir::lower::Lowering::lower_unset` lowers the
/// second bullet and panics on anything else, and this is what makes that
/// panic unreachable.
pub(crate) fn check_unset_target(
    expr: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    match &expr.kind {
        ExprKind::PropertyAccess {
            object,
            property,
            nullsafe,
        } => {
            check_property_access(object, property, *nullsafe, true, live, scope, ctx, env);
        }
        ExprKind::StaticPropertyAccess { .. } => {
            check_expr(expr, None, live, scope, ctx, env);
            // The declaring class and the name come back out of the entry the
            // check above recorded, so the message names the slot's own
            // identity — `Base::$count` for a `Sub::$count` written at the
            // access — exactly as `ExprInfo::StaticProperty` defines it.
            if let Some(ExprInfo::StaticProperty { class, name, .. }) = env.exprs.lookup(expr.span)
            {
                let (class, name) = (class.clone(), name.clone());
                report_unset_on_property(expr.span, &class, &name, env);
            }
        }
        ExprKind::Index { .. } => {
            // `unset` is the fourth spelling that writes through a target, so
            // it marks its subscript levels for the same reason the other
            // three do: a hooked or erased holder makes every level above it
            // read as `mixed`, and `E0482` would otherwise blame the subscript
            // for the receiver problem `check_write_target` is about to state
            // properly. `false`, because `unset($a[])` names no element and
            // keeps `E_APPEND_IN_READ_POSITION` — PHP refuses it as *"cannot
            // use [] for unsetting"* and so does this.
            mark_write_target_levels(expr, false, env);
            check_expr(expr, None, live, scope, ctx, env);
            let mut root = expr.unparenthesized();
            while let ExprKind::Index { base, .. } = &root.kind {
                root = base.unparenthesized();
            }
            if is_unset_holder(&root.kind) {
                check_write_target(expr, env);
            } else {
                report_unset_not_an_element(expr, true, env);
            }
        }
        _ => {
            check_expr(expr, None, live, scope, ctx, env);
            report_unset_not_an_element(expr, false, env);
        }
    }
}

/// The three roots `mwl_ir::lower::Lowering::write_back_array` can re-point,
/// which is what makes them the three holders an `unset()` may reach through:
/// ADR 0007 § 5 separates the array before the entry is removed, and the
/// separated copy has to land back in a slot that outlives the statement.
///
/// [`check_write_target`] asks a *different* question of the same root — which
/// of the places it found is refused, rather than whether it found one at all
/// — so the two are deliberately not folded together.
fn is_unset_holder(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Variable(_)
            | ExprKind::PropertyAccess { .. }
            | ExprKind::StaticPropertyAccess { .. }
    )
}

/// `E0234` — an `unset()` operand that is not an array element of a named
/// holder. Both halves are the same rule read from a different side, and
/// `subscripted` picks which one the message says: a subscript that found no
/// holder at its root, or an operand that is not a subscript at all.
fn report_unset_not_an_element(operand: &Expr, subscripted: bool, env: &mut Env<'_>) {
    let (label, help) = if subscripted {
        (
            "nothing holds the array this subscripts",
            "ADR 0007 § 5 separates the array before the entry is removed, so the separated copy \
             needs a local, a property or a static property to be written back into — bind the \
             value first, `unset()` the element there, and use the binding",
        )
    } else {
        (
            "this is not an array element",
            "`unset()` removes an array entry and nothing else — every MWL binding is declared \
             with a type and definitely assigned (ADR 0007 § 1), so there is no way to make one \
             undefined again; assign `null` where the declared type is nullable, or let the \
             binding go out of scope",
        )
    };
    env.diags.report(
        Diagnostic::error(
            code::E_UNSET_TARGET_NOT_AN_ELEMENT,
            "`unset()` takes an array element of a named holder",
        )
        .with_primary(operand.span, label)
        .with_help(help),
    );
}

pub(super) fn report_unset_on_property(span: Span, qname: &QName, name: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNSET_ON_PROPERTY,
            format!(
                "`unset()` on `{qname}::${name}` is refused; a declared property can never \
                 become uninitialized again"
            ),
        )
        .with_primary(span, "unset here")
        .with_help(
            "ADR 0022 already guarantees this property is always definitely initialized; \
             assign `null` instead if it is nullable",
        ),
    );
}

/// Reports `E_UNKNOWN_MEMBER` for a property/method access this module
/// resolved a receiver class for, but found nothing declared under `name` on
/// it or any ancestor.
/// ADR 0094's three levels, enforced: `private` is reachable only from the
/// declaring class's own bodies, `protected` from those and from any class
/// that extends it, `public` from everywhere.
///
/// Keyed on the **accessing** class ([`Ctx::current_class`]) and never on the
/// receiver's static type — `crate::signatures::is_visible_from` states why
/// that distinction is the whole rule. `member` is the reference as it should
/// read in the message (`$count` for a property, `m()` for a method), so the
/// one diagnostic serves both halves of the pass without a kind flag to
/// branch on.
pub(super) fn check_member_visibility(
    span: Span,
    owner: &QName,
    member: &str,
    level: mwl_syntax::ast::Visibility,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    use mwl_syntax::ast::Visibility;

    if crate::signatures::is_visible_from(level, owner, ctx.current_class, env.graph) {
        return;
    }
    let (word, reach) = match level {
        // Unreachable — `is_visible_from` answered `true` above — but written
        // as an arm rather than an `unreachable!` so adding a fourth level is
        // a compile error here instead of a panic in a request.
        Visibility::Public => return,
        Visibility::Private => ("private", format!("only `{owner}`'s own bodies reach it")),
        Visibility::Protected => (
            "protected",
            format!("only `{owner}` and the classes that extend it reach it"),
        ),
    };
    let written = ctx.current_class.map_or_else(
        || "this access is outside any class".to_owned(),
        |class| format!("this access is inside `{class}`"),
    );
    env.diags.report(
        Diagnostic::error(
            code::E_MEMBER_NOT_VISIBLE,
            format!("`{owner}::{member}` is `{word}`, so {reach}"),
        )
        .with_primary(span, written)
        .with_help(format!(
            "widen the declaration to `public`, or reach the value through a member `{owner}` \
             does expose"
        )),
    );
}

pub(super) fn report_unknown_member(
    span: Span,
    qname: &QName,
    name: &str,
    kind: &str,
    env: &mut Env<'_>,
) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no {kind} named `{name}`"),
        )
        .with_primary(span, "referenced here"),
    );
}

/// ADR 0063 R20, at the one place two spellings can reach one `Core` member:
/// an instance member's receiver travels in argument slot 0, so
/// `Core\Regex\Match::text($m)` passes the arity check that `$m->text()`
/// passes and lowers to the identical helper call. Worse, the *zero*-argument
/// spelling passes it too, since a `Core` instance member declares no
/// parameter for its receiver — and that one reaches the helper with an empty
/// argument slice.
///
/// Reported for a `Core` class only. A user-declared class's non-static method
/// called statically is PHP's own error, and belongs with the visibility rules
/// this crate still owes rather than here.
pub(super) fn report_core_instance_member(
    span: Span,
    qname: &QName,
    name: &str,
    env: &mut Env<'_>,
) {
    env.diags.report(
        Diagnostic::error(
            code::E_CORE_INSTANCE_MEMBER_CALLED_STATICALLY,
            format!("`{qname}::{name}` is an instance member, so it is called on a value"),
        )
        .with_primary(span, "called through the class name here")
        .with_help(format!(
            "write `$value->{name}(…)`; ADR 0063 R20 gives every `Core` operation exactly one \
             spelling"
        )),
    );
}

/// Both visibility rules a resolved method call answers to, in the order a
/// reader wants them: ADR 0043 § 3's private-interface-method rule first,
/// because it is the more specific refusal, and ADR 0094's three levels only
/// where that one did not already fire.
///
/// They overlap exactly: a `private` interface method is `Visibility::Private`
/// *and* [`MethodSig::interface_private`], and
/// `signatures::is_visible_from` asks the same question of it that
/// [`check_interface_private_visibility`] does. Reporting both would name one
/// mistake twice, and the ADR 0043 wording is the one that explains it — so
/// that arm returns here rather than falling through.
///
/// `name` is written bare; the `()` that marks it as a method in the message
/// is added here, so no call site has to remember it.
pub(super) fn check_method_visibility(
    owner: &QName,
    name: &str,
    sig: &MethodSig,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    check_interface_private_visibility(owner, name, sig, span, ctx, env);
    if sig.interface_private {
        return;
    }
    check_member_visibility(span, owner, &format!("{name}()"), sig.visibility, ctx, env);
}

/// ADR 0043 § 3: a `private` interface method is an internal helper, never
/// part of that interface's contract — visible only from inside its own
/// declaring interface's method bodies (a default or another private
/// method), never through an implementing class, a subinterface, or any
/// other interface. `owner` is the [`QName`] [`resolve_method`] found `sig`
/// declared on, which may differ from the receiver's own static type when
/// the method was inherited — exactly the case this check cares about.
pub(super) fn check_interface_private_visibility(
    owner: &QName,
    name: &str,
    sig: &MethodSig,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if !sig.interface_private || ctx.current_class == Some(owner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE,
            format!("`{owner}`'s private method `{name}` is not visible here"),
        )
        .with_primary(span, "not part of the interface's contract")
        .with_help(format!(
            "`{name}` is an internal helper of `{owner}` — call it only from `{owner}`'s own \
             method bodies"
        )),
    );
}
