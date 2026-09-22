//! A member reference that is not a call: a property access, a class
//! constant, an enum case — and which receiver shapes this checker diagnoses
//! rather than `nvs_hir`.
//!
//! **Diagnosing a missing member is split by receiver, not duplicated.** A
//! `self::`/`static::`/`parent::`/explicit-class-name static call, static
//! property, or class constant is already checked for existence by
//! `nvs_hir::members`, so this module only recovers its *type* there and adds
//! no second diagnostic. A `$this->prop` property access is the same story
//! (`nvs_hir::members` already reports `E_UNDEFINED_PROPERTY` for it). Every
//! other receiver shape — an instance method call regardless of receiver, and
//! a property access on anything but `$this` — has never been checked by
//! `nvs_hir` at all (it has no static type to check against), so this module
//! reports `E_UNKNOWN_MEMBER` for those directly.
//!
//! **A member that exists is then checked for being reachable.** `rule:core-api/written-visibility`'s
//! `private`/`protected` levels are applied by [`check_member_visibility`],
//! keyed on the accessing class rather than on the receiver — every property
//! access lands there, whatever its receiver's spelling, and a method call
//! does not yet.
//!
//! [`check_property_access`]'s shape/`object`/`mixed` arms are `rule:types/erased-member-access`
//! whole: a field a shape names types cleanly with no diagnostic either way,
//! and records the slot `nvs-ir` reads it at
//! ([`crate::expr_table::ExprInfo::ShapeProperty`]); a name it doesn't list,
//! a plain `object` receiver, and a `mixed` one are silently `mixed` rather
//! than `E_UNKNOWN_MEMBER`, and record the same entry carrying the written
//! name alone — which is what `rule:classes/no-dynamic-properties`'s runtime-checked fallback is
//! keyed on, and it throws now rather than being deferred. A `mixed` is there
//! for `rule:types/conversion`'s reason rather than § 4's: it is the one unchecked
//! position, so even "is this an object at all" is deferred to that throw.
//! Every *other* receiver — a scalar, an `array<T>`, a union naming no single
//! class — is `E_RECEIVER_HAS_NO_PROPERTIES` where it is written (`rule:php-migration/every-divergence-is-deliberate-and-listed`
//! row 13). `unset()`'s operand is narrowed to one shape by
//! [`check_unset_target`], which is that rule's only home: a declared
//! property, static or instance, is refused regardless of nullability
//! (`rule:classes/unset-is-refused-on-a-property`), and so is every operand that is not an array element of a
//! named holder.
//!
//! [`reject_dynamic_class_name`] draws the same line one type earlier, and it
//! lives here because the mistake is one mistake with one code wherever it is
//! written: the three spellings that reach a class through a *value* share it
//! — `$x is $c` ([`super::type_test`]), `new $c()` and `$c::f()` in
//! [`super::calls`]. What each site takes instead of a value is
//! `rule:types/class-reference-sites`'s, and everything else about the type
//! test — what it answers, what it folds, the two things it refuses — is
//! [`super::type_test`]'s.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;
use crate::expr_table::ObserverCalls;

/// `Class::CONST` — [`super::infer`]'s `ExprKind::ClassConstAccess` arm.
///
/// Three shapes are typed precisely, and they split by what the left-hand side
/// names. `EnumName::CaseName` is `rule:statements/an-enum-name-is-a-type-everywhere`'s case, recovered as `Ty::Enum`
/// — or, where the position names that one case, as `rule:types/enum-case-type`'s narrower
/// `Ty::EnumCase`, the same take-your-type-from-the-position rule
/// `crate::expr::literals` states in full;
/// `Core\Math::PI` is `rule:classes/no-free-functions-or-constants`'s class constant, recovered as the declared type
/// of the `nvs_stdlib::registry::CoreConst` row; and `Limits::MAX` on a
/// user-declared class is the same ADR's constant, recovered as the declared
/// type `crate::signatures::ConstSig` recorded for it.
/// `nvs_hir::members` has already checked that every one of the three exists,
/// so this only recovers the type — and, for all three, records the *value*,
/// which is what a constant with no storage behind it leaves `nvs-ir` needing.
#[expect(
    clippy::too_many_arguments,
    reason = "the five-parameter checking context every expression walker in 
              this module carries, plus the constant reference's own two 
              spans and the expectation `rule:types/enum-case-type` places its case against"
)]
pub(crate) fn infer_class_const(
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
    super::reject_class_side_outside_class(class, ctx, env);
    // `$o::VERSION` is the mistake `$o::f()` makes, written on a constant, and
    // it is not one of `rule:types/class-reference-sites`'s three sites: a
    // constant is inlined where it is read, so there is no class to resolve it
    // on at run time and a `class<T>` operand buys nothing here. Left
    // unreported, the arms below record no value and `nvs-ir` panics naming
    // the table it found no entry in — see [`reject_dynamic_class_name`].
    if !is_written_class_side(class) {
        env.diags.report(
            Diagnostic::error(
                code::E_DYNAMIC_CLASS_NAME,
                "the class side of a `::` constant must be a written class name",
            )
            .with_primary(class.span, "not a class name")
            .with_help(
                "a constant is inlined where it is read, so its class is written, never carried in \
                 a value — `App::VERSION`, or `self::VERSION` inside the class; a `class<T>` \
                 reference opens `new`, a `::` call and `is`, not a constant \
                 (`rule:types/class-reference-sites`)",
            ),
        );
        return env.interner.mixed();
    }
    let qname = resolve_class_expr(class, ctx, env);
    // `rule:testing/interaction-after-the-fact`'s method reference. The
    // spelling names no constant, and at the argument positions
    // [`super::calls::note_method_ref_args`] marked — the ones a registry row
    // wrote `nvs_stdlib::registry::CoreTy::MethodRef` at — it names the method
    // instead. Answered here because what a reference *is* is the same answer
    // the three arms below give a constant: a name folded at check time, so
    // `nvs-ir` lowers it through the entry it already reads and needs no arm of
    // its own.
    if env.method_ref_args.contains(&expr.span) {
        return method_reference(expr, class, name, qname.as_ref(), env);
    }
    // A `Core`-owned enum has no `SymbolKind::Enum` entry — nothing declared it
    // — but it is in the same enum table, seeded from
    // `nvs_stdlib::registry::ENUMS`, so asking that table is the one question
    // that answers both. `crate::enums::seed_core` owns why there is one table
    // rather than two.
    let is_enum = qname.as_ref().is_some_and(|qname| {
        matches!(env.symbols.get(qname), Some(sym) if sym.kind == SymbolKind::Enum)
            || (qname.is_core() && env.enums.get(qname).is_some())
    });
    match qname {
        Some(qname) if is_enum => {
            // `rule:enums/no-class-machinery`: the case *is* its integer constant, so `nvs-ir`
            // needs the value, not just the type — see `ExprInfo::EnumCase`. A
            // name `nvs_hir::members` already reported as undeclared records
            // nothing.
            let case = span_text(env.src, name).to_owned();
            if let Some(value) = env.enums.case(&qname, &case) {
                env.exprs.record(
                    expr.span,
                    ExprInfo::EnumCase {
                        value,
                        enum_: qname.clone(),
                        case: case.clone(),
                    },
                );
            } else if qname.is_core() {
                // One of the three places `Core`'s blanket trust is *narrowed*
                // rather than relied on — [`super::calls::infer_static_call`]
                // does the same for a member name: `nvs_hir::members` waves a
                // `Core\…::Anything` through because nothing declares it, but
                // `nvs_stdlib::registry::ENUMS` states every case a `Core` enum
                // has, so a name that is not one is knowably wrong here. Without
                // this the mistake reaches `nvs-ir` as a `Class::CONST` with no
                // value recorded, which panics.
                report_unknown_member(class.span, &qname, &case, "case", env);
            }
            let backing = env.enums.backing_of(&qname);
            // `rule:types/enum-case-type`: the case's own narrowed type where the position
            // names it, the whole enum everywhere else — the placement rule
            // `crate::expr::literals` applies to a `string`/`int` literal,
            // reached here because § 3's atom is a *case*, not a literal of
            // its backing value. `ExprInfo::EnumCase` is recorded either way,
            // so `nvs-ir` sees the same integer constant it always did.
            let placed = placed_literal(
                expected,
                env.interner,
                |ty| matches!(ty, Ty::EnumCase(q, _, c) if *q == qname && *c == case),
            );
            placed.unwrap_or_else(|| env.interner.enum_(qname, backing))
        }
        // `rule:classes/no-free-functions-or-constants`'s class constant, on a `Core` class the registry states. The
        // *value* is recorded, not just the type, for exactly `rule:enums/no-class-machinery`'s
        // reason one line above: a constant is inlined at every use site, so
        // `nvs-ir` needs the constant itself and there is no storage to read it
        // from at run time.
        Some(qname) if qname.is_core() => {
            let constant = span_text(env.src, name).to_owned();
            match crate::core_lib::constant(&qname, &constant, env.interner) {
                Some((ty, value)) => {
                    env.exprs.record(
                        expr.span,
                        ExprInfo::ClassConst {
                            class: qname.clone(),
                            name: constant.clone(),
                            value,
                        },
                    );
                    ty
                }
                None => {
                    // The third narrowing of `Core`'s blanket trust, on the same
                    // terms as the two above — and since the registry is the
                    // whole roster of `Core`, a class it does not hold has no
                    // constant either: `Core\Env::EOL` is this refusal and not
                    // the `nvs-ir` panic it used to be (`crate::core_lib`'s own
                    // docs own that rule).
                    report_unknown_member(class.span, &qname, &constant, "constant", env);
                    env.interner.mixed()
                }
            }
        }
        // `rule:classes/no-free-functions-or-constants`'s class constant on a **user-declared** class, which
        // `crate::signatures` records the declared type and the placed value
        // of — see `signatures::ConstSig`. The value is recorded for the two
        // arms above's reason a third time: a constant is inlined at every use
        // site, so `nvs-ir` has no storage to read it back from.
        Some(qname) => {
            let constant = span_text(env.src, name).to_owned();
            let found = crate::signatures::resolve_const_owned(
                &qname,
                &constant,
                env.signatures,
                env.graph,
            )
            .map(|(owner, sig)| (owner, sig.clone()));
            match found {
                Some((owner, sig)) => {
                    match sig.value {
                        // `static::NAME` is `rule:statements/static-is-a-member-modifier`'s
                        // late-bound read: the class is the frame's called
                        // class, not the one this body is written in, so the
                        // value cannot be inlined here. The declaring class's
                        // own value still travels beside the name — for the
                        // editor, and for `nvs-ir` to type the read — and the
                        // called class's table is what answers at run time.
                        Some(value) if matches!(class.kind, ExprKind::StaticExpr) => {
                            if has_scalar_form(&value) {
                                env.exprs.record(
                                    expr.span,
                                    ExprInfo::ClassConstLate {
                                        class: owner,
                                        name: constant.clone(),
                                        value,
                                    },
                                );
                            } else {
                                report_static_const_without_scalar(
                                    expr, &owner, &constant, sig.ty, env,
                                );
                            }
                        }
                        Some(value) => {
                            env.exprs.record(
                                expr.span,
                                ExprInfo::ClassConst {
                                    class: owner,
                                    name: constant.clone(),
                                    value,
                                },
                            );
                        }
                        // A declaration the constant folder could not reduce.
                        // Refused *here* rather than at the declaration
                        // because a constant nobody names costs nothing and
                        // has no wrong behaviour to report — and refused at
                        // all because `rule:classes/no-free-functions-or-constants` leaves `nvs-ir` nothing to
                        // lower a read to, which was a panic before this.
                        None => {
                            report_unfoldable_const(expr, &qname, &constant, sig.ty, env);
                        }
                    }
                    sig.ty
                }
                // A name no declaration in the chain has — already
                // `nvs_hir::members`' own `E0303`, so this only has to not
                // invent a type for it.
                None => env.interner.mixed(),
            }
        }
        None => env.interner.mixed(),
    }
}

