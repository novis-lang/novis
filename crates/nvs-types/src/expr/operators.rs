//! What an operator's operands have to be, and what it produces: `rule:types/arithmetic`'s result-type table and the three refusals layered onto it.
//!
//! [`binary_result`] is the table itself, including refusing `int ⊕ uint`.
//! `rule:classes/comparable`'s `Comparable` requirement is the amendment for the five ordering
//! operators when both operands are objects ([`object_comparison_result`]),
//! with no property-walk fallback. `rule:classes/stringable`'s sibling is
//! [`require_stringable`], which is what `rule:types/conversion`'s "anything → `string`"
//! row is worth at an *implicit* site — interpolation, concatenation,
//! `echo`/`print`: a scalar, and an object that provably implements the
//! reserved global `Stringable` interface ([`require_stringable_object`],
//! which the explicit `as string` calls on its own, since that conversion is
//! the one `rule:types/conversion` grants a `bytes` operand). `rule:types/arithmetic` refuses
//! `decimal ⊕ float` outright
//! ([`reject_decimal_float_operands`]), and `rule:enums/closed-integer-type` refuses arithmetic on an
//! enum and a conversion between two of them.
//!
//! Two more refusals are about which operands may *meet* rather than what they
//! produce. `rule:expressions/disjoint-comparison-refused` refuses `==`/`!=` between two statically **disjoint**
//! types ([`reject_disjoint_equality`]), and its § 6 points a `switch` label
//! and a `match` arm at the same check; `rule:types/array-combination` refuses `+`/`+=` with an
//! array operand ([`reject_array_combination`]), naming `Core\Arr::underlay`.
//!
//! `++`/`--` is the same table read as `± 1`, so its target has to be one of
//! § 4's numeric types ([`reject_increment_on_non_numeric`]). **PHP's string
//! increment does not exist** — `$s++` walking `"a"`→`"b"`→`"aa"` is a
//! divergence taken deliberately, and this is its home: `rule:types/conversion` fixes a
//! binding's type at its declaration and § 4's table has no row producing
//! `"b"` from a `string` and a `1`, so there is no arithmetic here to lower
//! and no type the result could take. Code that wants the next spreadsheet
//! column asks for it by name. The cost is one refused shape in ported code,
//! against a `string` that silently changes length and alphabet under `+= 1`.
//!
//! `as` is here too, as the conversion's *operand* rule
//! ([`reject_enum_to_enum_conversion`], and `rule:types/literal-types`'s
//! [`reject_impossible_literal_conversion`] for a conversion whose operand
//! already names a value the target's closed set does not contain), plus
//! `rule:expressions/nullable-conversion-availability`'s target rule for `as ?T`
//! ([`check_class_target_conversion`]: every class target is refused, with no
//! exceptions — § 3a's `tryParse` is the member that answers instead), plus
//! the table's own closure ([`reject_unconvertible`]: § 2 is a *closed* list
//! of rows, so a pair naming none of them has nothing to produce and nothing
//! to throw, and a class target is decided by whether the two types share a
//! value at all — [`reject_unrelated_class_conversion`]); what a
//! conversion does to a qualifier is [`super::quals`], and what its target type
//! may be spelled as is [`crate::lower`].
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `expr as T` — [`super::infer`]'s `ExprKind::Conversion` arm, and the only
/// conversion spelling there is (`rule:types/no-legacy-cast` rejects PHP's legacy `(T)expr`).
///
/// `rule:types/numeric-literal-placement`: `expr as T` is itself a placing position, so a numeric
/// *literal* written directly under one takes `T` as its target rather than
/// being typed first and converted afterwards. Without this, `19.99 as decimal`
/// would round-trip through an `f64` and lose everything past ~17 digits — § 4's
/// `float → decimal` row — making a wider literal unwritable anywhere that lacks
/// an annotation. Restricted to a literal operand on purpose: any other operand
/// already has a type of its own, and handing it an expectation would silently
/// change what `as` converts *from*. [`super::infer`] rather than
/// [`check_expr`], because a placement is not an assignment: `1 as string` still
/// places the literal at `string` and still converts, so the conformance check
/// [`check_expr`] would run here would reject every conversion that does any
/// work.
///
/// `rule:types/literal-types` adds the string literal to that same branch, for the same
/// reason one step further on: `"a" as "a"|"b"` is a conversion the target
/// *statically satisfies*, and without the placement the operand would be a
/// plain `string` converting into the set at run time. What the placement
/// cannot satisfy, [`reject_impossible_literal_conversion`] refuses outright —
/// § 6's two diagnostics, and the only pair of conversions the operand's own
/// type can prove nothing will ever come of.
///
/// A `legacy` conversion is PHP's `(T)expr`, which the parser has already
/// reported (`rule:types/no-legacy-cast`). Its operand is still checked and it
/// is still typed `T`, but none of the conversion's own refusals run, so the
/// one mistake gives the one diagnostic.
#[allow(clippy::too_many_arguments)]
pub(crate) fn infer_conversion(
    expr: &Expr,
    inner: &Expr,
    ty: &Type,
    legacy: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let result = lower_type(ty, ctx, env);
    let inner_ty = if matches!(
        inner.kind,
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_)
    ) {
        super::infer(inner, Some(result), live, scope, ctx, env)
    } else {
        check_expr(inner, None, live, scope, ctx, env)
    };
    if legacy {
        return apply_qualifier_conversion_rule(inner_ty, result, env.interner);
    }
    // The *object* half only: `as string` is the explicit conversion, and
    // `rule:types/conversion`'s table grants it rows — `bytes` among them — that no
    // implicit site gets. `rule:expressions/nullable-conversion-availability` row 1 makes `as ?string` available
    // exactly where `as string` is a row, so the sugar asks the same question
    // of the `T` inside it: an object that cannot render is neither.
    let string_target = nullable_inner_target(result, env).unwrap_or(result);
    if matches!(env.interner.get(string_target), Ty::String) {
        require_stringable_object(inner_ty, inner.span, env);
        // `rule:core-classes/html-to-source`: the carrier renders, so the row above admits it — and
        // `Markup as string` is precisely the keystroke that reopens the hole.
        reject_carrier_as_text(inner_ty, inner.span, env);
    }
    check_class_target_conversion(ty, result, expr.span, env);
    // The two tables are one question asked of two spellings. `as ?T` interns
    // as `Union([Null, T])`, which is one `ConvKind::Wide` target and so says
    // nothing to the table below — `rule:expressions/nullable-conversion-availability` is what judges it, over the
    // `T` inside the sugar, and it refuses one row the plain form has no
    // reason to look at.
    if is_written_nullable(ty) {
        reject_unavailable_nullable_conversion(inner_ty, result, expr.span, env);
    } else {
        reject_unconvertible(inner_ty, result, expr.span, env);
    }
    reject_enum_to_enum_conversion(inner_ty, result, expr.span, env);
    reject_secret_markup_conversion(inner_ty, result, expr.span, env);
    reject_non_literal_markup_conversion(inner, result, expr.span, env);
    reject_impossible_literal_conversion(inner, inner_ty, result, expr.span, env);
    reject_impossible_class_reference_conversion(inner, result, expr.span, ctx, env);
    check_property_key_conversion(inner, ty, result, expr.span, env);
    apply_qualifier_conversion_rule(inner_ty, result, env.interner)
}

/// `rule:types/class-reference`: **a written-out class name is decided at compile time.**
/// `Dog::class as class<Animal>` and `'Shop\Dog' as class<Animal>` are a
/// compile-time yes when `Dog` is an `Animal` and a compile-time refusal when it
/// is not, so there is no run-time check for the program to reach and no throw
/// for it to handle. This is the ordinary shape a factory takes, which is why
/// § 2 singles it out: the common case pays nothing at run time.
///
/// **Two operand shapes.** `::class` goes through the file's imports and
/// namespace ([`super::members::check_class_name_const`], which also reports a
/// name it resolves to nothing). A plain string literal is the class's whole
/// name ([`nvs_hir::QName::from_literal`]), and `nvs_hir::requires` has already
/// loaded its class through `autoload` by the same reading, so a name that is
/// still undeclared here is one nothing can load: [`code::E_UNDEFINED_CLASS`],
/// the code `Bogus::class` gets. Every other operand — a class constant, a
/// concatenation, a variable — is the § 2 row proper, checked where the value
/// arrives against the classes the program loaded.
///
/// The refusal of a class outside `T` is [`code::E_NO_CONVERSION`], the code
/// [`reject_unrelated_class_conversion`] gives the same mistake written at an
/// instance — two types that share no value.
fn reject_impossible_class_reference_conversion(
    inner: &Expr,
    to: TypeId,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let operand = inner.unparenthesized();
    if !matches!(
        operand.kind,
        ExprKind::ClassNameConst { .. } | ExprKind::Str(_)
    ) {
        return;
    }
    // Under `as ?class<T>` as well, and by the same sentence: `rule:expressions/nullable-conversion-availability`
    // makes the sugar answer `null` where the checked form throws, and § 2's
    // written-out operand never throws — it is decided here — so a name
    // outside `T`'s hierarchy has no run-time failure for the `?` to convert
    // into a `null` and is the same refusal. The target reaches here as
    // `Union([Null, ClassRef])`, which [`nullable_inner_target`] unwraps.
    let to = nullable_inner_target(to, env).unwrap_or(to);
    let Ty::ClassRef(arg) = *env.interner.get(to) else {
        return;
    };
    let Ty::Class(target_q, _) = env.interner.get(arg).clone() else {
        return;
    };
    let from_q = match &operand.kind {
        // `None` is a class side already reported by `check_class_name_const`,
        // and so is a name that resolves to nothing — a second diagnostic here
        // would only say it vaguer.
        ExprKind::ClassNameConst { class } => {
            let Some(from_q) = super::members::resolve_class_expr(class, ctx, env) else {
                return;
            };
            from_q
        }
        ExprKind::Str(text) => {
            let Some(from_q) = class_literal_name(*text, &target_q, env) else {
                return;
            };
            from_q
        }
        _ => return,
    };
    if from_q == target_q || nvs_hir::hierarchy::implements_interface(&from_q, &target_q, env.graph)
    {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_NO_CONVERSION,
            format!("`{from_q}` is not a `{target_q}`, so this can never be a `class<{target_q}>`"),
        )
        .with_primary(span, "converted here")
        .with_help(
            "`rule:types/class-reference` decides a class name written out in the source at compile \
             time, and this one names a class outside the hierarchy: convert to `class<T>` at a \
             class the name actually reaches, or make the class an implementor of the one written here",
        ),
    );
}

/// The class a string literal under `as class<T>` names, or `None` after
/// reporting [`code::E_UNDEFINED_CLASS`] when it names none: the text is not a
/// class name at all, or no declaration and no `autoload` entry answers it.
/// `Core`'s classes and the reserved global ones are declared by no source
/// file and pass through, as they do for `X::class`.
fn class_literal_name(text: Span, target: &QName, env: &mut Env<'_>) -> Option<QName> {
    let cooked = crate::string_lit::cook_string_literal(env.src, text);
    let name = QName::from_literal(&cooked);
    if let Some(name) = &name
        && (env.symbols.get(name).is_some()
            || name.is_core()
            || name.is_reserved_global_class()
            || name.is_reserved_global_interface())
    {
        return Some(name.clone());
    }
    let headline = match &name {
        Some(name) => format!("`{name}` is not declared"),
        None => format!("`{cooked}` is not a class name"),
    };
    env.diags.report(
        Diagnostic::error(code::E_UNDEFINED_CLASS, headline)
            .with_primary(
                text,
                "no declaration and no `autoload` entry has this class",
            )
            .with_help(format!(
                "a string converted to `class<{target}>` is the class's whole name, with its \
                 namespace and no leading `\\`: no `namespace` and no `use` applies to it"
            )),
    );
    None
}

/// `rule:types/property-key`: **a written-out operand is decided where it is written.**
/// `"email" as property<User>` is a compile-time yes when `User` declares a
/// public `$email`, and [`code::E_UNKNOWN_MEMBER`] — the diagnostic an ordinary
/// `$user->emial` already gets — when it does not, rather than a throw the
/// program has to reach. A hand-written key therefore pays nothing at run time
/// and misspells at build time.
///
/// [`reject_impossible_class_reference_conversion`]'s shape one type over: a
/// plain string literal is decided where it is written, since a property name
/// is written as text and § 2's door is the `string` row itself. A computed
/// operand is that row proper, checked where the value arrives, and reaches
/// nothing here.
///
/// A name outside the set and a name naming a `private` property are one
/// failure with one message, which is § 2's sentence: visibility is decided at
/// the conversion, once. Under `as ?property<T>` as well, for
/// `rule:expressions/nullable-conversion-availability`'s
/// reason the sibling states — the sugar answers `null` where the checked form
/// throws, and an operand decided here never had a throw to convert.
///
/// **Every other operand is § 2's `string` row proper, and this is where its
/// run-time test is prepared.** The roster resolved here is the whole content
/// of that test, and it is the one thing erasure loses — see
/// [`ExprInfo::PropertyKey`], which is what carries it to `nvs-ir` and why the
/// entry is keyed by the annotation's span. Both spellings record it, since
/// `as ?property<T>` answers `null` exactly where the checked form throws and
/// so tests the same set.
fn check_property_key_conversion(
    inner: &Expr,
    ty: &Type,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let to = nullable_inner_target(to, env).unwrap_or(to);
    let Some(argument) = super::members::property_key_argument(to, env.interner) else {
        return;
    };
    let Ty::Class(qname, _) = env.interner.get(argument).clone() else {
        return;
    };
    let roster = super::members::public_property_names(&qname, env);
    if roster.is_empty() {
        reject_empty_property_key_set(&qname, ty.span, env);
    }
    let ExprKind::Str(text) = &inner.kind else {
        // § 2's `string` and `property<U>` rows: the name arrives when the
        // statement runs, so the set it must be one of travels to the lowering
        // instead of being decided here. An empty roster is recorded like any
        // other, refused above though it is: `nvs-ir` has no fallback for a
        // conversion whose annotation carries no entry, and a type that cannot
        // be reached at run time is not a type that is never lowered.
        env.exprs.record(
            ty.span,
            ExprInfo::PropertyKey {
                class: qname.to_string(),
                names: roster,
            },
        );
        return;
    };
    if roster.is_empty() {
        // The *type* is the fault and it has been named; the operand handed to
        // it is not a second mistake, and `E0799` plus an `E0405` about a name
        // no spelling could have got right reads as two problems.
        return;
    }
    let name = crate::string_lit::cook_string_literal(env.src, *text);
    if roster.contains(&name) {
        return;
    }
    let listed = roster
        .iter()
        .map(|property| format!("`${property}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let help = format!(
        "`rule:types/property-key` decides a written-out operand where it is written, and a key's values are \
         `{qname}`'s public declared properties: {listed}"
    );
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no public property `${name}`"),
        )
        .with_primary(span, "converted here")
        .with_help(help),
    );
}