/// `Mailer::send` at an argument position that admits one — the method's own
/// name, checked against the class or interface the reference wrote and folded
/// to a `string` constant there and then.
///
/// The name is checked against the class the *reference* names rather than
/// against the type of the double beside it, which is the check
/// `rule:testing/interaction-after-the-fact` asks for and the one a reader can
/// act on: what a test writes is `Mailer::send`, and what a rename moves is
/// `Mailer`'s own declaration. A reference to a method of some other interface
/// names a call the double cannot have recorded, which the assertion then
/// reports against the record it did keep.
///
/// A class expression that resolved to nothing is left alone: `nvs_hir` has
/// already reported the name, and this only has to not invent a second
/// diagnostic for it.
fn method_reference(
    expr: &Expr,
    class: &Expr,
    name: Span,
    qname: Option<&QName>,
    env: &mut Env<'_>,
) -> TypeId {
    let method = span_text(env.src, name).to_owned();
    let Some(qname) = qname else {
        return env.interner.string();
    };
    if resolve_method(qname, &method, env.signatures, env.graph).is_none() {
        env.diags.report(
            Diagnostic::error(
                code::E_METHOD_REF_UNDECLARED,
                format!("`{qname}` declares no method named `{method}`"),
            )
            .with_primary(class.span, "the method is named here")
            .with_help(
                "`rule:testing/interaction-after-the-fact`: the method an assertion names is a \
                 compile-checked reference, so renaming the method updates or breaks the test \
                 rather than leaving it passing against a method that no longer exists",
            ),
        );
        return env.interner.string();
    }
    // The same `ExprInfo::ClassConst` entry a constant records, holding the
    // method's name as the `string` it lowers to: the reference *is* that name
    // by the time anything runs, and a helper reading it is handed the constant
    // the call site wrote (`nvs_stdlib::registry::CoreTy::MethodRef`).
    env.exprs.record(
        expr.span,
        ExprInfo::ClassConst {
            class: qname.clone(),
            name: method.clone(),
            value: crate::ConstArg::Str(method),
        },
    );
    env.interner.string()
}

/// The `E0792` half of the arm above, which owns why the read and not the
/// declaration is the position.
///
/// The help names the three shapes that reach it, because after
/// [`crate::defaults::eval_const_value`]'s array fold they are the whole of
/// what is left: another class's constant, an enum case, and `Foo::class` —
/// written as the value itself or nested inside a container, since neither
/// position folds. Each is a compile-time constant `rule:attributes/payload-is-a-compile-time-constant` already admits
/// at a *property* default ([`crate::defaults::const_reference_default`]) and
/// none has a `ConstArg` here, so the refusal is a gap named rather than a
/// rule: what closes it is that resolver, one position along, which needs the
/// `Ctx` this table's collection pass does hold.
/// Whether a folded constant is one of the kinds a class descriptor's table
/// carries (`nvs_runtime::ConstantValue`): the four scalars, with `uint`
/// travelling as the `int` bits it is. A `static::` read of anything else has
/// no run-time value to answer with.
fn has_scalar_form(value: &crate::defaults::ConstArg) -> bool {
    use crate::defaults::ConstArg;
    matches!(
        value,
        ConstArg::Str(_)
            | ConstArg::Int(_)
            | ConstArg::Uint(_)
            | ConstArg::Bool(_)
            | ConstArg::Float(_)
    )
}

/// `E0832`: a `static::NAME` whose declaration folds to no scalar — the read
/// the called class's constant table cannot answer. Reported at the read, as
/// [`report_unfoldable_const`] is, because the declaration itself is fine and
/// `self::NAME` still inlines it.
fn report_static_const_without_scalar(
    expr: &Expr,
    owner: &QName,
    constant: &str,
    ty: TypeId,
    env: &mut Env<'_>,
) {
    let declared = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_STATIC_CONST_HAS_NO_SCALAR_VALUE,
            format!("`static::{constant}` reads `{owner}::{constant}`, which is `{declared}` and has no scalar value"),
        )
        .with_primary(expr.span, "read through `static::`")
        .with_help(
            "`rule:statements/static-is-a-member-modifier`: a `static::` constant is read at run time off the \
             called class's table, which carries `string`, `int`, `uint`, `bool` and `float` \
             values and nothing else — write `self::` or the class name, which inline the \
             constant, or make the constant a scalar",
        ),
    );
}