/// `rule:types/property-key`'s second refusal: `property<T>` where `T` declares no public
/// property at all is `E0799` where it is written, "since no value of that type
/// could ever exist".
///
/// **Reported here rather than beside the class-kind half in
/// `crate::lower::lower_property_key`**, which is where the written type is
/// lowered and where the rule obviously belongs. It cannot go there: that pass
/// runs during signature *collection* as well as at check time, and
/// `Env::signatures` points at an empty placeholder during collection — so a
/// roster read from a lowering is empty for every class in the program, and the
/// rule would refuse every annotation ever written. The conversion is the one
/// place the table is real and the roster is already walked.
///
/// Siting it at the conversion loses nothing, because **`as` is this type's
/// only source** — `rule:types/property-key`, and the ADR's own title. A `property<T>`
/// annotation over an empty `T` that no conversion ever feeds declares a
/// variable no value can reach, and every spelling that would reach one is
/// refused here, with the annotation's own span underlined.
fn reject_empty_property_key_set(qname: &QName, span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS,
            format!("`property<{qname}>` names a class that declares no public property"),
        )
        .with_primary(span, "no name could ever reach this key")
        .with_help(
            "`rule:types/property-key` makes a key's values the public declared property names of its \
             argument — its own and its ancestors' — so a class declaring none has an empty set \
             and no value of the type exists; a `private` property is not one of them, so name \
             the class that declares the property publicly, or make it `public`",
        ),
    );
}

/// The binary-operator result-type table, `rule:types/arithmetic`, amended by `rule:types/ordering` for `< <= > >= <=>` when both operands are objects. Beyond that one
/// amendment, only `int`/`uint`/`float` operands are modeled this slice —
/// anything else (`mixed`, an unresolved call result) falls back to `mixed`
/// rather than diagnosing, since no general operator-overload rule is
/// implemented yet.
pub(crate) fn binary_result(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    // Ahead of the whole table: an operand that is not a value has no row to
    // be judged by, whichever operator it was written under. `.` is the one
    // exception and keeps its own code — [`reject_void_operand`] says why.
    if let Some(mixed) = reject_void_operand(op, lhs, rhs, span, env) {
        return mixed;
    }
    let arithmetic_or_bitwise = matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr
    );
    if arithmetic_or_bitwise && let Some(mixed) = reject_enum_operand(op, lhs, rhs, span, env) {
        return mixed;
    }
    match op {
        // `rule:security/taint-propagation` / `rule:security/secret-propagation`: concatenating a qualified operand with
        // an unqualified one poisons the result on that axis, the same
        // "poisoned" shape `rule:types/declaration` already uses for mixed-type arithmetic —
        // `tainted` and `secret` poison independently of each other.
        BinaryOp::Concat => {
            let tainted = is_tainted(lhs, env.interner) || is_tainted(rhs, env.interner);
            let secret = is_secret(lhs, env.interner) || is_secret(rhs, env.interner);
            qualified_scalar(false, tainted, secret, env.interner)
        }
        // `rule:core-classes/html-auto-escape`'s and `rule:tooling/styling-is-a-value-not-a-grammar`'s composition rules are ahead of the
        // arithmetic table rather than rows of it: a carrier is a class, and
        // every class beside an arithmetic operator is refused two lines down.
        BinaryOp::Add => carrier_composition_result(lhs, rhs, span, env)
            .or_else(|| reject_array_combination(lhs, rhs, span, env))
            .or_else(|| reject_unrowed_arithmetic_operand(op, lhs, rhs, span, env))
            .unwrap_or_else(|| arithmetic_result(lhs, rhs, span, env)),
        BinaryOp::Sub | BinaryOp::Mul => reject_unrowed_arithmetic_operand(op, lhs, rhs, span, env)
            .unwrap_or_else(|| arithmetic_result(lhs, rhs, span, env)),
        BinaryOp::Mod => reject_unrowed_arithmetic_operand(op, lhs, rhs, span, env)
            .or_else(|| reject_float_modulo(lhs, rhs, span, env))
            .unwrap_or_else(|| arithmetic_result(lhs, rhs, span, env)),
        BinaryOp::Pow => reject_unrowed_arithmetic_operand(op, lhs, rhs, span, env)
            .unwrap_or_else(|| power_result(lhs, rhs, span, env)),
        BinaryOp::Div => reject_unrowed_arithmetic_operand(op, lhs, rhs, span, env)
            .unwrap_or_else(|| division_result(lhs, rhs, span, env)),
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            bitwise_result(op, lhs, rhs, span, env)
        }
        BinaryOp::Cmp => object_comparison_result(op, lhs, rhs, span, env)
            .or_else(|| reject_unordered_operand(op, lhs, rhs, span, env))
            .unwrap_or_else(|| env.interner.int()),
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
            object_comparison_result(op, lhs, rhs, span, env)
                .or_else(|| reject_unordered_operand(op, lhs, rhs, span, env))
                .unwrap_or_else(|| env.interner.bool_ty())
        }
        BinaryOp::Eq | BinaryOp::NotEq => {
            reject_disjoint_equality(lhs, rhs, span, env);
            // `rule:security/secret-comparison-is-constant-time`: two `secret` operands compare in constant time.
            // Nothing about the *result* changes — it is a `bool` either way —
            // so this records the fact for `nvs-ir` rather than returning a
            // different type. It has to be recorded here because the qualifier
            // does not survive `nvs_ir::ty::Ty`, which is § 1's promise that a
            // `secret string` costs no representation; see
            // `ExprInfo::SecretEquality`.
            if is_secret(lhs, env.interner) || is_secret(rhs, env.interner) {
                env.exprs
                    .record(span, crate::expr_table::ExprInfo::SecretEquality);
            }
            env.interner.bool_ty()
        }
        BinaryOp::And | BinaryOp::Or => env.interner.bool_ty(),
        // `$a ?? $b` yields `$b` exactly when `$a` is `null`, so `null` is
        // gone from the result unless `$b` can be one — which is what makes
        // `string $s = $maybe ?? "d";` type-check at all. Recorded for
        // `nvs-ir` at the same time: see `ExprInfo::Coalesce`.
        BinaryOp::Coalesce => {
            let non_null = env.interner.without_null(lhs);
            let result = env.interner.make_union([non_null, rhs]);
            env.exprs.record(
                span,
                crate::expr_table::ExprInfo::Coalesce { non_null, result },
            );
            result
        }
        _ => env.interner.mixed(),
    }
}

/// `rule:expressions/disjoint-comparison-refused`: a comparison whose two static types are **disjoint** — no single value
/// inhabits both — is a compile error, because the compiler already knows the
/// answer and the author did not mean to write it. Reported for `==`/`!=` from
/// [`binary_result`] and, per that ADR's § 6, for a `switch` label and a
/// `match` arm against their subject ([`crate::locals`]'s `Switch` arm and
/// [`super::infer`]'s `Match`), which are the same comparison written without
/// the operator.
///
/// Only the refusal lives here. What equality *means* at a type that survives
/// it is § 3, which is `nvs-runtime`'s; the narrowing a null test performs is
/// [`crate::locals`]'s `narrow`.
pub(crate) fn reject_disjoint_equality(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) {
    if !types_are_disjoint(lhs, rhs, env) {
        return;
    }
    let lhs_is_null = matches!(env.interner.get(lhs), Ty::Null);
    let rhs_is_null = matches!(env.interner.get(rhs), Ty::Null);
    // The table gives the `null` row its own wording: the author wrote a test
    // that reads as a question, and what they need told is that the binding
    // they tested was never declared able to answer it.
    let (message, help) = if lhs_is_null || rhs_is_null {
        let held = env.interner.describe(if lhs_is_null { rhs } else { lhs });
        (
            format!("`{held}` cannot hold `null`, so this test is always false"),
            "declare the binding nullable — `?T` — if it is meant to be optional, or drop the test",
        )
    } else {
        let lhs_described = env.interner.describe(lhs);
        let rhs_described = env.interner.describe(rhs);
        let help = disjoint_help(env.interner.get(lhs), env.interner.get(rhs));
        (
            format!(
                "`{lhs_described}` and `{rhs_described}` are disjoint; no value is both, so this \
                 comparison is always false"
            ),
            help,
        )
    };
    env.diags.report(
        Diagnostic::error(code::E_DISJOINT_EQUALITY, message)
            .with_primary(span, "compared here")
            .with_help(help),
    );
}

/// The advice `E0466` ends on, chosen from the two operands' domains.
///
/// The generic sentence names a cast, and a cast is the answer for most of
/// `rule:expressions/disjoint-comparison-refused`'s refused rows — but not for
/// the rows where no conversion exists in that direction. An enum reaches a
/// number through `as int` and nothing else, two enums share no value at all,
/// and two unrelated classes are joined by no conversion the language has. A
/// help naming a cast there tells a reader to write something that does not
/// compile.
fn disjoint_help(lhs: &Ty, rhs: &Ty) -> &'static str {
    let (Some(lhs_domain), Some(rhs_domain)) = (equality_domain(lhs), equality_domain(rhs)) else {
        return "convert one side deliberately and then compare — `$s == ($n as string)`";
    };
    match (lhs_domain, rhs_domain) {
        (EqDomain::Enum(_), EqDomain::Numeric) | (EqDomain::Numeric, EqDomain::Enum(_)) => {
            "an enum case carries an integer — compare `$e as int` against the number"
        }
        (EqDomain::Enum(_), _) | (_, EqDomain::Enum(_)) => {
            "an enum case is only ever equal to a case of the same enum — `match` on it, or \
             compare `$e as int` on both sides"
        }
        (EqDomain::Object, EqDomain::Object) => {
            "two unrelated classes have no value in common — compare something both of them \
             carry, such as an id property"
        }
        _ => "convert one side deliberately and then compare — `$s == ($n as string)`",
    }
}

/// Whether no single value inhabits both `lhs` and `rhs` — `rule:expressions/disjoint-comparison-refused`'s
/// table, read as a *disjointness* question rather than an equality-of-types
/// one, which is what keeps every shape ordinary code writes compiling.
///
/// Deliberately one-sided: it answers `true` only where disjointness is
/// provable from the two types alone, and `false` — "these may overlap" — for
/// everything it does not model. A missed diagnostic costs an author nothing;
/// a wrong one costs them a program that used to build. `mixed` (§ 5 resolves
/// it at runtime instead), `iterable`, an intersection, an options bag and a
/// type variable all take that branch, as does any class name this
/// compilation did not declare.
///
/// [`super::type_test`] reads the same answer as a question about one value —
/// a subject disjoint from the type it is tested against is a `$x is T` that
/// folds to `false` — and that one-sidedness is what keeps the fold honest
/// there too.
pub(crate) fn types_are_disjoint(lhs: TypeId, rhs: TypeId, env: &Env<'_>) -> bool {
    if lhs == rhs {
        return false;
    }
    // A union is disjoint from the other side only when *every* member is,
    // which is the whole of the table's `?T`/union rows: `?T == null` and
    // `?T == T` both find an overlapping member, while `T == null` has none.
    if let Ty::Union(members) = env.interner.get(lhs) {
        return members.iter().all(|m| types_are_disjoint(*m, rhs, env));
    }
    if let Ty::Union(members) = env.interner.get(rhs) {
        return members.iter().all(|m| types_are_disjoint(lhs, *m, env));
    }
    let (lhs_ty, rhs_ty) = (env.interner.get(lhs), env.interner.get(rhs));
    let (Some(lhs_domain), Some(rhs_domain)) = (equality_domain(lhs_ty), equality_domain(rhs_ty))
    else {
        return false;
    };
    if lhs_domain != rhs_domain {
        return true;
    }
    // One row refines further: two class names share a domain, but no
    // instance is both unless one of them reaches the other.
    match (lhs_ty, rhs_ty) {
        (Ty::Class(lhs_q, _), Ty::Class(rhs_q, _)) => classes_are_unrelated(lhs_q, rhs_q, env),
        _ => false,
    }
}

/// The domains `rule:expressions/disjoint-comparison-refused`'s table partitions comparable types into: two
/// values can only ever be equal when their types land in the same one.
/// `None` means "not modeled", and [`types_are_disjoint`] owns what that
/// buys.
#[derive(PartialEq, Eq)]
enum EqDomain<'a> {
    Null,
    Bool,
    /// `int`, `uint`, `float` and `decimal` are **one** domain — the table's
    /// numeric row makes them mathematically exact over the full range of
    /// both, so no pairing among them is ever disjoint.
    Numeric,
    Str,
    Bytes,
    Array,
    /// A class, a shape, or `object`.
    Object,
    Callable,
    /// An enum is its own domain, per name — which is what makes both
    /// `$e == 1` and `$e == $otherEnum` refusals, the first answered by
    /// `rule:enums/no-class-machinery`'s `as int` and the second by an explicit `match`.
    Enum(&'a QName),
    /// `class<T>` — its own domain, and deliberately not [`Self::Object`]'s
    /// and not [`Self::Str`]'s. A class reference's value is a descriptor, so
    /// two of them compare by descriptor identity and nothing else can be
    /// equal to one: an instance is not its own class, and `$cls == "Dog"` is
    /// exactly the string-as-a-class confusion `rule:types/class-reference` exists to keep out
    /// (`Foo::class` stays a `string`, and `as class<T>` is the only door).
    /// Not parameterised by the argument, because covariance (§ 3) means two
    /// class references at different arguments can hold the same descriptor.
    ClassRef,
    /// `property<T>` — its own domain, for [`Self::ClassRef`]'s reason applied
    /// to a member name: two keys are equal when they name the same property,
    /// and `$key == "email"` is the string-as-a-member confusion `rule:types/property-key`
    /// keeps out, `as property<T>` being the only door. Not parameterised by
    /// the argument, because § 3's contravariance means two keys at different
    /// arguments can hold the same name.
    PropertyKey,
}

fn equality_domain(ty: &Ty) -> Option<EqDomain<'_>> {
    Some(match ty {
        Ty::Null => EqDomain::Null,
        Ty::Bool | Ty::True | Ty::False => EqDomain::Bool,
        Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal => EqDomain::Numeric,
        // A qualifier is a fact about where a value has been, never about
        // which values it can hold (`rule:security/taint-propagation` / `rule:security/secret-propagation`), so all four
        // spellings of each base are one domain.
        Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString => {
            EqDomain::Str
        }
        Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes => EqDomain::Bytes,
        // `rule:types/literal-types` gives a literal type its base's representation exactly,
        // so it lands in its base's domain and nothing more: `$mode == "z"`
        // where `$mode` is `"a"|"b"` compares two strings and is answered at
        // run time. Refusing it because the two literal *sets* do not overlap
        // would be a new row in `rule:expressions/disjoint-comparison-refused`'s table, not a consequence of
        // this one.
        Ty::StringLiteral(_) => EqDomain::Str,
        Ty::IntLiteral(_) => EqDomain::Numeric,
        Ty::Array(_) => EqDomain::Array,
        Ty::ClassRef(_) => EqDomain::ClassRef,
        Ty::PropertyKey(_) => EqDomain::PropertyKey,
        Ty::Object | Ty::Class(..) | Ty::Shape(_) => EqDomain::Object,
        // A written signature is a fact about how a closure may be *called*,
        // never about which closures a position holds, so it shares the bare
        // type's domain: two callable values compare by identity either way,
        // and `rule:types/callable-signature` adds nothing a comparison could
        // read.
        Ty::Callable | Ty::CallableSig { .. } => EqDomain::Callable,
        // Both spellings of "a value of this enum" — `rule:types/enum-case-type` keeps a case
        // type a *subtype* of its enum, so it shares its enum's domain and
        // stays disjoint from every other one, `int` included.
        Ty::Enum(qname, _) | Ty::EnumCase(qname, _, _) => EqDomain::Enum(qname),
        Ty::Mixed
        | Ty::Iterable
        | Ty::Void
        | Ty::Never
        | Ty::Union(_)
        | Ty::Intersection(_)
        | Ty::CoreShape(_)
        | Ty::TypeVar(_)
        // Beside `TypeVar` for its reason: a `Core` parameter's own spelling
        // is never the type of an expression, so nothing ever compares one.
        | Ty::ShapeOfCallables(_) => return None,
    })
}