fn report_unfoldable_const(
    expr: &Expr,
    qname: &QName,
    constant: &str,
    declared: TypeId,
    env: &mut Env<'_>,
) {
    // Two declared types name a mistake in the *declaration*, and a read of
    // one is a second diagnostic about it rather than a mistake of its own —
    // "one mistake, one diagnostic", the rule the whole `expr` module reports
    // under. `mixed` is a constant with no written annotation, already `E0246`
    // where it is declared; `bytes` has no literal to write at all (`rule:types/bytes`, and `ConstArg::Bytes`'s own doc), so no value could have folded and
    // the read is not where that is worth saying.
    if declared == env.interner.mixed()
        || matches!(
            env.interner.get(declared),
            Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes
        )
    {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_CLASS_CONST_NO_CONSTANT_FORM,
            format!("`{qname}::{constant}` has no compile-time value to inline"),
        )
        .with_primary(expr.span, "this constant's declaration folds to no value")
        .with_help(
            "`rule:classes/no-free-functions-or-constants` inlines a class constant at every use site, so its value has to have a \
             constant form — a literal, or an array literal of them. Another class's \
             constant, an enum case and `Foo::class` are the three this compiler cannot yet \
             fold into one, written on their own or nested inside a container: write the value \
             out here, or move it to a `static` member the class initializes",
        ),
    );
}

/// Whether a `Core` name is a class a value can be an instance of — the
/// roster a downcast to a `Core` class is held to
/// (`crate::expr::operators`), and the one `nvs-ir` reads to decide that a
/// `Core` name a closure captured has a descriptor to point at.
///
/// Two families answer yes. A registered class with instances
/// ([`crate::core_lib::has_instances`]) carries the descriptor
/// `nvs_stdlib::class_descriptors` publishes for the process, and a namespaced
/// entry of `nvs_hir::errors::TREE` — `Core\Db\DbError`, which a `catch` binds
/// and a test asks about — is laid out into the unit's own table like every
/// other exception class, being under `Core\` by spelling alone.
///
/// A **namespace** class answers no: it declares neither a slot nor an instance
/// member, so nothing is ever an instance of it and the test has no descriptor
/// to walk rather than an answer of `false`.
#[must_use]
pub fn testable_core_class(qname: &QName) -> bool {
    qname.is_core()
        && (crate::core_lib::has_instances(qname)
            || nvs_hir::errors::is_exception_class(&qname.to_string()))
}

/// Whether a tested type names classes alone, none of which a value is ever
/// an instance of — the `false` fold `crate::expr::type_test`'s type arm takes
/// for a subject of any type at all, where the operand rather than the pair
/// settles the answer.
///
/// A `Core` **namespace** class is the whole of it. [`testable_core_class`]
/// admits every other `Core` name, and a declared class lays a descriptor into
/// the unit whether or not the program ever builds one, so `$x is Unbuilt` is
/// an ordinary run-time walk that answers `false` rather than a settled
/// question.
///
/// A union is this only when **every** member is, since a live member still
/// has its own test to run — `$x is int|Core\Str` is the `int` row and nothing
/// else. An intersection is this whenever **any** member is, a value having to
/// be an instance of all of them at once. Every other position — an array
/// element, a shape field — leaves the type inhabited (an empty `array<T>`
/// holds no element to fail), so the settled answer belongs to that position
/// and `nvs_ir`'s own never-matching row is what takes it there.
pub(crate) fn names_no_instance(ty: TypeId, interner: &TypeInterner) -> bool {
    match interner.get(ty) {
        Ty::Class(qname, _) => qname.is_core() && !testable_core_class(qname),
        Ty::Union(members) => members
            .iter()
            .all(|member| names_no_instance(*member, interner)),
        Ty::Intersection(members) => members
            .iter()
            .any(|member| names_no_instance(*member, interner)),
        _ => false,
    }
}

/// Whether a checked type admits an object at run time — the question
/// `crate::expr::type_test`'s value arm folds on, and nothing else asks.
///
/// Deliberately answered by listing the types that *cannot*: a scalar, an
/// `array<T>`, an enum and the literal types that erase to one. Everything
/// else — `mixed`, `object`, a class, a shape, a `callable`, an `Iterable`, a
/// type variable, an intersection — keeps the run-time test, so a type this
/// pass has not thought about is never refused by accident.
///
/// A subject this answers `false` for settles `$x is $cls` at `false` before
/// the program runs, so the test folds there rather than recording a class
/// `nvs-ir` would narrow a slot holding a scalar to.
pub(crate) fn can_hold_an_object(ty: TypeId, interner: &TypeInterner) -> bool {
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

/// `rule:classes/clone-is-shallow`'s operand rule: `clone $x` is "a new instance of `$x`'s
/// class", so a value that can hold no object names nothing to instantiate
/// and `nvs_ir::lower::expr` panics rather than lowering one.
///
/// Asked through [`can_hold_an_object`] for the reason that predicate's own
/// doc gives — only the types that provably cannot are refused, so `mixed`,
/// `object` and a type variable keep the run-time behaviour they have.
pub(crate) fn reject_non_object_clone(ty: TypeId, span: Span, env: &mut Env<'_>) {
    if can_hold_an_object(ty, env.interner) {
        return;
    }
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_CLONE_OPERAND_NOT_AN_OBJECT,
            format!("`clone` makes a new instance of a class, and `{described}` is not an object"),
        )
        .with_primary(span, "this value's type names no class to instantiate")
        .with_help(
            "an `array<T>` and a scalar are already copied when they are assigned (`rule:programs/memory-priority`'s \
             copy-on-write), so there is nothing for `clone` to do — drop it (`rule:classes/clone-is-shallow`)",
        ),
    );
}

/// The same shape for `throw`, over the two ways an operand can fail to be a
/// `Throwable`: a type that can hold no object at all, and a class outside
/// spec § 10's tree.
///
/// The second half is what [`is_throwable_shaped`] answers for `rule:security/secret-sinks-refuse`'s
/// constructor rule, asked here of the thrown value instead. `mixed` and
/// `object` pass both, deliberately: the tree is what `catch` matches on at
/// run time, and refusing a type this pass cannot decide would cost the
/// rethrow shapes that carry one.
pub(crate) fn reject_unthrowable(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let outside_the_tree = match class_qname_of(ty, env.interner) {
        Some(qname) => !is_throwable_shaped(&qname, env.graph),
        None => false,
    };
    if can_hold_an_object(ty, env.interner) && !outside_the_tree {
        return;
    }
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_THROW_OPERAND_NOT_THROWABLE,
            format!("`throw` takes a `Throwable`, and `{described}` is not one"),
        )
        .with_primary(span, "thrown here")
        .with_help(
            "every class a `catch` can match descends from `Throwable` (the standard library's \
             § 10 tree): throw a `RuntimeError`, a `LogicError` or one of your own classes \
             extending one",
        ),
    );
}

/// A `catch` clause or arm naming the class `Core\Script::finish()` raises —
/// `nvs_diagnostics::code::E_CATCH_ARM_NAMES_THE_FINISH_MARKER`.
///
/// Asked of the arm's *written* type rather than of the guarded expression,
/// because the marker never reaches a value position a program can name: it is
/// a root of its own (`nvs_hir::errors::TREE`'s own docs), so the only place a
/// program can mention it at all is the one spelling refused here.
///
/// One class and not a union, because
/// `nvs_diagnostics::code::E_CATCH_UNION_TYPE_UNSUPPORTED` has already refused
/// every clause naming more than one by the time this is asked — so a union
/// reaching here would be a hole in that refusal rather than a shape this one
/// has to read.
pub(crate) fn reject_finish_marker_arm(ty: TypeId, span: Span, env: &mut Env<'_>) {
    if class_qname_of(ty, env.interner) != Some(QName::parse(nvs_hir::errors::FINISH_MARKER)) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_CATCH_ARM_NAMES_THE_FINISH_MARKER,
            format!(
                "`{}` is not a class a `catch` can name",
                nvs_hir::errors::FINISH_MARKER
            ),
        )
        .with_primary(span, "named here")
        .with_help(
            "`Core\\Script::finish()` ends the request rather than failing it, and no `catch` \
             admits the value it unwinds on: put the work in a `finally`, which the unwind runs, \
             or register a `Core\\Script::onExit` hook, which the ending fires",
        ),
    );
}

/// The class or enum a resolved type names, if it names one at all — the
/// receiver-type question every member-access/call arm below needs answered
/// before it can look anything up in a [`crate::signatures::SignatureTable`].
pub(crate) fn class_qname_of(ty: TypeId, interner: &TypeInterner) -> Option<QName> {
    match interner.get(ty) {
        Ty::Class(q, _) | Ty::Enum(q, _) => Some(q.clone()),
        _ => None,
    }
}

/// The class a `class<T>` value names, or `None` for every other type — the
/// question `rule:types/class-reference-sites`'s three sites ask before they fall through to
/// [`reject_dynamic_class_name`].
///
/// The answer is the argument's own [`TypeId`] rather than a [`QName`] because
/// each site wants something different from it: `new $cls()` types itself as
/// `T`, `$cls::f()` resolves a member on `T`'s name, and `$x is $cls`
/// wants neither — the descriptor is the whole test. `T` is a class or an
/// interface by construction, since `crate::lower`'s `lower_class_ref` refuses
/// anything else as `E0795`, so [`class_qname_of`] answers for it.
pub(crate) fn class_ref_argument(ty: TypeId, interner: &TypeInterner) -> Option<TypeId> {
    match interner.get(ty) {
        Ty::ClassRef(inner) => Some(*inner),
        _ => None,
    }
}

/// The class a `property<T>` value's names belong to, or `None` for every
/// other type — [`class_ref_argument`]'s question asked of
/// `rule:types/property-key`'s key, answered the same way and for the same reason: the argument's own
/// [`TypeId`], since the one caller that wants a name asks [`class_qname_of`]
/// for it. `T` is a class by construction, `crate::lower`'s
/// `lower_property_key` having refused everything else as `E0799`.
pub(crate) fn property_key_argument(ty: TypeId, interner: &TypeInterner) -> Option<TypeId> {
    match interner.get(ty) {
        Ty::PropertyKey(inner) => Some(*inner),
        _ => None,
    }
}

/// `rule:types/property-key`'s roster — every property name a `property<T>` value may hold:
/// `T`'s own public declarations and its ancestors', deduplicated and sorted so
/// a diagnostic listing them reads the same way twice.
///
/// The walk is [`crate::signatures::resolve_property_owned`]'s run over a whole
/// class rather than one name, and it chains `implements` for the reason that
/// one does — the two have to agree, or a name this refuses at the conversion
/// would still resolve at the access. An interface declares no properties
/// today, which is why `property<SomeInterface>` is `E0799` at all, so the
/// chain costs nothing now and keeps the two agreeing if that ever changes.
///
/// Visibility is read at the class that *declares* the property, which is what
/// [`crate::signatures::property_visibility`] asks for: the keyword is written
/// there, and a subclass neither adds one nor takes one away.
pub(crate) fn public_property_names(qname: &QName, env: &Env<'_>) -> Vec<String> {
    let mut names = Vec::new();
    collect_public_properties(qname, env, &mut FxHashSet::default(), &mut names);
    names.sort_unstable();
    names.dedup();
    names
}

/// [`public_property_names`]'s recursion, with the `seen` guard that keeps a
/// cyclic `extends` — which the graph may hold, since reporting the cycle is
/// `nvs_hir`'s job and not this walk's — from running forever.
fn collect_public_properties(
    qname: &QName,
    env: &Env<'_>,
    seen: &mut FxHashSet<QName>,
    names: &mut Vec<String>,
) {
    if !seen.insert(qname.clone()) {
        return;
    }
    if let Some(sig) = env.signatures.get(qname) {
        for name in sig.properties.keys() {
            if crate::signatures::property_visibility(qname, name, env.signatures)
                == nvs_syntax::ast::Visibility::Public
            {
                names.push(name.clone());
            }
        }
    }
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    for parent in links.extends.iter().chain(links.implements.iter()) {
        collect_public_properties(parent, env, seen, names);
    }
}

/// `rule:types/property-key-access`: `$obj->$key` and `$obj->{$expr}`, the one computed member
/// name the language admits, and `E0235` for every operand that is not one.
///
/// Two things have to hold, and neither is a spelling. The operand's type must
/// be a `property<T>` — the only door into which is `as`, so the set of names
/// it can hold was checked where it was written — and the receiver must
/// *satisfy* `T`, since a key promises its names against `T`'s roster and a
/// receiver that is not a `T` was never asked about them. § 4's three
/// neighbours are exactly the ways those fail: a `mixed` or shape-typed
/// receiver has no `T` to check against, a call is refused at its own site
/// ([`super::calls::check_member_name`]), and an `unset` is refused below for
/// the reason every declared property already refuses one.
///
/// The read answers § 5's **union of the set**: the key names one of `T`'s
/// public properties and nothing else, so what comes back is one of their
/// declared types and nothing else. The union is taken over `T`'s own roster
/// rather than the receiver's, because `T` is what the key was checked against
/// — a receiver that adds public properties of its own adds no name the key
/// can hold.
///
/// § 5's other refusal — `E0782` where the set holds a `readonly` property —
/// is **not** here, because it applies to a write only and inference sees the
/// same access either way. It is asked off the entry this records, at the
/// place that already knows a target is being written:
/// [`super::assign::check_write_target`].
fn check_keyed_property(
    object_ty: TypeId,
    access_span: Span,
    name_span: Span,
    name_ty: Option<TypeId>,
    is_unset: bool,
    env: &mut Env<'_>,
) -> TypeId {
    let argument = name_ty.and_then(|ty| property_key_argument(ty, env.interner));
    let satisfied = argument.is_some_and(|argument| {
        is_assignable(object_ty, argument, env.interner, env.graph, env.signatures)
    });
    let key_class = satisfied
        .then(|| argument.and_then(|argument| class_qname_of(argument, env.interner)))
        .flatten();
    let Some(key_class) = key_class else {
        report_computed_member_name(name_span, COMPUTED_PROPERTY_HELP, env);
        return env.interner.mixed();
    };
    if is_unset {
        // The same refusal [`report_unset_on_property`] states, over a name
        // that is not written down: every name the key can hold is a declared
        // property of `key_class`, so there is no operand for which this one
        // would be allowed.
        env.diags.report(
            Diagnostic::error(
                code::E_UNSET_ON_PROPERTY,
                format!(
                    "`unset()` through a `property<{key_class}>` key is refused; every name it \
                     can hold is a declared property, and a declared property can never become \
                     uninitialized again"
                ),
            )
            .with_primary(name_span, "unset here")
            .with_help(
                "`rule:classes/definite-property-initialization` already guarantees every one of those properties is definitely \
                 initialized; assign `null` through the key instead where the property is \
                 nullable",
            ),
        );
        return env.interner.mixed();
    }
    let members: Vec<TypeId> = public_property_names(&key_class, env)
        .iter()
        .filter_map(|name| {
            crate::signatures::resolve_property_owned(&key_class, name, env.signatures, env.graph)
                .map(|(_, ty)| ty)
        })
        .collect();
    // A class with no public property at all, which `rule:types/property-key` refuses at the
    // written `property<T>` itself — an empty union is not a type this interner
    // has, and § 2's every conversion into such a key throws, so no value of it
    // can reach an access. The entry is recorded all the same: `nvs-ir` has no
    // fallback for a `PropertyAccess` span with no entry, and "unreachable at
    // run time" is not "never lowered".
    let ty = if members.is_empty() {
        env.interner.mixed()
    } else {
        env.interner.make_union(members)
    };
    env.exprs.record(
        access_span,
        ExprInfo::KeyedProperty {
            class: key_class.to_string(),
            ty,
        },
    );
    ty
}