/// Whether no instance can be both a `lhs_q` and a `rhs_q` — the table's "two
/// unrelated classes" row, and the one place this check consults the
/// hierarchy rather than the type alone.
///
/// An **interface** on either side is never unrelated: this sees two names,
/// and some third class the comparison never mentions may implement both. A
/// name with no `class` declaration in this compilation — every `Core` class
/// among them — is treated the same way, which is the same scoping
/// [`object_comparison_result`] and [`require_stringable`] already use.
fn classes_are_unrelated(lhs_q: &QName, rhs_q: &QName, env: &Env<'_>) -> bool {
    if lhs_q == rhs_q {
        return false;
    }
    if !is_declared_class(lhs_q, env) || !is_declared_class(rhs_q, env) {
        return false;
    }
    !nvs_hir::implements_interface(lhs_q, rhs_q, env.graph)
        && !nvs_hir::implements_interface(rhs_q, lhs_q, env.graph)
}

fn is_declared_class(qname: &QName, env: &Env<'_>) -> bool {
    env.symbols
        .get(qname)
        .is_some_and(|symbol| symbol.kind == nvs_hir::SymbolKind::Class)
}

/// `rule:classes/ordering-lowers-to-compare-to`, `rule:classes/comparable` and `rule:classes/comparable-is-same-class-only`: `< <= > >= <=>` lower to a `compareTo` call when both
/// operands are objects, so ordering them requires both sides to be the same
/// class and that class to (transitively) implement the reserved global
/// `Comparable` interface — returns `None` when either operand isn't a class
/// at all, leaving [`binary_result`]'s ordinary scalar/`mixed` fallback in
/// place untouched, since this ADR only amends `rule:types/arithmetic`'s table with a
/// new object-operand row rather than replacing it. An enum operand
/// (`Ty::Enum`) is deliberately not treated as an object here either — ADR
/// 0010's own item (still unimplemented) is what would say whether an enum
/// can ever be `Comparable`.
pub(crate) fn object_comparison_result(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let (Ty::Class(lhs_q, _), Ty::Class(rhs_q, _)) =
        (env.interner.get(lhs).clone(), env.interner.get(rhs).clone())
    else {
        return None;
    };
    if lhs_q != rhs_q {
        report_comparable_diagnostic(
            span,
            format!(
                "`{lhs_q}` and `{rhs_q}` are different classes; `<`/`<=`/`>`/`>=`/`<=>` never \
                 compare across classes, even when both implement `Comparable`"
            ),
            env,
        );
        return Some(env.interner.mixed());
    }
    let comparable = QName::parse(nvs_hir::interfaces::COMPARABLE);
    if !reaches_comparable(&lhs_q, &comparable, env) {
        report_comparable_diagnostic(
            span,
            format!(
                "`{lhs_q}` does not implement `Comparable`; ordering two objects with \
                 `<`/`<=`/`>`/`>=`/`<=>` requires it"
            ),
            env,
        );
        return Some(env.interner.mixed());
    }
    // `rule:classes/ordering-lowers-to-compare-to`: the comparison *is* a `compareTo` call, so `nvs-ir` needs
    // its resolved target the same way an ordinary `$a->compareTo($b)` does —
    // recorded under the *binary expression's* own span, since there is no
    // call node in the AST to key it by. `Comparable::compareTo` is bodiless,
    // so `has_body` is `false` and the call dispatches on the receiver's
    // runtime class, exactly like any other call to an interface method.
    if let Some((owner, sig)) = resolve_method(&lhs_q, "compareTo", env.signatures, env.graph) {
        // One argument, at the one parameter: the operator writes the call
        // itself, so its mapping is fixed rather than resolved.
        let slots = vec![ArgSlot::Param(0)];
        let call = resolved_call(owner, "compareTo".to_owned(), &sig, slots, env.signatures);
        env.exprs.record(span, ExprInfo::Call(call));
    }
    Some(match op {
        BinaryOp::Cmp => env.interner.int(),
        _ => env.interner.bool_ty(),
    })
}

/// Whether `qname` reaches `Comparable` — **two tables, because a `Core` class
/// declares nothing.** [`nvs_hir::implements_interface`] answers for a written
/// `implements Comparable` and for the reflexive case (a value typed at the
/// interface itself), and it is the whole answer for a user class;
/// [`crate::signatures::resolve_interface_args`] answers for a class whose
/// conformance was *seeded* rather than written, which is
/// `crate::core_lib`'s `Core\Time\Duration` and its siblings — a `Core` class
/// has no [`nvs_hir::ClassGraph`] entry at all, so the first table cannot see
/// it.
fn reaches_comparable(qname: &QName, comparable: &QName, env: &Env<'_>) -> bool {
    nvs_hir::implements_interface(qname, comparable, env.graph)
        || crate::signatures::resolve_interface_args(qname, comparable, env.signatures, env.graph)
            .is_some()
}

pub(crate) fn report_comparable_diagnostic(span: Span, message: String, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_COMPARISON_REQUIRES_COMPARABLE, message)
            .with_primary(span, "compared here")
            .with_help("implement `Comparable`'s `compareTo(self $other): int` on the class"),
    );
}

/// Which of [`reject_unordered_operand`]'s three wordings an offending operand
/// takes — an owned classification rather than the borrowed [`EqDomain`] it is
/// derived from, so that the report below it may take `env` mutably.
#[derive(Clone, Copy)]
enum Unordered {
    Object,
    Str,
    Enum,
    Other,
}

/// A call that returns `void` is not an operand of anything, and this is the
/// refusal that says so — one step earlier than every other one in this file.
///
/// The refusals around it each read a *row* of `rule:types/arithmetic`'s table and object
/// that the operand names none. A `void` call names none either, but for a
/// reason no row can be about: it has no value at all, so there is nothing
/// there to look up. That is why this runs ahead of all of them, and ahead of
/// the "type not yet known" pass-through each of them shares —
/// [`equality_domain`] answers `None` for `Ty::Void` exactly as it does for
/// `mixed`, but `mixed`'s answer comes from a runtime tag it *has* and this
/// one has no value to carry a tag.
///
/// Unrefused it reached neither a diagnostic nor an answer: `nvs-ir` lowers a
/// `void` call to no value, so `V::nothing() + 1` failed the whole compilation
/// with "nvs-codegen does not lower an operand used before it is defined",
/// which names a bug in the compiler for what is a mistake in the program.
///
/// `.` is deliberately **not** routed here — [`code::E_NO_STRING_FORM`]
/// already names a `void` call in its own roster, alongside the three other
/// types with no implicit `string` form, and one rule draws one code.
/// [`code::E_VOID_IS_NOT_AN_OPERAND`]'s doc comment is this split's home.
/// Returns `Some(mixed)` once diagnosed, `None` for every operand the caller's
/// own table should answer itself.
fn reject_void_operand(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    if op == BinaryOp::Concat {
        return None;
    }
    if !matches!(env.interner.get(lhs), Ty::Void) && !matches!(env.interner.get(rhs), Ty::Void) {
        return None;
    }
    report_void_operand(span, env);
    Some(env.interner.mixed())
}

/// A call that returns `void` is not a condition either, and this is the
/// refusal that says so.
///
/// [`reject_void_operand`] above objects that `rule:types/arithmetic`'s table has no row
/// for a value that is not one. A condition is the one position that table is
/// not about — `rule:expressions/truthy-table`'s truthy table is, and it has a row for every type
/// there is, which is exactly why the missing value shows up here as nothing
/// at all rather than as a mismatch. So the two refusals are one sentence
/// apart and take two codes: [`code::E_VOID_IS_NOT_A_CONDITION`]'s own doc
/// comment is that split's home, and the short of it is that "not an operand"
/// is the wrong sentence to print under `if (V::nothing())`, where no operator
/// is written at all.
///
/// Called from [`check_condition`](super::check_condition) for the four
/// statement conditions and the ternary, and from [`super::infer`]'s own arms
/// for `!` and `empty()`, which infer their operand themselves.
pub(crate) fn reject_void_condition(ty: TypeId, span: Span, env: &mut Env<'_>) {
    if !matches!(env.interner.get(ty), Ty::Void) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_VOID_IS_NOT_A_CONDITION,
            "a call that returns `void` has no value, so there is nothing to test for truth",
        )
        .with_primary(span, "tested here")
        .with_help(
            "`rule:expressions/truthy-table`'s truthy table is over values, and a `void` call is not one; give the \
             callee a return type and `return` from it, or call it as its own statement",
        ),
    );
}

/// The one diagnostic both operand positions report — the binary operators
/// through [`reject_void_operand`] and the three arithmetic prefixes through
/// [`reject_unary_arith_operand`], so that they agree on their wording as well
/// as on their rule.
fn report_void_operand(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_VOID_IS_NOT_AN_OPERAND,
            "a call that returns `void` has no value, so it is not an operand",
        )
        .with_primary(span, "operated on here")
        .with_help(
            "`rule:types/arithmetic`'s table has no row for a value that is not one; give the callee a \
             return type and `return` from it, or call it as its own statement",
        ),
    );
}

/// `rule:types/arithmetic`'s ordering row is a **closed** list, so an operand it does not
/// name has no `<`/`<=`/`>`/`>=`/`<=>` at all and is refused where it is
/// written rather than answered below.
///
/// The table orders the numeric types against each other, and `rule:classes/comparable` orders
/// two objects of one `Comparable` class — [`object_comparison_result`] owns
/// that half and runs first, so everything reaching here either names no class
/// at all or names one on only one side. Everything else PHP orders, it orders
/// by converting an operand first, which `rule:types/conversion` never does by itself:
/// two strings order through `Core\Str::compare`, an enum case through its
/// backing `as int`, and an `array<T>`, a `callable` and `null` not at all.
///
/// `bool` is deliberately a row rather than a refusal — `false < true` is the
/// ordering of the one bit it already is, which is PHP's answer too and needs
/// no conversion to be exact. [`code::E_ORDERING_HAS_NO_ROW`]'s own doc
/// comment is that decision's home.
///
/// Scoped exactly like [`reject_bitwise_operand`] beside it: an operand whose
/// type is not yet known ([`equality_domain`] answering `None` — `mixed`, a
/// union, a type variable) passes through, since what it holds is a run-time
/// question. Returns `Some(mixed)` once diagnosed, `None` for every pair the
/// caller's own table should answer itself.
fn reject_unordered_operand(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    // The left operand first, so a pair that is wrong on both sides reports
    // once and names the side written first.
    let (offender, domain) = [lhs, rhs].into_iter().find_map(|ty| {
        let domain = match equality_domain(env.interner.get(ty))? {
            // A row: the numeric widenings, and the one bit `bool` already is.
            EqDomain::Numeric | EqDomain::Bool => return None,
            EqDomain::Object => Unordered::Object,
            EqDomain::Str => Unordered::Str,
            EqDomain::Enum(_) => Unordered::Enum,
            EqDomain::Bytes
            | EqDomain::Array
            | EqDomain::Callable
            | EqDomain::Null
            | EqDomain::ClassRef
            | EqDomain::PropertyKey => Unordered::Other,
        };
        Some((ty, domain))
    })?;
    if matches!(domain, Unordered::Object) {
        // The object family keeps one code however the receiver was spelled:
        // an erased `object`, a shape and a class-against-`object` pair all
        // name no class to ask `Comparable` about, which is what the two
        // arms above this one report when the class *is* named.
        report_comparable_diagnostic(
            span,
            format!(
                "`{}` names no class to check `Comparable` on; ordering two objects with \
                 `<`/`<=`/`>`/`>=`/`<=>` requires a class that implements it",
                env.interner.describe(offender)
            ),
            env,
        );
        return Some(env.interner.mixed());
    }
    let spelling = match op {
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        _ => "<=>",
    };
    let help = match domain {
        Unordered::Str => {
            "`Core\\Str::compare` is the ordering two strings have; `rule:types/arithmetic` tabulates no \
             `<` for text, because PHP's own answer there is a conversion Novis never makes by \
             itself"
        }
        Unordered::Enum => {
            "an enum case is a name rather than a number (`rule:enums/closed-integer-type`); order the backing values \
             instead — `($a as int) < ($b as int)`"
        }
        _ => {
            "`rule:types/arithmetic`'s `< <= > >= <=>` row is the numeric types, plus two objects of one \
             `Comparable` class (`rule:classes/comparable`); this operand is on neither half"
        }
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ORDERING_HAS_NO_ROW,
            format!(
                "`{spelling}` has no meaning for `{}`",
                env.interner.describe(offender)
            ),
        )
        .with_primary(span, "ordered here")
        .with_help(help),
    );
    Some(env.interner.mixed())
}