/// `E0235`, `rule:types/property-key-access`'s refusal, reported where the operand is written.
///
/// The code and the headline are `rule:classes/no-dynamic-properties`'s — what a computed name cannot
/// do is unchanged — and only the *place* moved, from `nvs_syntax`'s parser to
/// here, because § 4 made the operand's type the question and a parser sees no
/// types. The help differs by site, which is why it is the caller's: only a
/// property access has a `property<T>` to suggest.
pub(crate) fn report_computed_member_name(span: Span, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_DYNAMIC_MEMBER_NAME,
            "a member name cannot be computed (`->$name` / `->{expr}`)",
        )
        .with_primary(span, "this names a member only when the statement runs")
        .with_help(help),
    );
}

/// [`report_computed_member_name`]'s help at a property access, where `rule:types/property-key-access` leaves one way to write the same thing.
pub(crate) const COMPUTED_PROPERTY_HELP: &str = "`rule:types/property-key-access` admits `$obj->$key` only where `$key` is a `property<T>` the receiver \
     satisfies — convert the name with `as property<ClassName>`, where the set it may hold is \
     checked, or write the member out; data whose keys are only known at run time belongs in an \
     `array<T>`, whose keys are `string`";

/// [`report_computed_member_name`]'s help at a call, where it does not.
pub(crate) const COMPUTED_METHOD_HELP: &str = "`rule:classes/no-call-magic` refuses a computed *dispatch* itself rather than its spelling, and `rule:types/property-key`'s \
     `property<T>` names a property rather than a method — write the call out, or `match` on the \
     name and call each arm";

/// Whether `object` is exactly the `$this` variable — the one receiver shape
/// `nvs_hir::members` already diagnoses a missing property on, so
/// [`infer`]'s `PropertyAccess` arm must not diagnose it a second time.
pub(crate) fn is_this_receiver(object: &Expr, src: &nvs_diagnostics::SourceFile) -> bool {
    matches!(&object.kind, ExprKind::Variable(span) if span_text(src, *span) == "$this")
}

/// Resolves a `Class::…`-side expression to the class it names, the same way
/// `nvs_hir::members::check_member_ref` does for existence checking:
/// `self`/`static` against the enclosing class, `parent` against its first
/// `extends` link, an explicit name via the same unqualified/qualified/
/// fully-qualified lookup every resolver in this codebase shares. A dynamic
/// class side (a variable, a parenthesized expression, ...) has no statically
/// knowable class and resolves to `None` — callers fall back to `mixed` with
/// no diagnostic, matching `nvs_hir::members`'s own silent skip for the same
/// shape.
pub(crate) fn resolve_class_expr(class_expr: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
    match &class_expr.kind {
        ExprKind::SelfExpr | ExprKind::StaticExpr => ctx.current_class.cloned(),
        ExprKind::ParentExpr => {
            let current = ctx.current_class?;
            env.graph.get(current)?.extends.first().cloned()
        }
        ExprKind::ConstFetch(name) => {
            let text = span_text(env.src, name.span);
            Some(nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports))
        }
        _ => None,
    }
}

/// Whether a `Class::…`-side expression *names* a class rather than computing
/// one — the four spellings [`resolve_class_expr`] resolves.
///
/// Asked instead of testing that resolution for `None`, because that answers
/// `None` for two written spellings as well: `self::` outside any class, and
/// `parent::` in a class with no `extends`. Both are already diagnosed where
/// the name is resolved, and a second report here would name the wrong rule.
pub(crate) fn is_written_class_side(class_expr: &Expr) -> bool {
    matches!(
        &class_expr.kind,
        ExprKind::SelfExpr | ExprKind::StaticExpr | ExprKind::ParentExpr | ExprKind::ConstFetch(_)
    )
}

/// `rule:types/conversion`'s no-computed-names rule, at the four spellings that reach a
/// class through a *value*: `$x is $c` (`crate::expr::type_test`), `new $c()`,
/// `$c::f()` ([`super::calls`]) and `$c::CONST` ([`infer_class_const`]). The
/// headline names the spelling; the label and the help are the rule, which does
/// not vary by site.
///
/// **A `class<T>` operand is not this mistake at the first three.** `rule:types/class-reference-sites`
/// gives those a checked dynamic form, and each asks [`class_ref_argument`]
/// before reaching here; a constant has no such form, since it is inlined where
/// it is read. What is left is a value the checker can resolve to no class at
/// all, so the help names the conversion that turns one into a value it can —
/// the fix an author can take, rather than only the written-out form.
///
/// Nothing below the checker could resolve such a name either — `nvs-codegen`
/// bakes a descriptor's address in as a constant — so this is the last place
/// the mistake can be reported as one. Left unreported, each of the four
/// records nothing in the typed-expression table and `nvs-ir` panics naming
/// the table it found no entry in, which is an internal message for an
/// ordinary mistake.
pub(crate) fn reject_dynamic_class_name(headline: &str, span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_DYNAMIC_CLASS_NAME, headline)
            .with_primary(span, "not a class name")
            .with_help(
                "Novis has no dynamic class names (`rule:types/conversion`, the rule that rejects `$$var` \
                 and `eval`) — write the class, or convert the name once and carry the result: \
                 `$name as class<Base>` yields a class reference this site accepts, checked \
                 against `Base`'s hierarchy where the conversion stands (`rule:types/class-reference-sites`)",
            ),
    );
}

/// `Foo::class` — [`super::infer`]'s `ExprKind::ClassNameConst` arm.
///
/// The whole construct is a compile-time constant `string`: PHP resolves the
/// written name against the file's imports and namespace and hands back the
/// fully qualified name, with no runtime step and no requirement that the
/// class be loaded. [`resolve_class_expr`] is the same resolution a static
/// call's class side already gets, and [`nvs_hir::QName`]'s `Display` renders
/// it the way PHP does — `App\User`, no leading `\` — so the value is recorded
/// here rather than left for `nvs-ir` to re-derive from a name it cannot even
/// spell (`nvs-hir` is a dev-dependency there).
///
/// Recording it as [`ExprInfo::CoreConst`] is deliberate reuse rather than a
/// near-miss: that variant means "an `rule:classes/no-free-functions-or-constants` constant, inlined at its use
/// site, whose value is here because there is no storage to read it back
/// from," which is exactly what this is. `nvs-ir` materializes it through the
/// same `emit_const_arg` a parameter default already goes through.
///
/// The class side is **not** checked as a value. Doing so would report
/// `E0319`/`E0321` on every `Foo::class` in the program, for the same reason
/// `nvs_hir::members::walk_class_side` skips the four name-shaped expressions.
/// A side that resolves to nothing is a *dynamic* one — `$obj::class`,
/// `($e)::class` — and is [`code::E_CLASS_NAME_CONST_NOT_STATIC`]: an object
/// carries no name a program can read back. The type stays `string` either
/// way, so a refused site does not then also mismatch its binding.
pub(crate) fn check_class_name_const(
    expr: &Expr,
    class: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    // `static::class` is the one class side that resolves *and* is wrong to
    // fold: `rule:statements/static-is-a-member-modifier`'s late static binding makes `static` whichever class the
    // call was made on, so an inherited method's `static::class` is the
    // subclass, and folding it would answer the declaring class instead. The
    // frame already holds that class as a `Ty::ClassDesc` — parameter 0 in a
    // `static` method, `$this`'s own descriptor in an instance one — so this
    // records the run-time entry and `nvs-ir` reads the name off it.
    //
    // Whether `static` has a class at all is not this function's question:
    // `resolve_class_expr` answers `None` outside one, and the arm below
    // reports it with the same code every other unresolvable side takes.
    if matches!(class.kind, ExprKind::StaticExpr) && ctx.current_class.is_some() {
        // The frame has to be the method's own: inside a closure body there is
        // no called class to read, and `E0834` says so where it is written.
        super::report_class_keyword_outside_class("static", class.span, ctx, env);
        env.exprs.record(expr.span, ExprInfo::ClassNameOf);
        return env.interner.string();
    }
    // A class side that is not name-shaped is an *expression*, and the only
    // one that carries a class at run time is an object: its descriptor is one
    // load at `nvs_runtime::OBJ_CLASS_OFFSET`, which is the same load
    // `$obj->method()` already makes. Checking it as a value here is safe
    // precisely because it is not name-shaped — the four name-shaped sides
    // return above or resolve below, so the `E0319`/`E0321`-on-every-`Foo::`
    // problem this function's docs describe cannot arise.
    if !matches!(
        class.kind,
        ExprKind::SelfExpr | ExprKind::StaticExpr | ExprKind::ParentExpr | ExprKind::ConstFetch(_)
    ) {
        let recv_ty = check_expr(class, None, live, scope, ctx, env);
        return check_dynamic_class_name_const(class, expr, recv_ty, env);
    }
    match resolve_class_expr(class, ctx, env) {
        Some(qname) => {
            // A *written* name is checked for existing, which is where Novis
            // parts company with PHP: PHP folds `Bogus::class` to `"Bogus"`
            // with no complaint at all, because the string is on its way to
            // `new $name` or `$name::m()` and the question is answered there.
            // Novis has neither spelling (`rule:classes/no-free-functions-or-constants`), so a name that resolves to
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
                env.diags.report(nvs_hir::undeclared_name(
                    nvs_hir::Undeclared {
                        qname: &qname,
                        text: span_text(env.src, class.span),
                        span: class.span,
                        namespace: ctx.namespace,
                        stmts: env.stmts,
                        src: env.src,
                    },
                    env.symbols,
                    &nvs_stdlib::registry::type_names(),
                ));
            }
            let value = crate::defaults::ConstArg::Str(qname.to_string());
            env.exprs.record(expr.span, ExprInfo::CoreConst { value });
        }
        // `self`, `static` or `parent` written outside any class — the only
        // sides left that resolve to nothing, every other shape having been
        // answered above. There is no enclosing declaration for the name to
        // come from and no receiver to read one off.
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CLASS_NAME_CONST_NOT_STATIC,
                    "`::class` needs a class named at compile time",
                )
                .with_primary(class.span, "this names no class the compiler can resolve")
                .with_help(
                    "`self`, `static` and `parent` each name a class through the declaration \
                     they are written in, and there is none here — write the class itself, \
                     `Foo::class`",
                ),
            );
        }
    }
    env.interner.string()
}

/// `$obj::class` and every other class side that is an expression rather than
/// a name — [`check_class_name_const`]'s second half, split out because the
/// accept and the refusal are one `match` over the operand's type and the
/// function above is already long.
///
/// **An object is the only operand that carries a class at run time.** Its
/// descriptor is one load at `nvs_runtime::OBJ_CLASS_OFFSET` and the name is
/// read off that, so `$obj::class` answers the class the receiver *is* rather
/// than the class its variable was declared as — which is PHP's own rule, and
/// the reason it cannot be folded even where the declared type is known: a
/// `User $u = new Admin();` must still answer `Admin`.
///
/// Everything else is [`code::E_CLASS_NAME_CONST_NOT_STATIC`], and the three
/// refusals differ only in what the help points at:
///
/// * a `class<T>` **is** a class reference already, so the name is a
///   conversion rather than a member read — `rule:types/class-reference`'s `class<T>` →
///   `string` row, `$c as string`.
/// * a `mixed` or a union might hold an object and might not. Accepting it
///   would put a tag test and a throw behind a spelling that reads like a
///   field read, so it is refused in favour of narrowing it first — or of
///   `Core\Reflect::forObject`, which is the member whose whole job is the
///   erased receiver (`rule:classes/no-free-functions-or-constants`).
/// * anything else never holds an object at all.
fn check_dynamic_class_name_const(
    class: &Expr,
    expr: &Expr,
    recv_ty: TypeId,
    env: &mut Env<'_>,
) -> TypeId {
    // A `?Foo` reaches the same refusal a `mixed` does, and deliberately: the
    // `null` half carries no descriptor, so the accepted spelling would have
    // to throw on it. `if ($obj !== null)` narrows to the class half and the
    // read below then lowers, which is the same shape `->` already requires of
    // a nullable receiver.
    let erased = env.interner.is_nullable(recv_ty);
    let help = match env.interner.get(recv_ty) {
        Ty::Class(..) | Ty::Object if !erased => {
            env.exprs.record(expr.span, ExprInfo::ClassNameOf);
            return env.interner.string();
        }
        Ty::ClassRef(_) => {
            "a `class<T>` is already a class reference, so its name is a conversion rather \
             than a member read — write `as string`"
        }
        _ if erased => {
            "narrow it to a class first — `if ($v is Foo)`, or a `!= null` test — \
             or ask `Core\\Reflect::forObject($v)`, whose description carries the name for a \
             receiver whose type was erased"
        }
        Ty::Mixed => {
            "narrow it to a class first — `if ($v is Foo)` — or ask \
             `Core\\Reflect::forObject($v)`, whose description carries the name for a \
             receiver whose type was erased"
        }
        _ => {
            "only an object carries a class at run time. Write the class itself — \
             `Foo::class`, `self::class`, `static::class`"
        }
    };
    env.diags.report(
        Diagnostic::error(
            code::E_CLASS_NAME_CONST_NOT_STATIC,
            "`::class` needs a class named at compile time",
        )
        .with_primary(
            class.span,
            format!(
                "this is a `{}`, which names no class",
                env.interner.describe(recv_ty)
            ),
        )
        .with_help(help),
    );
    env.interner.string()
}