/// `rule:types/arithmetic`'s arithmetic table is **closed** at the operand end too, and
/// this is the refusal that says so — [`reject_unordered_operand`]'s twin, one
/// row of the same table over.
///
/// The operands that table names are the numeric types: `int`, `uint`,
/// `float`, and `decimal` through `rule:types/arithmetic`. Everything else PHP adds it
/// adds by *converting* first, and `rule:types/conversion` has no implicit conversion for
/// that to be, so a `string`, a `bytes`, an `array<T>`, a `callable`, `null`
/// and an object have no `+` at all.
///
/// `bool` is the operand this exists for, and it is deliberately the opposite
/// call to the one [`reject_unordered_operand`] makes: `false < true` is the
/// ordering of the one bit and needs no conversion, but `true + true` is
/// PHP's "convert to `int` first" and nothing else. Unrefused it did not even
/// answer PHP's number — `nvs_ir::ty::Ty::Bool` is `nvs-codegen`'s `integral`,
/// so the pair became an `iadd` over the `i8` a `bool` is stored in and
/// `echo true + true` printed `1` where PHP prints `2`.
///
/// An **enum** operand never reaches here: [`binary_result`] refuses it first
/// through [`reject_enum_operand`], so a type never draws two codes for one
/// rule. Scoped exactly like [`reject_bitwise_operand`]: an
/// operand whose type is not yet known ([`equality_domain`] answering `None` —
/// `mixed`, a union, the `int|float` a division returns) passes through, and
/// its answer comes from its runtime tag instead (`Helper::ValueAdd`). Returns
/// `Some(mixed)` once diagnosed, `None` for every pair the caller's own table
/// should answer itself.
fn reject_unrowed_arithmetic_operand(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    // The left operand first, so a pair that is wrong on both sides reports
    // once and names the side written first.
    let (offender, domain) = [lhs, rhs].into_iter().find_map(|ty| {
        let domain = match equality_domain(env.interner.get(ty))? {
            // The table's own operands, and the one type [`binary_result`]
            // has already refused with a diagnostic of its own.
            EqDomain::Numeric | EqDomain::Enum(_) => return None,
            other => other,
        };
        Some((ty, domain))
    })?;
    let spelling = binary_operator_spelling(op);
    let help = match domain {
        EqDomain::Bool => {
            "`rule:types/arithmetic`'s arithmetic rows are the numeric types; PHP converts a `bool` to an \
             `int` first and Novis never converts by itself, so say it — `($b ? 1 : 0)`"
        }
        EqDomain::Str => {
            "`rule:types/arithmetic` tabulates no arithmetic for text; `.` is how two strings combine, and \
             `$s as int`/`$s as float` is how one becomes a number"
        }
        // The two sink carriers are the only classes with an arithmetic row, so
        // the general help below would be false where it is most likely to be
        // read: `$m + "raw"` is a half-composed carrier, and the fix is to make
        // the other side one rather than to look for a member that deliberately
        // does not exist ([`carrier_composition_result`]). Each names its own
        // section, because the way to lift the other operand differs — one
        // escapes for HTML, the other substitutes control bytes for a terminal.
        EqDomain::Object if carrier_of(offender, env) == Some(crate::CORE_HTML_MARKUP_CLASS) => {
            "`rule:core-classes/html-auto-escape` composes `Markup` with `Markup` and nothing else: lift the other \
             operand with `as Markup` if it is a source literal, or escape it with \
             `Core\\Html::escape(...)` — `+` is not a sink and will not escape it for you"
        }
        EqDomain::Object if carrier_of(offender, env) == Some(crate::CORE_CLI_TEXT_CLASS) => {
            "`rule:tooling/styling-is-a-value-not-a-grammar` composes `Cli\\Text` with `Cli\\Text` and nothing else: lift the other \
             operand with `Core\\Cli\\Text::plain(...)`, which substitutes its control bytes on \
             the way in — `+` is not a sink and will not substitute them for you"
        }
        EqDomain::Object => {
            "Novis has no operator overloading: `rule:types/arithmetic` names two classes in its arithmetic \
             rows and both are sink carriers — `rule:core-classes/html-auto-escape`'s `Core\\Html\\Markup` and `rule:tooling/terminal-output-is-a-sink` \
             § 2's `Core\\Cli\\Text` — so for every other class the operation belongs in a method \
             on it"
        }
        _ => {
            "`rule:types/arithmetic`'s arithmetic rows are `int`, `uint`, `float` and `decimal`; this \
             operand is none of them, and `as` is the only way to make it one"
        }
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ARITHMETIC_HAS_NO_ROW,
            format!(
                "`{spelling}` has no meaning for `{}`",
                env.interner.describe(offender)
            ),
        )
        .with_primary(span, "no arithmetic row for this operand")
        .with_help(help),
    );
    Some(env.interner.mixed())
}

/// `%` with a `float` operand — the one refusal in this file that both
/// operands *are* numbers for. [`code::E_FLOAT_MODULO`]'s own doc comment is
/// that decision's home: PHP's `%` converts to an integer and returns one,
/// `rule:types/arithmetic`'s "either operand a `float`" row would return a `float`, and
/// the spec's own `Core\Math::mod` row settles it the third way — "integer `%`
/// is the operator", so the floating-point remainder is that member and the
/// operator is refused at both ends rather than guessed at either. The other
/// end is the catchable throw `nvs_runtime::helpers::value_arith` raises where
/// only the tags can see it.
///
/// A `decimal` on either side is left to [`reject_decimal_float_operands`] one
/// level down, which objects to the pair before it objects to the operator.
fn reject_float_modulo(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> Option<TypeId> {
    let pair = (env.interner.get(lhs), env.interner.get(rhs));
    if matches!(pair, (Ty::Decimal, _) | (_, Ty::Decimal)) {
        return None;
    }
    if !matches!(pair, (Ty::Float, _) | (_, Ty::Float)) {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_FLOAT_MODULO,
            "`%` has no meaning for `float`",
        )
        .with_primary(span, "no `float` row for `%`")
        .with_help(
            "PHP's `%` converts both operands to an integer and answers one, where `rule:types/arithmetic`'s \
             float row would answer a `float`; say which was meant — `($a as int) % ($b as int)`, \
             or `Core\\Math::mod($a, $b)` for the floating-point remainder",
        ),
    );
    Some(env.interner.mixed())
}

pub(crate) fn arithmetic_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_decimal_float_operands(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        // `rule:types/arithmetic`: a `decimal` combined with an integer stays `decimal` —
        // an `int`/`uint` is exact in 96 bits, so nothing is lost. The
        // `decimal`/`float` pair never reaches here; it was rejected above.
        (Ty::Decimal, Ty::Decimal | Ty::Int | Ty::Uint) | (Ty::Int | Ty::Uint, Ty::Decimal) => {
            env.interner.decimal()
        }
        (Ty::Float, _) | (_, Ty::Float) => env.interner.float(),
        (Ty::Int, Ty::Int) => env.interner.int(),
        (Ty::Uint, Ty::Uint) => env.interner.uint(),
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        _ => env.interner.mixed(),
    }
}

/// `rule:core-classes/html-auto-escape`'s `Markup + Markup` is `Markup` and
/// `rule:tooling/styling-is-a-value-not-a-grammar`'s
/// `Text + Text` is `Text` — the only rows of any operator table whose operands
/// are a class, and the only arithmetic-shaped pairs that are not arithmetic at
/// all.
///
/// **One rule stated twice, not two rules**, which is why the two share this
/// function rather than getting one each: a sink carrier composes with its own
/// kind and with nothing else, and `nvs_runtime::is_carrier` is the same pair
/// of names answered for the render side. A cross-carrier `$m + $t` is refused
/// with everything else, since the two sinks substitute different things and a
/// value that passed one rule passed the wrong one for the other.
///
/// It is `+` rather than `.` because both sections word it that way, and the
/// wording is load-bearing: `.` is [`BinaryOp::Concat`], whose result is a
/// `string`, and composing two carriers into an untrusted one would lose
/// exactly what a carrier exists to carry. `+` has no `string` row to be
/// confused with, so the class pair is unambiguous.
///
/// **Both operands, never one.** `$m + "x"` and `$m + 1` fall straight through
/// to [`reject_unrowed_arithmetic_operand`], which refuses any class beside an
/// arithmetic operator — which is the refusal § 5 wants, since the other side
/// would have to be escaped and this operator is not a sink.
///
/// Returns `Some` for the pair and `None` for everything else, so
/// [`binary_result`]'s own table runs unchanged.
fn carrier_composition_result(
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let left = carrier_of(lhs, env)?;
    let right = carrier_of(rhs, env)?;
    if left != right {
        return None;
    }
    // Which carrier this is has to be recorded rather than re-derived:
    // `nvs_ir::ty::Ty` erases both to `Ty::Object`, so the lowering sees one
    // pair of representations and two possible symbols. See
    // `ExprInfo::CarrierComposition`.
    let symbol = if left == crate::CORE_HTML_MARKUP_CLASS {
        crate::CORE_HTML_MARKUP_CONCAT
    } else {
        crate::CORE_CLI_TEXT_CONCAT
    };
    env.exprs.record(
        span,
        crate::expr_table::ExprInfo::CarrierComposition { symbol },
    );
    Some(lhs)
}

/// Which sink carrier `ty` is, or `None` for every other type — the name in
/// [`crate::CORE_HTML_MARKUP_CLASS`] or [`crate::CORE_CLI_TEXT_CLASS`], both of
/// which are `nvs-stdlib`'s own re-export of the runtime constant the sink
/// renders against.
///
/// A `&'static str` rather than a `bool` per carrier because two callers need
/// to tell the two apart and not merely to recognise one:
/// [`carrier_composition_result`] admits a pair only when it is the *same*
/// carrier twice, and [`reject_unrowed_arithmetic_operand`]'s help names the
/// section the reader is inside.
fn carrier_of(ty: TypeId, env: &Env<'_>) -> Option<&'static str> {
    let Ty::Class(qname, _) = env.interner.get(ty) else {
        return None;
    };
    let name = qname.to_string();
    [crate::CORE_HTML_MARKUP_CLASS, crate::CORE_CLI_TEXT_CLASS]
        .into_iter()
        .find(|carrier| *carrier == name)
}

/// `rule:types/arithmetic` names no enum among its operands: a case is not a
/// number, so `Permission::Read | Permission::Write` and `-$level` are
/// refused naming `as int`/`as uint` as the fix. Called by [`binary_result`]
/// ahead of every arithmetic and bitwise row, so no table below it ever sees
/// an enum operand.
///
/// Returns `Some(mixed)` once diagnosed, naming the left operand when both
/// are enum values, and `None` for every other pair so the caller's own table
/// runs unchanged.
fn reject_enum_operand(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let offender = [lhs, rhs]
        .into_iter()
        .find(|ty| carries_an_enum_value(*ty, env))?;
    report_enum_operand(binary_operator_spelling(op), offender, span, env);
    Some(env.interner.mixed())
}

/// Whether a value of `ty` can be an enum case: an enum, a case-subset type,
/// or a union with either among its members — `?Size` is the one written
/// most. A union has to be refused here rather than answered from its runtime
/// tag the way `rule:types/arithmetic` answers other unions, because a case
/// carries its backing integer's tag (`rule:enums/representation`), so at run
/// time `-$size` would negate the integer and give a value that is no case.
fn carries_an_enum_value(ty: TypeId, env: &Env<'_>) -> bool {
    let is_enum = |ty: &Ty| matches!(ty, Ty::Enum(..) | Ty::EnumCase(..));
    match env.interner.get(ty) {
        Ty::Union(members) => members.iter().any(|m| is_enum(env.interner.get(*m))),
        other => is_enum(other),
    }
}

/// The one diagnostic every operator over an enum value reports — the binary
/// rows through [`reject_enum_operand`], `-`, `+` and `~` through
/// [`reject_unary_arith_operand`], and `++`/`--` through
/// [`reject_increment_on_non_numeric`] — so all of them agree on the code and
/// the wording.
fn report_enum_operand(spelling: &str, ty: TypeId, span: Span, env: &mut Env<'_>) {
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_ENUM_ARITHMETIC_UNSUPPORTED,
            format!("`{spelling}` has no meaning for `{described}`"),
        )
        .with_primary(span, "an enum case is not a number")
        .with_help(
            "to compute with the integer behind a case, convert it first: `$x as int` \
             (`$x as uint` for an enum backed by `uint`), or `$x as ?int` when the value can \
             be `null`",
        ),
    );
}

/// How `op` is written, for the diagnostics that name it.
fn binary_operator_spelling(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Pow => "**",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        _ => ">>",
    }
}

/// `rule:types/array-combination`: binary `+` and `+=` with an array operand are a compile error naming
/// `Core\Arr::underlay`. PHP's array union operator is *removed*, not migrated,
/// so there is no silent behaviour change to fall into — the operator simply
/// stops compiling, and `nvs convert` rewrites `$a + $b` to the member.
///
/// Returns `Some` once diagnosed, `None` for every other operand pair so
/// [`arithmetic_result`]'s own table runs unchanged. The recovery type is the
/// array operand rather than `mixed`, so `$a += $b` reports this once instead
/// of also failing [`check_compound_assign`]'s write-back check with a second,
/// less useful diagnostic.
fn reject_array_combination(
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let lhs_is_array = matches!(env.interner.get(lhs), Ty::Array(_));
    let rhs_is_array = matches!(env.interner.get(rhs), Ty::Array(_));
    if !lhs_is_array && !rhs_is_array {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ARRAY_PLUS_UNSUPPORTED,
            "`+` is not defined with an array operand",
        )
        .with_primary(span, "array operand of `+`")
        .with_help(concat!(
            r"use `Core\Arr::underlay($a, $b)`, which keeps the left array's value at every ",
            "key both of them have — the one operation `$a + $b` ever meant",
        )),
    );
    Some(if lhs_is_array { lhs } else { rhs })
}

pub(crate) fn division_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_decimal_float_operands(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        // `rule:types/arithmetic`'s deliberate divergence from `int / int`: a decimal
        // quotient is always `decimal`, never a union with `float`. Division is
        // the one place the result may be inexact, and the ADR fixes its
        // rounding in the language rather than in a union the caller unpacks.
        (Ty::Decimal, Ty::Decimal | Ty::Int | Ty::Uint) | (Ty::Int | Ty::Uint, Ty::Decimal) => {
            env.interner.decimal()
        }
        (Ty::Float, _) | (_, Ty::Float) => env.interner.float(),
        (Ty::Int, Ty::Int) => {
            let int = env.interner.int();
            let float = env.interner.float();
            env.interner.make_union([int, float])
        }
        (Ty::Uint, Ty::Uint) => {
            let uint = env.interner.uint();
            let float = env.interner.float();
            env.interner.make_union([uint, float])
        }
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        _ => env.interner.mixed(),
    }
}

pub(crate) fn bitwise_result(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    if let Some(mixed) = reject_bitwise_operand(op, lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        (Ty::Int, Ty::Int) => env.interner.int(),
        (Ty::Uint, Ty::Uint) => env.interner.uint(),
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        // Only an operand whose type is not yet known reaches here now — a
        // `mixed`, a union, a type variable — because every *known* type but
        // the three integer spellings above has been refused two lines up.
        _ => env.interner.mixed(),
    }
}

/// `rule:types/arithmetic`'s `& | ^ ~ << >>` row is `int` and `uint`, and this is what
/// makes that a rule rather than a table the checker happened not to model.
///
/// Every other operand PHP answers by *converting* first — `1.5 & 1.5` is
/// `1 & 1` there, `"ab" & "cd"` is bytewise, `true & true` is `1` — and
/// `rule:types/conversion` has no implicit conversion for any of them to be. Left
/// unmodelled they reached [`bitwise_result`]'s `_ => mixed` arm with nothing
/// reported, and what happened below was worse than a refusal in every
/// direction: a `float` pair became a bit-and over the `f64`'s own bits and
/// answered `1.5`, a `decimal` pair panicked `nvs_ir::lower::expr`'s `rule:types/arithmetic` table, and a `string` pair reached `nvs-codegen`'s "no `BinOp` over
/// this representation".
///
/// A `decimal` is the operand worth naming twice: it is a number, so it
/// passes every "is this arithmetic" guard around it, and it is a coefficient
/// and a scale rather than a bit pattern, so `rule:types/arithmetic` grants it
/// arithmetic, equality and ordering and stops. The refusal here is the one
/// that leaves that table's own catch-all no reachable target.
///
/// Scoped like the refusals around it: an operand whose type is not yet known
/// ([`equality_domain`] answering `None` — `mixed`, a union, a type variable)
/// passes through, and so does an integer *literal* type, which is the same
/// integer one placement later. Returns `Some(mixed)` once diagnosed, `None`
/// for every pair the table below should answer itself.
fn reject_bitwise_operand(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let spelling = binary_operator_spelling(op);
    // The left operand first, so a pair that is wrong on both sides reports
    // once and names the side written first.
    let offender = [lhs, rhs]
        .into_iter()
        .find(|ty| bitwise_operand_is_refused(env.interner.get(*ty)))?;
    report_bitwise_operand(spelling, offender, span, env);
    Some(env.interner.mixed())
}