/// Shared body for a property access, whether it appears as an ordinary
/// expression (`$obj->prop`, `is_unset` false) or as `unset()`'s operand
/// (`is_unset` true) — the receiver/member resolution is identical either
/// way; only what happens once a *declared* property is found differs (ADR
/// 0028 § 3: `unset()` on one is refused outright, per `rule:classes/definite-property-initialization`'s guarantee
/// that a declared property can never become uninitialized again).
#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`check_property_member`] states, with the \
              nullsafe flag in place of the receiver type it computes"
)]
pub(crate) fn check_property_access(
    access_span: Span,
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
        access_span,
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
/// on it is exactly `->`, which is also what `nvs-ir` lowers it to. Neither
/// does a `void` member: there is no `?void`, the value is unusable either
/// way, and unioning one would make every `$obj?->doThing();` statement carry
/// a type nothing can consume.
pub(crate) fn nullsafe_result(
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
pub(crate) fn strip_nullsafe_receiver(
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
    // `nvs-ir` has no lowering for it at all — `class_qname_of` answers
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

/// `rule:classes/property-observer-pipeline`'s second step for a property access on `qname`, or `None` when
/// that class implements no `PropertyObserver`.
///
/// § 4 is what makes this a compile-time question at all: whether a class
/// implements the interface is read off its declaration, so a class that never
/// asked for the mechanism pays nothing and emits nothing — there is no
/// runtime probe here and no branch below one.
///
/// The two labels are resolved through the class graph rather than spelled
/// from `qname`, because an implementor may inherit either body from a parent,
/// and each is kept only when the resolved declaration *has* one: a bodiless
/// resolution names no compiled function, which is
/// [`ObserverCalls`](crate::expr_table::ObserverCalls)' own convention.
fn observer_calls(qname: &QName, env: &Env<'_>) -> Option<ObserverCalls> {
    let observer = QName::parse(nvs_hir::interfaces::PROPERTY_OBSERVER);
    if !nvs_hir::hierarchy::implements_interface(qname, &observer, env.graph) {
        return None;
    }
    let label = |member: &str| {
        crate::signatures::resolve_method(qname, member, env.signatures, env.graph)
            .filter(|(_, sig)| sig.has_body)
            .map(|(owner, _)| format!("{owner}::{member}"))
    };
    Some(ObserverCalls {
        get: label("onPropertyGet"),
        set: label("onPropertySet"),
    })
}

/// [`check_property_access`]'s member half: everything after the receiver's
/// own type is known, so that `?->` and `->` reach it identically.
///
/// # Every access this returns from records an entry, or is refused
///
/// `nvs_ir::lower` reads a `PropertyAccess` back out of
/// [`crate::expr_table`] and has no fallback for a span with no entry — its
/// read arm (`lower_property_access`) and its write arm (`lower_store`'s
/// `PropertyAccess`) both panic there. This is the only function that decides,
/// so the proof that neither panic has a reachable target is here and nowhere
/// else. The split is exhaustive over the two questions an access asks:
///
/// - **The member name.** A computed one (`->$name`, `->{expr}`) is decided by
///   a *type* rather than by a spelling since `rule:types/property-key-access`, and it is
///   [`check_keyed_property`] that decides: an operand that is not a
///   `property<T>` the receiver satisfies is `E0235` here, and one that is
///   records [`ExprInfo::KeyedProperty`] carrying § 5's union. That arm is
///   recorded even where `T` declares no public property at all — no value of
///   such a key can exist, since § 2's every conversion into one throws, but
///   "unreachable at run time" is not "never lowered" and this proof is about
///   the second. `rule:classes/no-dynamic-properties` owns why every other operand is refused at all.
/// - **The receiver's type.** A [`Ty::Shape`] records [`ExprInfo::ShapeProperty`]
///   with the field's slot; [`Ty::Object`] and [`Ty::Mixed`] record the same
///   variant erased, `rule:types/erased-member-access`'s name-keyed half. A type naming a class
///   records [`ExprInfo::Property`] or [`ExprInfo::HookedProperty`] when the
///   name resolves, and is `E_UNKNOWN_MEMBER` when it does not — on **every**
///   class kind, the `Core` namespace and the reserved exception tree
///   included, which is the hole this paragraph was written for. Everything
///   else — a scalar, an `array<T>`, an enum, a union naming no single class —
///   is `E_RECEIVER_HAS_NO_PROPERTIES`, and a nullable receiver is
///   `E_NULLABLE_RECEIVER` on the way in.
///
/// The one return with no entry and no diagnostic of its own is `$this->name`
/// for a name the class does not declare, which `nvs_hir::members` already
/// refused as `E_UNDEFINED_PROPERTY` before this ran.
#[expect(
    clippy::too_many_arguments,
    reason = "the four-part checking context every function in this module \
              threads — live set, scope, ctx, env — plus the receiver, its \
              already-computed type, the member and `unset()`'s flag"
)]
pub(crate) fn check_property_member(
    access_span: Span,
    object: &Expr,
    object_ty: TypeId,
    property: &MemberName,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let name_ty = check_member_name(property, live, scope, ctx, env);
    let name_span = match property {
        MemberName::Ident(name_span) => name_span,
        MemberName::Variable(e) | MemberName::Expr(e) => {
            return check_keyed_property(object_ty, access_span, e.span, name_ty, is_unset, env);
        }
        _ => return env.interner.mixed(),
    };
    let name = span_text(env.src, *name_span).to_owned();

    // `rule:types/erased-member-access`, extending `rule:classes/no-dynamic-properties`'s "a dynamically computed property
    // name is a checked runtime throw, never a fallback" rule to a second
    // trigger: an *erased receiver type*. A **required** field a shape type
    // names is proven present at compile time — reading it never throws, so
    // this just recovers its type, same as any other statically-known access.
    // An optional one is proven only in type, and the arm below says what a
    // read of it answers guarded and unguarded. A name
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
    // hints slot 0 and answers `mixed`, leaving `nvs_runtime::ClassDesc::
    // field_slot`'s by-name search — and § 4's catchable missing-name throw —
    // as the whole of the resolution. The `guarded` bit rides along on all
    // four and says whether that throw is the answer at all here.
    //
    // A `mixed` receiver is the fourth, and it is `rule:types/conversion`'s one
    // unchecked position rather than a fourth kind of erasure: PHP accepts
    // `$m->name` and so does this, deferring the whole question — is it even
    // an object, and does that object carry this name — to the same run-time,
    // catchable throw. What a `mixed` receiver does *not* share with the
    // other three is a proven tag, so the fetch sees the tagged value itself
    // (`nvs_ir::ir::InstKind::SlotGet`, whose receiver operand is therefore
    // not always a `Ty::Object`).
    match env.interner.get(object_ty).clone() {
        Ty::Shape(fields) => {
            // A **required** field the shape names is proven present, so
            // reading it never throws (`rule:types/erased-member-access`) —
            // but it is *not* proven to be at one slot. The interner sorted
            // this list by name and every producer of a shape value lays its
            // slots out in that same order, so the position resolved here is
            // right exactly where this shape is the value's own exact type.
            // Through a *widened view* — § 3's width subtyping, which is the
            // one way a value's shape and its receiver's differ — it is not,
            // which is why § 4 keys the fetch on the **name** and this records
            // one; the slot rides along as the hint the runtime tries first
            // (`nvs_ir::InstKind::SlotGet`).
            //
            // An **optional** one — `{a?: int}` — is the middle case
            // `rule:types/shape-type` names: the shape proves the type and not
            // the presence, so the read answers the declared `T` and an absent
            // key is that same throw. Under a `??`, an `isset` or an `empty`
            // it is instead a *guarded* read, and there the expression answers
            // `?T` so the `null` it may produce is inside the left operand's
            // own static type — the shape of `ExprInfo::Index`'s guarded
            // subscript, one storage kind along. The `?` never reaches the
            // recorded entry: `ty` is the field's declared type there and the
            // `guarded` bit beside it is what `nvs-ir` reads.
            let resolved = fields
                .iter()
                .position(|field| field.name == name)
                .and_then(|slot| {
                    Some((
                        u32::try_from(slot).ok()?,
                        fields[slot].ty,
                        fields[slot].required,
                    ))
                });
            let (slot, ty, required) = resolved.unwrap_or_else(|| (0, env.interner.mixed(), false));
            let guarded = !required && env.coalesce_guarded.contains(&access_span);
            env.exprs.record(
                object.span.to(*name_span),
                ExprInfo::ShapeProperty {
                    name: name.clone(),
                    slot,
                    ty,
                    guarded,
                },
            );
            if guarded {
                let null = env.interner.null();
                return env.interner.make_union([ty, null]);
            }
            return ty;
        }
        Ty::Object | Ty::Mixed => {
            // An erased receiver promises no field at all, so every name it
            // carries is the optional case above with nothing static to read
            // off it: a guarded access answers `null` for absence rather than
            // throwing, and `mixed` already admits that `null` without being
            // unioned with one.
            let ty = env.interner.mixed();
            let guarded = env.coalesce_guarded.contains(&access_span);
            env.exprs.record(
                object.span.to(*name_span),
                ExprInfo::ShapeProperty {
                    name: name.clone(),
                    slot: 0,
                    ty,
                    guarded,
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
                // `rule:classes/property-hooks`: a hooked property's access is a call to its
                // accessor, not a field touch — except inside that property's
                // own hooks, where `$this->p` is the backing slot (see
                // `Ctx::current_hook`). `is_unset` never reaches here with a
                // hook in play without also having been refused above, so
                // there is no third case.
                let hooks = crate::signatures::hooks_of(&owner, &name, env.signatures);
                let inside_own_hook =
                    ctx.current_hook == Some(name.as_str()) && is_this_receiver(object, env.src);
                // An access inside the property's own hooks is the *backing
                // slot* — which is `rule:classes/property-observer-pipeline`'s first step, the one the
                // access that called this hook is already running the second
                // step for. Observing it here would report one write twice,
                // so this is the one property access on an observing class
                // that carries no observer.
                let observer = (!inside_own_hook)
                    .then(|| observer_calls(&qname, env))
                    .flatten();
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
                                    nvs_syntax::ast::PropertyHookKind::Get,
                                )
                            }),
                            set: hooks.set.then(|| {
                                crate::signatures::hook_label(
                                    &owner,
                                    &name,
                                    nvs_syntax::ast::PropertyHookKind::Set,
                                )
                            }),
                            observer,
                        },
                    );
                    return ty;
                }
                // `nvs-ir` needs this access's resolved declaring class to
                // lower an eventual field-read instruction — see
                // `crate::expr_table`'s own module docs. The key must match
                // `nvs-ir`'s lookup exactly: `object.span.to(*name_span)` is
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
                        observer,
                    },
                );
                ty
            }
            None => {
                // `$this->missing` is already `E_UNDEFINED_PROPERTY`
                // from `nvs_hir::members` — every other receiver
                // shape has never been checked before this.
                //
                // A `Core` class and the reserved exception tree used to be
                // excused here alongside it, and that was the one shape a
                // property write had no refusal in front of: nothing was
                // diagnosed and nothing was recorded, so `nvs_ir::lower`
                // reached a `PropertyAccess` with no table entry and panicked.
                // Neither excuse survives inspection — the exception tree's
                // own properties *are* in `env.signatures` (`$e->message`
                // resolves through this same call), and no `Core` class
                // declares an instance property at all
                // (`nvs_stdlib::registry` is the whole surface, and it is
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
        // the first family and this refuses it instead — `rule:php-migration/every-divergence-is-deliberate-and-listed` row 13,
        // which is row 8's rule ("nothing makes an absent thing read as a
        // zero value") at the one storage kind a *declared* type already
        // answers before the program runs. `mixed` is not here: it took the
        // erased arm above, because deferring is what `rule:types/conversion` makes it
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
                         position (`rule:types/conversion`) and defers the whole question to a catchable \
                         throw at run time",
                    ),
                );
            }
            env.interner.mixed()
        }
    }
}