/// Whether one operand of a bitwise operator is refused outright — the
/// predicate [`reject_bitwise_operand`]'s doc comment explains.
fn bitwise_operand_is_refused(ty: &Ty) -> bool {
    if matches!(ty, Ty::Int | Ty::Uint | Ty::IntLiteral(_)) {
        return false;
    }
    equality_domain(ty).is_some()
}

/// The one diagnostic both bitwise spellings report — the five binary
/// operators through [`reject_bitwise_operand`] and `~` through
/// [`reject_unary_arith_operand`], so that the six agree on their wording as
/// well as on their rule.
fn report_bitwise_operand(spelling: &str, ty: TypeId, span: Span, env: &mut Env<'_>) {
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_BITWISE_NOT_INTEGER,
            format!("`{spelling}` has no meaning for `{described}`"),
        )
        .with_primary(span, "a bitwise operator is over integers")
        .with_help(
            "`rule:types/arithmetic`'s `& | ^ ~ << >>` row is `int` and `uint` only; PHP converts this \
             operand first and Novis never converts by itself, so say it — `$x as int`",
        ),
    );
}

/// `rule:types/arithmetic`: `decimal ⊕ float` is a compile error, on the same grounds
/// `int ⊕ uint` already is — there is no type that represents both operands'
/// values, so the fix is to convert one side and say which. Returns
/// `Some(mixed)` once diagnosed, `None` for every other pair so the caller's
/// own table runs unchanged. Comparison is deliberately *not* routed through
/// here: the same § 3 permits `decimal < 1.5`, because an exact comparison is
/// computable even where a common arithmetic type is not.
pub(crate) fn reject_decimal_float_operands(
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let pair = (env.interner.get(lhs), env.interner.get(rhs));
    if !matches!(pair, (Ty::Decimal, Ty::Float) | (Ty::Float, Ty::Decimal)) {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_DECIMAL_FLOAT_ARITHMETIC,
            "`decimal` and `float` have no representable common type in arithmetic",
        )
        .with_primary(span, "mixed `decimal`/`float` operand")
        .with_help("convert one side explicitly with `as decimal`/`as float`"),
    );
    Some(env.interner.mixed())
}

/// `rule:types/arithmetic`'s last row: `**` with a `decimal` base is a compile error,
/// because a general decimal power has no exact result at a bounded scale —
/// `Core\Decimal::pow` names the rounding instead. A decimal *exponent* is
/// refused by the same diagnostic: the row does not define `int ** decimal`
/// either, and letting it fall through to [`arithmetic_result`]'s decimal row
/// would invent a fractional exponentiation the ADR never granted.
pub(crate) fn power_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if matches!(env.interner.get(lhs), Ty::Decimal) || matches!(env.interner.get(rhs), Ty::Decimal)
    {
        env.diags.report(
            Diagnostic::error(
                code::E_DECIMAL_FLOAT_ARITHMETIC,
                "`**` is not defined on `decimal`",
            )
            .with_primary(span, "`decimal` operand of `**`")
            .with_help("use `Core\\Decimal::pow`, which names the rounding it does"),
        );
        return env.interner.mixed();
    }
    arithmetic_result(lhs, rhs, span, env)
}

pub(crate) fn report_int_uint(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_INT_UINT_ARITHMETIC,
            "`int` and `uint` have no representable common type in arithmetic",
        )
        .with_primary(span, "mixed-signedness operand")
        .with_help("convert one side explicitly with `as int`/`as uint`"),
    );
}

/// `rule:classes/stringable`: every implicit string-conversion site — interpolation,
/// concatenation, `echo`/`print`, `as string`/`(string)` — accepts an object
/// only when its static type provably implements the reserved global
/// `Stringable` interface. Returns without diagnosing for any non-`Ty::Class`
/// operand (including `Ty::Enum`, `mixed`, and a scalar) and for an
/// unmodeled `Core` class, the same scoping [`object_comparison_result`] and
/// [`check_property_access`] already use.
/// Reports the three ways `-`, `+` or `~` can be handed an operand `rule:types/arithmetic`
/// tabulates no row for. One call site, because it is one question asked of
/// one operand and a program is owed one diagnostic for it.
///
/// **An enum value** — a plain enum, a case-subset type, or a union with
/// either in it, `?Size` included — takes `E_ENUM_ARITHMETIC_UNSUPPORTED`,
/// the code the binary operators report for it too ([`report_enum_operand`]).
///
/// **An object** takes `E_TYPE_MISMATCH`: Novis has no operator overloading, so
/// there is no arithmetic an object can take part in — and the first place a
/// program reaches for one is
/// `rule:types/duration-literal`'s `-7d`, which
/// that ADR refuses outright in favour of `->minus(7d)`. Left unchecked it
/// reaches `nvs-codegen`, which panics naming the representation; a
/// diagnostic naming the operator is what the author needs.
///
/// **Every other non-numeric operand** takes `E_UNARY_ARITH_NOT_NUMERIC`,
/// scoped exactly the way [`reject_increment_on_non_numeric`] next door is:
/// only a type whose [`equality_domain`] is known *and* is not the numeric
/// one, so `mixed`, a union (`7 / 2` produces `int|float`), a type variable
/// and an error placeholder all pass through. PHP answers `+"5"`, `-"5"` and
/// `~"ab"` by *converting* the operand first, and `rule:types/conversion` has no implicit
/// conversion for that to be — which is what makes unary `+` safe to lower as
/// the identity it is over a number (`nvs_ir::lower::expr`'s `Plus` arm): the
/// operand it would silently pass through unchanged is refused here instead,
/// naming `as int`.
///
/// Only the three arithmetic prefixes: `!` is `rule:expressions/truthy-positions`'s truthy test, which
/// every type is legal in, and `@` never reaches the checker at all — the
/// parser refuses it as `E0236`.
pub(crate) fn reject_unary_arith_operand(op: UnaryOp, ty: TypeId, span: Span, env: &mut Env<'_>) {
    // The same step-earlier objection the binary operators make, and the same
    // code: `-V::nothing()` has no operand rather than an operand with no row.
    // Unary `+` is refused here too, though it is the identity over every type
    // the table does name — the identity of nothing is still nothing.
    if matches!(env.interner.get(ty), Ty::Void) {
        report_void_operand(span, env);
        return;
    }
    let spelling = match op {
        UnaryOp::Neg => "-",
        UnaryOp::Plus => "+",
        _ => "~",
    };
    // An enum value, nullable or not, takes the code every operator over one
    // reports — `rule:enums/representation` gives a case its backing
    // integer's tag, so a `?Size` operand cannot be answered at run time.
    if carries_an_enum_value(ty, env) {
        report_enum_operand(spelling, ty, span, env);
        return;
    }
    if matches!(
        env.interner.get(ty),
        Ty::Class(..) | Ty::Object | Ty::Shape(_)
    ) {
        let described = env.interner.describe(ty);
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_MISMATCH,
                format!("`{spelling}` has no meaning for `{described}`"),
            )
            .with_primary(span, "an object takes part in no arithmetic")
            .with_help(
                "Novis has no operator overloading; call the member that does this — a \
                 `Core\\Time\\Duration` negates with `->negated()` and subtracts with `->minus(…)`",
            ),
        );
        return;
    }
    // `~` parts from `-` and `+` on the numeric row, and this is the whole of
    // the difference: `rule:types/arithmetic` grants arithmetic over four numeric types
    // and bit operations over two, so a `float` or a `decimal` operand is a
    // number with no bit pattern to complement. It takes the *bitwise* code,
    // because the rule that author needs is that row's rather than "this is
    // not a number" — every other operand of `~` is refused below, with the
    // sentence its two siblings use.
    if op == UnaryOp::BitNot && matches!(env.interner.get(ty), Ty::Float | Ty::Decimal) {
        report_bitwise_operand("~", ty, span, env);
        return;
    }
    let refused = {
        let resolved = env.interner.get(ty);
        !matches!(equality_domain(resolved), None | Some(EqDomain::Numeric))
    };
    if !refused {
        return;
    }
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_UNARY_ARITH_NOT_NUMERIC,
            format!("`{spelling}` has no meaning for `{described}`"),
        )
        .with_primary(span, "a unary arithmetic operator is over numbers")
        .with_help(
            "`rule:types/arithmetic`'s arithmetic is over `int`, `uint`, `float` and `decimal`; PHP \
             converts this operand first and Novis never converts by itself, so say it — \
             `$x as int`",
        ),
    );
}

/// Reports `E_INCREMENT_NOT_NUMERIC` for `++`/`--` on a target that is not one
/// of `rule:types/arithmetic`'s numeric types — the module doc above owns the decision,
/// including why PHP's string increment is not among them.
///
/// An enum value, a nullable one included, takes `E_ENUM_ARITHMETIC_UNSUPPORTED`
/// instead, through [`report_enum_operand`]. Everything else is scoped exactly
/// the way the refusals around it are: only a type whose [`equality_domain`]
/// is known *and* is not the numeric one is refused, so `mixed`, a union
/// (`int|float` is what `7 / 2` produces), a type variable and an error
/// placeholder all pass through and are settled below.
pub(crate) fn reject_increment_on_non_numeric(ty: TypeId, span: Span, env: &mut Env<'_>) {
    if carries_an_enum_value(ty, env) {
        report_enum_operand("++`/`--", ty, span, env);
        return;
    }
    let refused = {
        let resolved = env.interner.get(ty);
        !matches!(equality_domain(resolved), None | Some(EqDomain::Numeric))
    };
    if !refused {
        return;
    }
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_INCREMENT_NOT_NUMERIC,
            format!("`++`/`--` has no meaning for `{described}`"),
        )
        .with_primary(span, "an increment is `± 1`, and this is not a number")
        .with_help(
            "`rule:types/arithmetic`'s arithmetic is over `int`, `uint`, `float` and `decimal`; PHP's \
             string increment does not exist in Novis, because a binding never changes type",
        ),
    );
}

/// `rule:expressions/nullable-conversion-availability`'s
/// class row, which is **absolute**: `as` converts between the types `rule:types/conversion` tabulates and `rule:types/literal-types`'s literal and enum-case types, and none of those
/// is a class. `$obj as ?SomeClass` asks class membership, which `is`
/// plus `rule:types/unions-and-mixed`'s narrowing already answers; `$s as ?Core\Uri` asks for a
/// parse, which is that class's own `tryParse` (§ 3a).
///
/// That second half is a **reversal**. § 3 first admitted a closed two-class
/// *parse roster* — `Core\Uri` and `Core\Uuid` — and this function recorded an
/// `ExprInfo` for it that `nvs-ir` lowered to one non-member `CoreCall`. The
/// ADR withdrew it: `as?` spells a downcast everywhere a reader has met it, so
/// spelling a parse that way inverted the syntax's one intuition for exactly
/// two memorized names, and it never removed the second spelling it was
/// justified by removing. So there is no roster to consult here any more, and
/// nothing to record — a class target is one diagnostic, always.
///
/// Keyed on the **written `?T` sugar**, not on the interned target: § 1
/// deliberately leaves the `Core\Uri|null` union spelling out of the form, so
/// a target reached any other way is not this ADR's and is left alone. A
/// plain `as SomeClass` is left alone too, and goes to
/// [`reject_unconvertible`] instead — that is `rule:types/conversion`'s table having no
/// row for a class type, which is a different sentence reaching a different
/// help. The two never fire on the same expression, since this one runs only
/// on the written `?T` sugar and that one only where the sugar is absent.
fn check_class_target_conversion(ty: &Type, to: TypeId, span: Span, env: &mut Env<'_>) {
    if !is_written_nullable(ty) {
        return;
    }
    let Some(class) = nullable_class_target(to, env) else {
        return;
    };
    // `Core\Uri` and `Core\Uuid` reach this arm like every other class, and
    // the help names `tryParse` for them because that is the member `rule:expressions/try-parse` leaves standing — the whole point of the withdrawal is that they
    // are not special here.
    let help = if nvs_stdlib::registry::TRY_PARSE_CLASSES.contains(&class.as_str()) {
        format!(
            "text becomes one through `{class}::tryParse($s)`, which answers `null` rather \
             than throwing"
        )
    } else {
        "ask `$x is Name` and use the value the test narrows; text becomes a value \
         through that class's own named constructor"
            .to_owned()
    };
    env.diags.report(
        Diagnostic::error(
            code::E_CLASS_CONVERSION_TARGET,
            format!("`{class}` is a class, so `as ?{class}` is not a conversion"),
        )
        .with_primary(span, "converted here")
        .with_help(help),
    );
}

/// Whether the target was written as the `?T` sugar, `(...)` transparent —
/// `nvs_ir::lower`'s own `nullable_target` reads it the same way, and the two
/// have to agree or a target this leaves alone reaches a lowering that
/// expects it to have been decided here.
fn is_written_nullable(ty: &Type) -> bool {
    match &ty.kind {
        TypeKind::Nullable(_) => true,
        TypeKind::Paren(inner) => is_written_nullable(inner),
        _ => false,
    }
}

/// The `T` an `as ?T` target names, or `None` where the target is not that
/// sugar's own shape.
///
/// `?T` interns as exactly `Union([Null, T])` — the checker has no separate
/// nullable type ([`crate::lower::lower_type`]) — so this reads the one
/// non-`null` member back out. A union with more than one is `?("a"|"b")`,
/// whose membership chain `nvs_ir::lower` builds out of the atoms rather than
/// out of one target, and it is left to the caller's `None` path for that
/// reason.
fn nullable_inner_target(to: TypeId, env: &Env<'_>) -> Option<TypeId> {
    let Ty::Union(members) = env.interner.get(to) else {
        return None;
    };
    let mut named = members
        .iter()
        .filter(|member| !matches!(env.interner.get(**member), Ty::Null));
    let only = *named.next()?;
    named.next().is_none().then_some(only)
}

/// The class an `as ?T` target names, or `None` for every other target.
fn nullable_class_target(to: TypeId, env: &Env<'_>) -> Option<String> {
    match env.interner.get(nullable_inner_target(to, env)?) {
        Ty::Class(name, _) => Some(name.to_string()),
        _ => None,
    }
}

/// `rule:expressions/nullable-conversion-availability`'s table, which is `rule:types/conversion`'s asked one row further on:
/// **a conversion that exists and failed is `null`; a conversion that does
/// not exist is a diagnostic** — and a conversion that cannot fail is a
/// diagnostic too, because the `?` then promises a `null` no run produces.
///
/// So the sugar takes both halves. The closed-table refusal
/// ([`reject_unconvertible`]) runs against the `T` *inside* the sugar rather
/// than against the union it interns as, which is what makes `array<int> as
/// ?int` the same diagnostic `array<int> as int` already was; and the
/// cannot-fail row above it has no counterpart in the plain form at all,
/// since `as T` is happy to be total.
///
/// § 3's third row — **any** class target, absolutely — is
/// [`check_class_target_conversion`]'s, whose help names `tryParse` rather
/// than a row. Skipped here so one expression takes one diagnostic.
fn reject_unavailable_nullable_conversion(from: TypeId, to: TypeId, span: Span, env: &mut Env<'_>) {
    if nullable_class_target(to, env).is_some() {
        return;
    }
    let Some(inner) = nullable_inner_target(to, env) else {
        return;
    };
    if !nullable_conversion_is_total(from, to, inner, env) {
        reject_unconvertible(from, inner, span, env);
        return;
    }
    let described_from = env.interner.describe(from);
    let described_to = env.interner.describe(inner);
    env.diags.report(
        Diagnostic::error(
            code::E_NULLABLE_CONVERSION_CANNOT_FAIL,
            format!(
                "`{described_from} as ?{described_to}` cannot fail, so it never answers `null`"
            ),
        )
        .with_primary(span, "converted here")
        .with_help(format!(
            "write `as {described_to}`: `rule:expressions/nullable-conversion-availability` makes a `?T` that is never `null` a compile \
             error, since every reader after it then has to check for a value the conversion \
             cannot produce"
        )),
    );
}

/// Whether `as ?T` would answer `null` on no value at all — `rule:expressions/nullable-conversion-availability`'s
/// "a conversion that **cannot fail**" row.
///
/// The scalar rows are the `false` arms of `nvs_ir::lower::expr`'s own
/// `conversion_can_fail`, and the two have to agree: that function is what
/// picks the `?` helper for a row this one leaves standing, so a row this
/// calls total and it calls fallible would look for a helper that exists
/// while a row the other way round would reach the panic this refusal is
/// here to empty.
///
/// Two rows are this side's alone, because a representation cannot see them:
/// a value that already *is* one of the target's (`?int as ?int`,
/// `Mode::Read as ?Mode` — every case of `Mode` is a `Mode`), and `rule:expressions/nullable-conversion-availability` row 2's literal, enum-case and whole-enum targets, whose membership
/// test is fallible however the base representation reads.
fn nullable_conversion_is_total(from: TypeId, to: TypeId, inner: TypeId, env: &Env<'_>) -> bool {
    // `?int as ?int` is R17's forbidden second spelling and `$i as ?int` the
    // identity; both are the value already satisfying the target.
    if from == to || from == inner {
        return true;
    }
    if let (Ty::EnumCase(case_of, ..), Ty::Enum(named, _)) =
        (env.interner.get(from), env.interner.get(inner))
        && case_of == named
    {
        return true;
    }
    if is_closed_value_target(inner, env) {
        return false;
    }
    let from_kind = conversion_kind(from, env.interner);
    let to_kind = conversion_kind(inner, env.interner);
    use ConvKind::{Bool, Bytes, Decimal, Enum, Float, Int, Null, Object, Str, Uint, Void, Wide};
    match (from_kind, to_kind) {
        // A `void` call is neither an operand nor a target, and saying so is
        // [`reject_unconvertible`]'s job rather than this one's.
        (Void, _) | (_, Void) => false,
        // A target admitting more than one runtime shape is the widening row,
        // which has nothing to check and so nothing to fail.
        (_, Wide) => true,
        // ADR 0035: `as bool` is the condition's own test written out, and it
        // has an answer for every type — an object and an enum case included,
        // § 4 making both always truthy.
        (_, Bool) => true,
        // `rule:types/conversion` row 1: an enum to its own backing type, total and free.
        // Any *other* number is `rule:types/conversion`'s row and throws.
        (Enum(backing), target) => target == enum_backing_kind(backing),
        // `rule:types/conversion`: every `string` is valid UTF-8, so this direction alone
        // is total — `bytes as ?string` is the checked one.
        (Str, Bytes) => true,
        // `rule:types/conversion`'s "anything → `string`" row, "total for scalars". An
        // *object* is the row's exception ("needs `Stringable`, or it throws")
        // and is left fallible here for that reason.
        (Bool | Int | Uint | Float | Decimal | Str | Null, Str) => true,
        // One representation spelled two ways — a literal type and its base, a
        // `tainted`/`secret` value and its plain twin. The closed-value targets
        // returned above, so a same-kind pair left here converts nothing. The
        // `bool` pair is the `as bool` row two arms up rather than a missing
        // one here.
        (Int, Int)
        | (Uint, Uint)
        | (Float, Float)
        | (Decimal, Decimal)
        | (Bytes, Bytes)
        | (Null, Null)
        | (Object, Object) => true,
        _ => false,
    }
}

/// Whether the target is one of `rule:expressions/nullable-conversion-availability` row 2's **closed** sets — a
/// literal type, an enum case, or a whole enum — where the conversion is a
/// membership test that a value of the right representation can still miss.
/// Their base representation reads as free ([`conversion_kind`] folds
/// `"a"` to `Str` and `Mode::Read` to `Enum`), so without this the row above
/// would call the available form total.
fn is_closed_value_target(to: TypeId, env: &Env<'_>) -> bool {
    matches!(
        env.interner.get(to),
        Ty::IntLiteral(_)
            | Ty::StringLiteral(_)
            | Ty::True
            | Ty::False
            | Ty::Enum(..)
            | Ty::EnumCase(..)
    )
}

/// `rule:types/conversion`'s conversion table is **closed**, and this is the refusal that
/// says so. `as` "is total in intent and checked in fact: it either produces a
/// value of the target type or throws" — so a pair naming no row has nothing to
/// produce and nothing to throw, and the honest answer is a diagnostic where it
/// is written rather than a wrong value or a panic below.
///
/// The rows are that table's own, plus the three it delegates to: `rule:types/conversion`
/// for `string` ↔ `bytes`, `rule:types/conversion` for `decimal`, and `rule:types/conversion` for an
/// enum and its backing type. Two are not in any ADR's table and are here
/// because they are true of every type — `rule:expressions/truthy-positions`'s `as bool`, which is the
/// condition's own test said out loud, and the widening into a target that
/// admits more than one runtime shape.
///
/// This is what leaves `nvs_ir::lower::expr`'s `Lowering::convert` catch-all
/// only the two gaps its own message names (`array<T> as array<U>`, and ADR
/// 0024 § 5's `Core\Html\Markup`). Before it, `true as int`, `$xs as string`,
/// `$i as bytes`, `$case as float` and `null as string` each panicked there,
/// and `$foo as Bar` between two unrelated classes was worse than a panic: the
/// representations are equal, so it took the free `from == to` row and read
/// `Bar`'s slot list off a `Foo`.
fn reject_unconvertible(from: TypeId, to: TypeId, span: Span, env: &mut Env<'_>) {
    // The identical type is always the free no-op row, whatever its kind —
    // `$s as string` after a narrowing, `$f as Foo`. Only a *different* class
    // is the unsound reinterpret below.
    if from == to {
        return;
    }
    let from_kind = conversion_kind(from, env.interner);
    let to_kind = conversion_kind(to, env.interner);
    // A class target is not judged by the table at all — see
    // [`reject_unrelated_class_conversion`] for the three rows that exist and
    // why relatedness rather than a row is what decides them.
    if to_kind == ConvKind::Object {
        reject_unrelated_class_conversion(from, to, span, env);
        return;
    }
    if conversion_row_exists(from_kind, to_kind) {
        // `rule:types/conversion`'s `array<T> as array<U>` row is the one that is checked
        // element by element at run time, and what checks one element is its
        // runtime *tag* — so the row exists only for a `U` a tag can decide.
        if to_kind == ConvKind::Array {
            reject_uncheckable_element_type(to, span, env);
        }
        return;
    }
    let described_from = env.interner.describe(from);
    let described_to = env.interner.describe(to);
    let help = conversion_help(from_kind, to_kind);
    env.diags.report(
        Diagnostic::error(
            code::E_NO_CONVERSION,
            format!("`{described_from}` cannot be converted to `{described_to}`"),
        )
        .with_primary(span, "converted here")
        .with_help(help),
    );
}

/// A class target, which `rule:types/conversion` tabulates no row *producing* — and yet
/// three shapes of `as` legitimately name one, so a blanket refusal is wrong:
///
/// * **A downcast out of an erased view.** `$erased as Plain` over a plain
///   `object`, and `$other as Cell` over the interface `rule:classes/comparable`'s `compareTo`
///   receives, are the two spellings `rule:types/erased-member-access` leaves standing. Both erase
///   to one pointer representation, so the conversion runs nothing and the
///   check happens at the member access instead (`InstKind::SlotGet`).
/// * **`Core\Html\Markup`**, which decides for itself: `rule:security/tainted-qualifier`'s
///   `as Core\Html\Markup` is a source-literal `string` and has its own
///   diagnostic (`E_MARKUP_REQUIRES_LITERAL`, in [`crate::expr::quals`])
///   saying so. Exempted from the disjointness question exactly as
///   [`require_stringable_object`] exempts it, and for the same reason — the
///   owning rule is the rule, not this table. It is a *lift*, which is why it
///   is the one `Core` target decided by a rule rather than by a test.
/// * **Any other `Core` class with a descriptor** — the roster
///   [`crate::expr::testable_core_class`] answers for — which is a downcast
///   target on exactly the terms a declared class is, and is asked the same
///   disjointness question. The descriptor it tests against is the process-wide
///   one `nvs_stdlib::class_descriptors` publishes rather than one the unit
///   laid out, and `InstKind::ClassTest` already takes it.
/// * **The identical type**, returned by [`reject_unconvertible`] before this
///   is reached.
///
/// Two refusals are left, and they are the two halves of "can this conversion
/// be *checked*". A target naming a declared class is checked by testing the
/// value's runtime class, so what it refuses is the pair with **no value in
/// common** — `types_are_disjoint`'s question, `rule:expressions/disjoint-comparison-refused`'s, asked of a
/// conversion rather than of an equality. `$foo as Bar` between two unrelated
/// classes was worse than a panic before this refusal, because both erase to
/// `nvs_ir::ty::Ty::Object` and the conversion therefore took
/// `Lowering::convert`'s free `from == to` row: nothing ran, and `Bar`'s slot
/// list was then read off a `Foo`'s allocation.
///
/// A target naming **no testable class** — plain `object`, a `callable`, or a
/// `Core` namespace class, none of which has a descriptor to compare against —
/// has no such test to run, so the operand has to be an object *already*. From
/// anything wider the conversion could only assert the tag it cannot verify,
/// which is the same type confusion one step earlier, and it is
/// [`code::E_UNTESTABLE_CONVERSION_TARGET`] where it is written. `$plain as
/// object` stays the free widening row it always was. A shape has no
/// descriptor either, but its fields are the test `is` already runs, so it is
/// [`super::type_test::accept_shape_conversion`]'s row instead.
fn reject_unrelated_class_conversion(from: TypeId, to: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(to).clone() else {
        // Plain `object`, a shape and a `callable` are the other three
        // `ConvKind::Object` targets, and none of them names a class to test
        // against, nor to be unrelated to. A shape is tested by its fields.
        reject_untestable_object_target(from, to, span, env);
        return;
    };
    if qname.is_core() {
        // `rule:core-classes/html-auto-escape`'s `as Core\Html\Markup` is that section's own row and
        // `crate::expr::quals` owns it end to end — a source-literal `string`
        // and nothing else, `E_MARKUP_REQUIRES_LITERAL` for anything computed.
        // It is a lift rather than a test, so it is the one `Core` target this
        // function decides nothing about.
        if qname.to_string() == crate::CORE_HTML_MARKUP_CLASS {
            return;
        }
        // Every other `Core` class is a downcast target on a declared class's
        // terms: `crate::expr::testable_core_class` is the roster whose
        // descriptor the process publishes, so there is a class to test against
        // and the disjointness question below is the right one to ask. A `Core`
        // namespace class is on no roster — nothing is ever an instance of it —
        // and keeps `E_UNTESTABLE_CONVERSION_TARGET`.
        if !crate::expr::testable_core_class(&qname) {
            reject_untestable_object_target(from, to, span, env);
            return;
        }
    }
    if !types_are_disjoint(from, to, env) {
        return;
    }
    let described_from = env.interner.describe(from);
    env.diags.report(
        Diagnostic::error(
            code::E_NO_CONVERSION,
            format!("`{described_from}` cannot be converted to `{qname}`"),
        )
        .with_primary(span, "converted here")
        .with_help(
            "`rule:types/conversion` tabulates no conversion into a class, and these two share no value at \
             all: ask `$x is Name` and use the value the test narrows, or call that \
             class's own named constructor",
        ),
    );
}

/// The half of [`reject_unrelated_class_conversion`] that fires for a target
/// with no class descriptor behind it. Read that function's doc comment first.
///
/// A shape target is tested by its fields instead, and
/// [`super::type_test::accept_shape_conversion`] decides it first. For every
/// other target here, the operand already being an object is the free
/// widening row and the only accepted operand: `$plain as object`, `$plain as
/// callable`, `$uri as object` are one pointer on both sides and run nothing
/// at all. Everything else — `mixed`, a `?T`, a scalar, an `array<T>` — would
/// have to take a tag on trust, and `nvs_ir` has no instruction that could
/// check it.
fn reject_untestable_object_target(from: TypeId, to: TypeId, span: Span, env: &mut Env<'_>) {
    if super::type_test::accept_shape_conversion(from, to, span, env)
        || conversion_kind(from, env.interner) == ConvKind::Object
    {
        return;
    }
    let described_from = env.interner.describe(from);
    let described_to = env.interner.describe(to);
    env.diags.report(
        Diagnostic::error(
            code::E_UNTESTABLE_CONVERSION_TARGET,
            format!("`{described_from}` cannot be converted to `{described_to}`"),
        )
        .with_primary(span, "converted here")
        .with_help(
            "`rule:types/conversion` has no checked row into this target, because nothing about it can \
             be tested on the value at run time: convert to a declared class, a `Core` class with \
             instances such as `Core\\Time\\Date`, or a shape such as `{id: int}` whose fields \
             carry no qualifier — those are the checked ways out of `mixed`",
        ),
    );
}

/// How many levels of `array<…>` nesting an `array<U>` target may name.
///
/// The number is a *representation* fact rather than a language one:
/// `nvs_ir::lower::array_element_tags` packs one four-bit tag per level into a
/// `u64`, which holds `64 / 4` of them, and the runtime walk
/// (`nvs_runtime`'s `to_array_of`) reads the same word back. Sixteen levels of
/// `array<array<…>>` is far past anything a program writes, so the limit is
/// stated here — the one place a conversion's target is judged — rather than
/// designed around.
const ARRAY_ELEMENT_TAG_LEVELS: usize = u64::BITS as usize / 4;