/// `unset()`'s operand, which `rule:classes/unset-is-refused-on-a-property` narrows to exactly one shape:
/// **an array element of a named holder**, `$holder[key]`, where the holder is
/// a local, a property or a static property. This function is that rule's only
/// home, and it splits three ways:
///
/// - A **declared property**, instance or static, is refused as
///   `E0413` ([`report_unset_on_property`]) — `rule:classes/definite-property-initialization` guarantees such a
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
/// Nothing is left over: `nvs_ir::lower::Lowering::lower_unset` lowers the
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
            check_property_access(
                expr.span, object, property, *nullsafe, true, live, scope, ctx, env,
            );
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
                check_write_target(expr, ctx, env);
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

/// The three roots `nvs_ir::lower::Lowering::write_back_array` can re-point,
/// which is what makes them the three holders an `unset()` may reach through:
/// `rule:types/arrays` separates the array before the entry is removed, and the
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
            "`rule:types/arrays` separates the array before the entry is removed, so the separated copy \
             needs a local, a property or a static property to be written back into — bind the \
             value first, `unset()` the element there, and use the binding",
        )
    } else {
        (
            "this is not an array element",
            "`unset()` removes an array entry and nothing else — every Novis binding is declared \
             with a type and definitely assigned (`rule:types/declaration`), so there is no way to make one \
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

pub(crate) fn report_unset_on_property(span: Span, qname: &QName, name: &str, env: &mut Env<'_>) {
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
            "`rule:classes/definite-property-initialization` already guarantees this property is always definitely initialized; \
             assign `null` instead if it is nullable",
        ),
    );
}

/// Reports `E_UNKNOWN_MEMBER` for a property/method access this module
/// resolved a receiver class for, but found nothing declared under `name` on
/// it or any ancestor.
/// `rule:core-api/written-visibility`'s three levels, enforced: `private` is reachable only from the
/// declaring class's own bodies, `protected` from those and from any class
/// that extends it, `public` from everywhere.
///
/// Keyed on the **accessing** class ([`Ctx::current_class`]) and never on the
/// receiver's static type — `crate::signatures::is_visible_from` states why
/// that distinction is the whole rule. `member` is the reference as it should
/// read in the message (`$count` for a property, `m()` for a method), so the
/// one diagnostic serves both halves of the pass without a kind flag to
/// branch on.
pub(crate) fn check_member_visibility(
    span: Span,
    owner: &QName,
    member: &str,
    level: nvs_syntax::ast::Visibility,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    use nvs_syntax::ast::Visibility;

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

pub(crate) fn report_unknown_member(
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

/// `rule:core-api/shape-rules` R20, at the one place two spellings can reach one `Core` member:
/// an instance member's receiver travels in argument slot 0, so
/// `Core\Regex\Match::text($m)` passes the arity check that `$m->text()`
/// passes and lowers to the identical helper call. Worse, the *zero*-argument
/// spelling passes it too, since a `Core` instance member declares no
/// parameter for its receiver — and that one reaches the helper with an empty
/// argument slice.
///
/// Reported for a `Core` class only — a user-declared class's is
/// [`report_instance_method_called_statically`] beside it, which asks a
/// different question for a different reason.
pub(crate) fn report_core_instance_member(
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
            "write `$value->{name}(…)`; `rule:core-api/shape-rules` R20 gives every `Core` operation exactly one \
             spelling"
        )),
    );
}

/// The same shape for a user-declared class, and the reason it is a separate
/// refusal from [`report_core_instance_member`]: a `Core` instance member is
/// refused *wherever* it is written statically, because `rule:core-api/shape-rules` R20 gives it
/// one spelling and the static one would reach the identical helper. A
/// declared class's non-static method is refused only where the frame holds no
/// `$this` — `rule:statements/static-is-a-member-modifier` keeps PHP's semantics for `static`, so `self::f()`
/// and `parent::f()` inside an instance method are the ordinary forwarding
/// spelling and stay legal.
///
/// Not a diagnostic the checker may skip: `nvs_ir::lower::expr` reads the
/// enclosing frame's `$this` for a non-static target and panics when there is
/// none, naming this function's absence as the cause.
///
/// `rule:types/callable-is-a-closure`'s first-class callable `C::m(...)` is not one of those frames —
/// it records a `CallableRef` and, as `Core\Attributes::get<T>`'s argument, is
/// folded while checking — so the call site excludes it rather than this
/// function testing for it.
pub(crate) fn report_instance_method_called_statically(
    span: Span,
    qname: &QName,
    name: &str,
    env: &mut Env<'_>,
) {
    env.diags.report(
        Diagnostic::error(
            code::E_INSTANCE_METHOD_CALLED_STATICALLY,
            format!("`{qname}::{name}()` is not `static`, so it is called on a value"),
        )
        .with_primary(
            span,
            "called through the class name, with no `$this` in scope",
        )
        .with_help(format!(
            "call it on an instance — `$value->{name}(…)` — or declare `{name}` `static` if it \
             needs no receiver"
        )),
    );
}

/// Both visibility rules a resolved method call answers to, in the order a
/// reader wants them: `rule:classes/interface-private-methods`'s private-interface-method rule first,
/// because it is the more specific refusal, and `rule:core-api/written-visibility`'s three levels only
/// where that one did not already fire.
///
/// They overlap exactly: a `private` interface method is `Visibility::Private`
/// *and* [`MethodSig::interface_private`], and
/// `signatures::is_visible_from` asks the same question of it that
/// [`check_interface_private_visibility`] does. Reporting both would name one
/// mistake twice, and the `rule:classes/no-traits` wording is the one that explains it — so
/// that arm returns here rather than falling through.
///
/// `name` is written bare; the `()` that marks it as a method in the message
/// is added here, so no call site has to remember it.
pub(crate) fn check_method_visibility(
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

/// `rule:classes/interface-private-methods`: a `private` interface method is an internal helper, never
/// part of that interface's contract — visible only from inside its own
/// declaring interface's method bodies (a default or another private
/// method), never through an implementing class, a subinterface, or any
/// other interface. `owner` is the [`QName`] [`resolve_method`] found `sig`
/// declared on, which may differ from the receiver's own static type when
/// the method was inherited — exactly the case this check cares about.
pub(crate) fn check_interface_private_visibility(
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