/// `rule:types/conversion`'s `array<T> as array<U>` row, refused where the element type
/// `U` is one no runtime tag decides.
///
/// The row's whole content is "every element must satisfy `U`", checked once
/// per element on the way through, and the check available at that point is
/// the *same* one a closure parameter's entry check runs
/// (`nvs_ir::lower::param_tag_nibble`): a tag, four bits wide, with no room
/// for anything else. So the element types that convert are exactly the ones
/// whose whole meaning is their tag — `bool`, `int`, `uint`, `float`,
/// `decimal`, `string`, `bytes`, `null`, `mixed`, and an `array<…>` of any of
/// them, nested to [`ARRAY_ELEMENT_TAG_LEVELS`].
///
/// Everything else is refused **where it is written**, and each for a reason
/// this refusal is the safe answer to rather than a shape nobody got to:
///
/// * A **class**, a shape, a `callable` or plain `object` erase to one
///   pointer, so a tag proves objecthood and never the label a named-class
///   binding then reads at a *fixed offset*. That is
///   [`reject_untestable_object_target`]'s type confusion, one container in.
/// * An **enum** erases to its backing integer, so a tag would admit any
///   integer as a case — where `rule:types/conversion`'s own `mixed → EnumName` row
///   throws for a value no case names.
/// * A **literal type** or a **union** (`rule:types/literal-types`'s closed sets,
///   `?T`, `int|string`) admits some values of its representation and not
///   others, which is again more than a tag says. `mixed` is not in that list
///   and is accepted: it is `rule:types/grammar`'s one unchecked position, so an
///   element of it is checked by every reader instead.
///
/// What to write instead is in the help, and it is the same shape either way:
/// `array<mixed>` converts, and each element is converted where it is read.
fn reject_uncheckable_element_type(to: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Array(element) = env.interner.get(to) else {
        return;
    };
    let mut element = *element;
    for level in 0..ARRAY_ELEMENT_TAG_LEVELS {
        match env.interner.get(element) {
            Ty::Null
            | Ty::Bool
            | Ty::Int
            | Ty::Uint
            | Ty::Float
            | Ty::Decimal
            | Ty::String
            | Ty::Bytes
            | Ty::Mixed => return,
            Ty::Array(inner) if level + 1 < ARRAY_ELEMENT_TAG_LEVELS => element = *inner,
            _ => break,
        }
    }
    let described_from = env.interner.describe(to);
    let described_element = env.interner.describe(element);
    env.diags.report(
        Diagnostic::error(
            code::E_UNTESTABLE_CONVERSION_TARGET,
            format!("`{described_from}` cannot be an `as` target"),
        )
        .with_primary(span, "converted here")
        .with_help(format!(
            "`rule:types/conversion`'s `array<T> as array<U>` row checks every element against `U` as it \
             walks, and what checks one element is its runtime tag — which `{described_element}` \
             is not decided by: convert to `array<mixed>` and convert each element where it is \
             read"
        )),
    );
}

/// Which row of `rule:types/conversion`'s table a type can appear in — deliberately
/// coarser than [`Ty`], because the table is written over the *language's*
/// types rather than over an interned identity.
///
/// The union fold and the atom arms mirror `nvs_ir::lower::erase_checked_ty`
/// on purpose: a type that erases to one runtime representation converts as
/// that representation, and one that admits more than one is
/// [`ConvKind::Wide`], where the conversion is picked from the operand's tag
/// at run time and no static pair can be refused.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConvKind {
    Bool,
    Int,
    Uint,
    Float,
    Decimal,
    Str,
    Bytes,
    Null,
    Void,
    Array,
    /// An enum or one of its cases, carrying the backing type `rule:types/conversion`'s
    /// two rows are *about* — `Mode::Read as int` is a row and
    /// `Mode::Read as uint` is not.
    Enum(crate::enums::EnumBacking),
    /// A class, a shape, a `callable` or plain `object`: one pointer
    /// representation, and no row of the table produces one.
    Object,
    /// `class<T>` — `rule:types/class-reference`, and the one target kind that *is* produced by
    /// rows of its own. Deliberately not [`Self::Object`]: a descriptor is one
    /// pointer like an instance is, but the two rows that reach it check a
    /// hierarchy at run time and no row of § 2 reaches an instance at all.
    ///
    /// Unparameterised, because the table asks only which rows exist. `class<U>
    /// → class<T>` is one row whatever `U` and `T` are — a narrowing, checked
    /// against the descriptor — and where the pair is decidable at compile time
    /// [`reject_impossible_class_reference_conversion`] is what decides it.
    ClassRef,
    /// `property<T>` — `rule:types/property-key`, and [`Self::ClassRef`]'s story exactly:
    /// its own kind rather than [`Self::Str`]'s, because a key is not a string
    /// and `as` is the only door into one. Unparameterised for that sibling's
    /// reason too — the table asks only which rows exist, and `property<U> →
    /// property<T>` is one row whatever `U` and `T` are, with the name checked
    /// against `T`'s roster at run time.
    ///
    /// Where the operand is written out, the pair is decided at compile time by
    /// [`check_property_key_conversion`] instead, which is § 2's second
    /// sentence and the reason a hand-written key costs nothing at run time.
    PropertyKey,
    /// More than one runtime shape — `mixed`, a `?T`, a heterogeneous union,
    /// or a type this table does not model. **Never refused, on either side**:
    /// the row is chosen from the value's tag at run time.
    Wide,
}

/// [`ConvKind`] for one type. See that enum's doc comment for why the fold is
/// the one `erase_checked_ty` performs.
fn conversion_kind(id: TypeId, interner: &TypeInterner) -> ConvKind {
    match interner.get(id) {
        Ty::Bool | Ty::True | Ty::False => ConvKind::Bool,
        Ty::Int | Ty::IntLiteral(_) => ConvKind::Int,
        Ty::Uint => ConvKind::Uint,
        Ty::Float => ConvKind::Float,
        Ty::Decimal => ConvKind::Decimal,
        Ty::String
        | Ty::TaintedString
        | Ty::SecretString
        | Ty::SecretTaintedString
        | Ty::StringLiteral(_) => ConvKind::Str,
        Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes => ConvKind::Bytes,
        Ty::Null => ConvKind::Null,
        Ty::Void => ConvKind::Void,
        Ty::Array(_) => ConvKind::Array,
        Ty::ClassRef(_) => ConvKind::ClassRef,
        Ty::PropertyKey(_) => ConvKind::PropertyKey,
        Ty::Enum(_, backing) | Ty::EnumCase(_, backing, _) => ConvKind::Enum(*backing),
        Ty::Class(..) | Ty::Object | Ty::Shape(_) | Ty::Callable | Ty::CallableSig { .. } => {
            ConvKind::Object
        }
        Ty::Union(members) => {
            let mut shared: Option<ConvKind> = None;
            for member in members {
                let kind = conversion_kind(*member, interner);
                match shared {
                    None => shared = Some(kind),
                    Some(seen) if seen == kind => {}
                    _ => return ConvKind::Wide,
                }
            }
            shared.unwrap_or(ConvKind::Wide)
        }
        _ => ConvKind::Wide,
    }
}

/// The table itself, one arm per ADR row. Read [`reject_unconvertible`]'s doc
/// comment first: everything here is a row of `rule:types/conversion` or of one of the
/// three ADRs it delegates to, and `false` is the absence of a row rather than
/// a judgement of its own.
fn conversion_row_exists(from: ConvKind, to: ConvKind) -> bool {
    use ConvKind::{
        Array, Bool, Bytes, ClassRef, Decimal, Enum, Float, Int, Null, Object, PropertyKey, Str,
        Uint, Void, Wide,
    };
    match (from, to) {
        // A `void` call has no value, so it is neither an operand a row can
        // read nor a target a row can produce. Refused on both sides rather
        // than left to the widening row below, where `Helper::nothing() as
        // mixed` tagged a value that was never defined.
        (Void, _) | (_, Void) => false,
        // A class target never reaches this table — [`reject_unconvertible`]
        // sends it to [`reject_unrelated_class_conversion`] first, because
        // what decides one is whether the two types share a value rather than
        // which row of § 2 they name.
        (_, Object) => true,
        // Either side admits more than one runtime shape, so the row is the
        // operand's tag's and no static pair can be judged. `mixed` is the
        // whole of `rule:types/unions-and-mixed` here.
        (Wide, _) | (_, Wide) => true,
        // `rule:expressions/truthy-positions`, which makes a condition the one place a value is tested
        // without `as` — so `as bool` is that same test written out, and it
        // has an answer for every type the table above did not already
        // exclude.
        (_, Bool) => true,
        // `rule:types/class-reference`'s two rows, and the whole of what produces a class
        // reference — which is what makes `as` its only source. The `string`
        // row is the door: the text must name `T` or a class that is a `T`, and
        // it throws where it does not. The `class<U>` row is a narrowing,
        // checked against the descriptor the same way.
        //
        // The refusal below is written out rather than left to this function's
        // final `_ => false`, because it is a *decision* — every other operand
        // reaching a `class<T>` target has no row, and that is the sentence
        // that keeps `Foo::class` an ordinary `string`.
        (Str | ClassRef, ClassRef) => true,
        (_, ClassRef) => false,
        // `rule:types/property-key`'s three rows, and the whole of what produces a property
        // key — which is what makes `as` its only source, exactly as the two
        // rows above make it a class reference's. The `string` row is the door:
        // the text must name a public declared property of `T`, and it throws
        // where it does not. The `property<U>` row is a narrowing, checked the
        // same way against `T`'s roster. Refused below rather than left to the
        // final `_ => false` for the sibling's reason: it is the decision that
        // keeps a member name an ordinary `string` everywhere else.
        (Str | PropertyKey, PropertyKey) => true,
        (_, PropertyKey) => false,
        // § 2's third row, and the direction that cannot go wrong: a checked
        // name is still a name. Total, so it is written here rather than added
        // to the "anything → `string`" arm below, where it would read as one of
        // the scalars.
        (PropertyKey, Str) => true,
        // `rule:types/class-reference`'s own third row, the sibling of the one above and total
        // for the same reason: a class reference *is* a descriptor, and a
        // descriptor carries the class's fully qualified name. So this closes
        // the round trip the `string` row opens — `$name as class<Animal> as
        // string` is the name it started from, and `$obj::class` is the same
        // read one representation over.
        //
        // It answers the *descriptor's* class, which for a reference narrowed
        // from a subclass is that subclass rather than the `T` it was checked
        // against — the same rule `$obj::class` follows, and the reason this is
        // not the annotation's own name folded at compile time.
        (ClassRef, Str) => true,
        // `rule:types/conversion`'s "anything → `string`" row: total for scalars, and an
        // object needs `Stringable` — which `require_stringable_object` has
        // already asked at this same span. `null` is in the row for the reason
        // `E_NO_STRING_FORM`'s doc gives: PHP renders it as the empty string
        // and a `?string` holding one already does.
        (Bool | Int | Uint | Float | Decimal | Str | Null | Object, Str) => true,
        // `rule:types/conversion`'s numeric rows, each exact-or-throws.
        (Int, Uint) | (Uint, Int) | (Int | Uint, Float) | (Float, Int | Uint) => true,
        (Str, Int | Uint | Float) => true,
        // `rule:types/conversion`'s rows. `decimal → string` is in the "anything →
        // `string`" row above with the other scalars.
        (Int | Uint | Float | Str, Decimal) | (Decimal, Int | Uint | Float) => true,
        // `rule:types/conversion`'s pair, and the only two rows either side appears in.
        (Str, Bytes) | (Bytes, Str) => true,
        // `rule:types/conversion` refuses enum → a *different* enum by name rather than
        // by backing type, and `reject_enum_to_enum_conversion` is where that
        // sentence lives — so this leaves the pair alone rather than reporting
        // a second, vaguer diagnostic on the same span. An enum case converted
        // to its own enum reaches here too, and is the free row.
        (Enum(_), Enum(_)) => true,
        // `rule:types/conversion` row 1: an enum to its own underlying type, total and
        // free. Not to any *other* number — `$case as float` is that row and
        // then `rule:types/conversion`'s, written out.
        (Enum(backing), target) => target == enum_backing_kind(backing),
        // `rule:types/conversion` row 2, the same composition `Lowering::convert`
        // performs: the operand converts to the enum's backing scalar by
        // whichever row above applies, and the tag goes back on for free.
        (source, Enum(backing)) => conversion_row_exists(source, enum_backing_kind(backing)),
        // `rule:types/conversion`'s O(n) element row.
        (Array, Array) => true,
        // Two spellings of one representation — a literal type and its base,
        // `secret bytes` and `bytes`. `rule:types/literal-types` and `rule:security/secret-qualifier` both make
        // these free, and the qualifier rule that runs after this one
        // ([`super::quals`]) is what decides the result's own qualifiers. The
        // `bool` and `string` pairs are already true two rows up, so naming
        // them again here is an unreachable arm rather than a missing one.
        (Int, Int)
        | (Uint, Uint)
        | (Float, Float)
        | (Decimal, Decimal)
        | (Bytes, Bytes)
        | (Null, Null) => true,
        _ => false,
    }
}

/// The [`ConvKind`] of an enum's backing type — `rule:enums/one-backing-type` gives every enum
/// exactly one, `int` or `uint`.
fn enum_backing_kind(backing: crate::enums::EnumBacking) -> ConvKind {
    match backing {
        crate::enums::EnumBacking::Int => ConvKind::Int,
        crate::enums::EnumBacking::Uint => ConvKind::Uint,
    }
}

/// What to write instead, for each shape that reaches [`reject_unconvertible`].
/// Ordered operand-first where the operand is the whole reason there is no row,
/// target-first otherwise.
fn conversion_help(from: ConvKind, to: ConvKind) -> &'static str {
    use ConvKind::{Array, Bool, Bytes, ClassRef, Enum, Null, Object, PropertyKey, Str, Void};
    match (from, to) {
        (_, PropertyKey) => {
            "`rule:types/property-key` gives `property<T>` exactly two sources: a `string` naming a public \
             declared property of `T`, and a narrowing from another key — so make the name first \
             and convert that"
        }
        (PropertyKey, _) => {
            "a property key converts back to the `string` it names and to nothing else \
             (`rule:types/property-key`): read the property through it — `$obj->$key` — or convert to \
             `string` first"
        }
        (_, ClassRef) => {
            "`rule:types/class-reference` gives `class<T>` exactly two sources: a `string` naming a class that \
             is a `T`, and a narrowing from another class reference — so make the name first \
             (`Foo::class`, or the text a request carried) and convert that"
        }
        (ClassRef, _) => {
            "a class reference is a class descriptor: `rule:types/class-reference-sites`'s three sites take the value \
             itself, `as string` reads the class's own name off it, and `Core\\Reflect` answers \
             every other reflective question about a class (`rule:classes/no-free-functions-or-constants`)"
        }
        (Void, _) => {
            "a call that returns `void` has no value at all, so there is nothing here to convert"
        }
        (_, Void) => "`void` is a return type, not a value's type — there is nothing to produce",
        (_, Object) => {
            "`rule:types/conversion` tabulates no conversion into a class: ask `$x is Name` and use \
             the value the test narrows, or call that class's own named constructor"
        }
        (Array, Str) => {
            "an `array<T>` has no text of its own; render it — `Core\\Json::encode($a)` — or \
             build the string from its elements"
        }
        (Enum(_), Str) => {
            "an enum case is a named integer (`rule:enums/no-class-machinery`), not text — `$case as int as string`, \
             or a member of your own that names it"
        }
        (Enum(_), _) => {
            "`rule:types/conversion` converts an enum to its own backing type alone — convert to that first, \
             then to the type you want"
        }
        (_, Array) => {
            "`rule:types/conversion`'s only row producing an `array<T>` is another `array<U>` — text becomes \
             one through `Core\\Json::decode($s)`"
        }
        (_, Bytes) => {
            "`rule:types/conversion` gives `bytes` exactly one source, a `string`: render the value first — \
             `$v as string as bytes`"
        }
        (Bytes, _) => {
            "`rule:types/conversion` gives `bytes` exactly one target, a `string` — `$b as string`, and then \
             the type you want"
        }
        (Bool, _) => {
            "`bool` converts to `string` and to `bool` alone (`rule:types/conversion`); a number out of a \
             predicate is a branch said out loud — `$b ? 1 : 0`"
        }
        (Null, _) => {
            "`null` converts to `string` — the empty one — and to `bool`; every other target \
             would be a substituted default, and `as` never substitutes one"
        }
        _ => {
            "`rule:types/conversion`'s table is the whole list of conversions there is, and it has no row \
             for this pair"
        }
    }
}

/// `rule:types/conversion`: "`EnumName` → a different `EnumName`, even with the same
/// underlying type — **rejected**, even via `as`." Two enums sharing an
/// underlying type are not the same closed set, so this refuses the
/// conversion outright rather than letting [`ExprKind::Conversion`]'s
/// ordinary `lower_type` result stand unchecked; converting the same enum to
/// itself, or to/from anything that isn't `Ty::Enum` (its underlying type,
/// `mixed`, a checked-throw source) is untouched.
pub(crate) fn reject_enum_to_enum_conversion(
    from: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let (Ty::Enum(from_q, _), Ty::Enum(to_q, _)) =
        (env.interner.get(from).clone(), env.interner.get(to).clone())
    else {
        return;
    };
    if from_q == to_q {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ENUM_CONVERSION_UNSUPPORTED,
            format!(
                "`{from_q}` cannot be converted to `{to_q}`; two different enums are never \
                 interconvertible, even via `as`"
            ),
        )
        .with_primary(span, "converted here")
        .with_help("write an explicit `match` naming every case instead"),
    );
}

/// `rule:types/literal-types`: a
/// checked `as` into a closed set of literals or enum cases, whose operand
/// names one value and that value is not in the set — `"z" as "a"|"b"`,
/// `Mode::Admin as Mode::Read|Mode::Write`. Nothing about it is conditional at
/// run time: it would compile and then throw on every execution, so the
/// author is told now.
///
/// Both halves have to be closed for that to be provable. The **target** is
/// closed only when every atom is a literal or a case ([`closed_set_atoms`]) —
/// one wider atom (`string`, or the `null` an `as ?T` adds) is a member the
/// operand may well reach. The **operand** is closed only when the expression
/// itself names a single value ([`conversion_operand_singleton`]), since
/// `$s as "a"|"b"` over a plain `string` is exactly § 4's checked row and has
/// to compile.
///
/// This is deliberately not [`types_are_disjoint`]'s business.
/// `rule:expressions/disjoint-comparison-refused` puts a literal type in its base type's domain, so `$mode == "z"` stays
/// an ordinary run-time string comparison; what is refused here is a
/// *conversion* that can only throw, which is a different question reaching a
/// different answer.
fn reject_impossible_literal_conversion(
    inner: &Expr,
    inner_ty: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let Some(accepted) = closed_set_atoms(to, env.interner) else {
        return;
    };
    let Some(operand) = conversion_operand_singleton(inner, inner_ty, env) else {
        return;
    };
    if accepted.contains(&operand) {
        return;
    }
    if int_literal_reaches_a_case(operand, &accepted, env) {
        return;
    }
    // § 6: the accepted set is generated from the type, never written per
    // site, so every atom is rendered by the interner that holds it.
    let cases_only = accepted
        .iter()
        .all(|id| matches!(env.interner.get(*id), Ty::EnumCase(..)));
    let listed: Vec<String> = accepted
        .iter()
        .map(|id| format!("`{}`", env.interner.describe(*id)))
        .collect();
    let code = if cases_only {
        code::E_ENUM_CASE_SUBSET_MISMATCH
    } else {
        code::E_LITERAL_TYPE_MISMATCH
    };
    let named = env.interner.describe(operand);
    env.diags.report(
        Diagnostic::error(
            code,
            format!("`{named}` is not one of {}", listed.join(", ")),
        )
        .with_primary(span, "converted here")
        .with_help(
            "write one of the accepted values, or widen the target type to include this one",
        ),
    );
}

/// An `int` literal converted into a set of enum cases, one of which carries
/// that value: `1 as Mode::Read|Mode::Write` reaches `Mode::Write` on every
/// execution, so it is the opposite of [`reject_impossible_literal_conversion`]'s
/// row — the operand settles the question by itself and settles it in the set's
/// favour. The check is over the cases' *values* because that is what the
/// conversion tests at run time: `rule:types/conversion` makes `int → Mode` a
/// checked conversion over the backing type, and a set of cases narrows which
/// values pass without changing what is compared. [`Ty::EnumCase`] carries the
/// case's name rather than its value, and names the [`crate::enums::EnumTable`]
/// lookup below as the way to the value for anything that needs it.
///
/// This does not fold an int literal into an enum case anywhere else: assigning
/// `1` to a `Mode` stays refused, which is the hole § 3 keeps shut.
fn int_literal_reaches_a_case(operand: TypeId, accepted: &[TypeId], env: &Env<'_>) -> bool {
    let Ty::IntLiteral(value) = *env.interner.get(operand) else {
        return false;
    };
    accepted.iter().any(|id| {
        let Ty::EnumCase(qname, _, case) = env.interner.get(*id) else {
            return false;
        };
        match env.enums.case(qname, case) {
            Some(crate::enums::EnumValue::Int(written)) => written == value,
            Some(crate::enums::EnumValue::Uint(written)) => {
                u64::try_from(value).is_ok_and(|operand| operand == written)
            }
            None => false,
        }
    })
}

/// The atoms of a target type that is **entirely** literals and enum cases,
/// in the order the type itself states them, or `None` for every other
/// target — which is what keeps
/// [`reject_impossible_literal_conversion`] to the one row it can prove.
fn closed_set_atoms(to: TypeId, interner: &TypeInterner) -> Option<Vec<TypeId>> {
    let atoms: Vec<TypeId> = match interner.get(to) {
        Ty::Union(members) => members.clone(),
        _ => vec![to],
    };
    atoms
        .iter()
        .all(|id| {
            matches!(
                interner.get(*id),
                Ty::StringLiteral(_) | Ty::IntLiteral(_) | Ty::EnumCase(..) | Ty::True | Ty::False
            )
        })
        .then_some(atoms)
}

/// The one value the conversion's *operand* names, or `None` where it names
/// more than one.
///
/// The first arm is the placement in [`infer_conversion`] answering for
/// itself: an operand the target accepts has already **taken** one of the
/// target's own atoms, so it is in hand with nothing to re-derive. Everything
/// after it is the failing side of that same placement, where the literal has
/// widened back to its base and the value it named has to be recovered from
/// the expression — [`literal_self_type`] for § 1's two literals, and the
/// recorded [`crate::expr_table::ExprInfo::EnumCase`] for § 3's case. That
/// record rather than the expression's shape, because `Core\X::SOME_CONST` is
/// the same `ClassConstAccess` node and may be enum-typed without naming a
/// case at all.
fn conversion_operand_singleton(
    inner: &Expr,
    inner_ty: TypeId,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    if matches!(
        env.interner.get(inner_ty),
        Ty::StringLiteral(_) | Ty::IntLiteral(_) | Ty::EnumCase(..) | Ty::True | Ty::False
    ) {
        return Some(inner_ty);
    }
    if let Some(literal) = literal_self_type(inner, env) {
        return Some(literal);
    }
    let ExprKind::ClassConstAccess { name, .. } = &inner.kind else {
        return None;
    };
    if !matches!(
        env.exprs.lookup(inner.span),
        Some(crate::expr_table::ExprInfo::EnumCase { .. })
    ) {
        return None;
    }
    let Ty::Enum(qname, backing) = env.interner.get(inner_ty).clone() else {
        return None;
    };
    let case = span_text(env.src, *name).to_owned();
    Some(env.interner.enum_case(qname, backing, case))
}

/// `rule:types/conversion`'s "anything → `string`" row, at the four sites that take it
/// *implicitly*: `.`, `.=`, an interpolated piece and `echo`/`print`. The row
/// reads "total for scalars; an object needs `Stringable`", and this is both
/// halves of it — [`require_stringable_object`] for the object one, and the
/// refusal below for the four types the row does not reach at all.
///
/// Those four are `bytes`, `array<T>`, an enum case and a `void` call, and
/// each has a spelling that says what was meant. They are refused here rather
/// than below because `nvs_ir::lower::expr`'s `concat_operand` has no row for
/// any of them and could only panic — this refusal is what leaves that
/// function's catch-all no reachable target.
///
/// A `null` operand is **not** refused: it renders as the empty string, which
/// is both PHP's answer and the one a `?string` holding `null` already gets
/// from `Helper::TaggedToString` at run time. Refusing the static case while
/// the dynamic one prints nothing would be a divergence from PHP *and* from
/// Novis's own behaviour on the same value.
pub(crate) fn require_stringable(ty: TypeId, span: Span, env: &mut Env<'_>) {
    require_stringable_object(ty, span, env);
    let help = match env.interner.get(ty) {
        Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes => {
            "`rule:types/conversion` makes that conversion explicit, because which encoding the octets \
             are in is a decision — `$b as string`"
        }
        Ty::Array(_) => {
            "an `array<T>` has no text of its own; render it — `Core\\Json::encode($a)` — or \
             build the string from its elements"
        }
        Ty::Enum(..) | Ty::EnumCase(..) => {
            "an enum case is a named integer (`rule:enums/no-class-machinery`), not text — `$case as int`, or a \
             member of your own that names it"
        }
        Ty::Void => {
            "a call that returns `void` has no value at all, so there is nothing here to \
             print"
        }
        _ => return,
    };
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_NO_STRING_FORM,
            format!("`{described}` has no string form"),
        )
        .with_primary(span, "converted to `string` here")
        .with_help(help),
    );
}

/// `rule:core-classes/html-escape-answers-markup` and `rule:core-classes/html-to-source`, at the five sites that would turn a `Core\Html\Markup` back
/// into text *without* the sink being the one asking: `.`, `.=`, an
/// interpolated piece and `as string`.
///
/// The carrier renders — [`require_stringable_object`] says so, and it has to,
/// because `echo $markup` writing the markup out raw is the whole reason the
/// type exists. What this refuses is every *other* route to the same bytes.
/// § 2's own example is the one that matters: `"Hello " . $m` would flatten the
/// carrier into a `string`, which the HTML sink then escapes, and the program
/// ships `&amp;amp;` — the bug `rule:security/launderer-answers-a-carrier` removes, with `.` spelling it instead
/// of `escape`. § 3 refuses `as string` for the same reason in one keystroke
/// fewer.
///
/// **`Core\Cli\Text` is deliberately not refused here**, and the asymmetry is
/// `rule:security/launderer-answers-a-carrier`'s table rather than an oversight: the terminal's escape is
/// *idempotent*, so a `Text` flattened into a `string` and re-neutralized by
/// the sink comes out as the same bytes. The hazard is the second application
/// changing the output, and one sink over it does not.
///
/// `Core\Html::toSource` is the way through, and it is a member so that the
/// route is greppable and carries a written reason — which is what the help
/// text below says at every one of these sites.
pub(crate) fn reject_carrier_as_text(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(ty) else {
        return;
    };
    if qname.to_string() != crate::CORE_HTML_MARKUP_CLASS {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_CORE_CLASS_NOT_STRINGABLE,
            format!(
                "`{}` is not text; it is the carrier the HTML sink writes raw",
                crate::CORE_HTML_MARKUP_CLASS
            ),
        )
        .with_primary(span, "used as a `string` here")
        .with_help(
            "escaping is not idempotent, so text made out of a `Markup` is escaped a second \
             time at the sink — write the markup to the sink directly, compose it with `+`, or \
             say `Core\\Html::toSource($markup, \"why\")` if the bytes really are what you want",
        ),
    );
}

/// `rule:classes/stringable`'s half of the row above: an object is stringifiable exactly
/// where it provably implements the reserved global `Stringable`, with no
/// property-walk fallback and no `__toString`. A `Core`-owned class is asked
/// the same question of `nvs_stdlib::registry` instead, for the reason
/// [`core_class_renders`] gives.
///
/// A class that passes either question then records the *same* resolved
/// `toString` target, because a `Core` member resolves out of the seeded
/// signature table exactly as a declared one does — `nvs-ir` asks
/// `core_symbol_of` which of the two calls to emit, and that is the only place
/// the difference is visible. The one rendering class that records nothing is
/// `rule:security/capture-answers-the-carrier`'s sink carrier: it has no `toString` member to resolve, so
/// `resolve_method` answers `None` and the value renders through
/// `nvs_runtime::stringify` on its runtime class instead.
///
/// Called on its own by the `as string` conversion, which is why it is a
/// function rather than a branch of [`require_stringable`].
fn require_stringable_object(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(ty).clone() else {
        return;
    };
    if qname.is_core() {
        if !core_class_renders(&qname) {
            env.diags.report(
                Diagnostic::error(
                    code::E_CORE_CLASS_NOT_STRINGABLE,
                    format!("`{qname}` cannot be converted to `string`; it has no `toString`"),
                )
                .with_primary(span, "converted to `string` here")
                .with_help(
                    "a `Core` class renders exactly where the spec gives it a `toString`, and \
                     this one has none — call the member that answers the text you want",
                ),
            );
            return;
        }
    } else {
        let stringable = QName::parse("Stringable");
        if !nvs_hir::implements_interface(&qname, &stringable, env.graph) {
            env.diags.report(
                Diagnostic::error(
                    code::E_STRINGABLE_REQUIRED,
                    format!(
                        "`{qname}` cannot be converted to `string` here; it does not implement \
                         `Stringable`"
                    ),
                )
                .with_primary(span, "converted to `string` here")
                .with_help("implement `Stringable`'s `toString(): string` on the class"),
            );
            return;
        }
    }
    // `rule:classes/stringable`: the conversion *is* a `toString()` call, so `nvs-ir` needs
    // its resolved target the same way an ordinary `$obj->toString()` does —
    // and, exactly like `object_comparison_result`'s `compareTo`, there is no
    // call node in the AST to key it by. The operand's own span is the key,
    // and it goes in its own map rather than an `ExprInfo`: the operand is an
    // ordinary expression that has usually already recorded an entry there
    // (`echo $b->build()` records the call `it` is), and one span holding two
    // independent facts is what `ExprTypeTable`'s side maps exist for.
    if let Some((owner, sig)) = resolve_method(&qname, "toString", env.signatures, env.graph) {
        // `toString()` takes no arguments, so there is nothing to map.
        let call = resolved_call(
            owner,
            "toString".to_owned(),
            &sig,
            Vec::new(),
            env.signatures,
        );
        env.exprs.record_to_string(span, call);
    }
}

/// Whether a `Core`-owned class renders as text.
///
/// One line, because the answer is not this crate's to hold:
/// [`nvs_stdlib::registry::class_renders`] is its one home, and that function's
/// doc comment owns the two rules behind it. All this adds is the lookup key —
/// a `Core` class is keyed on the name as written, which is what
/// [`QName`]'s own rendering already is.
fn core_class_renders(qname: &QName) -> bool {
    nvs_stdlib::registry::class_renders(&qname.to_string())
}
