//! How a literal takes its type from the position it appears in, and what its
//! own text has to survive first.
//!
//! Three rules meet here, one per literal shape. `rule:types/arithmetic` bounds an
//! integer literal's magnitude: the bare digit run either fits `int`'s
//! `0..=i64::MAX` half, `uint`'s full range where a `uint` is expected, or
//! neither ([`int_literal_digits`]). `rule:types/decimal`, `rule:types/numeric-literal-placement`, `rule:types/arithmetic` and `rule:types/conversion` place a fractional
//! literal at `decimal` or `float` by the position rather than by its own
//! spelling ([`wants_decimal`], [`record_decimal_placement`]) and bound the
//! mantissa and scale it may carry. A string literal's escape grammar and a
//! heredoc's flexible indentation are cooked by [`crate::string_lit`] and
//! their complaints reported here, once per literal, so a malformed escape is
//! a diagnostic rather than a lowering-time surprise.
//!
//! `rule:types/single-value-types` add a fourth rule of the same shape, and
//! [`placed_literal`] is all of it: a `string` or `int` literal types as its
//! own singleton exactly where the position names that singleton, and as its
//! plain base everywhere else. That is what `rule:types/single-value-types`'s *Verification* M2 row
//! asks for — without it nothing a caller writes ever satisfies a single-value type
//! except through an `as` — and it is why § 4's free-widening rows in
//! [`super::assign`] are the only other half needed: every other position
//! already sees the base. [`super::members`] applies the same helper to an
//! enum case, which is § 3's atom rather than § 1's.
//!
//! [`check_array_literal`] is `rule:types/arrays`'s half of the same idea for the one
//! composite literal: an array literal checked against an `array<T>` target
//! checks every element directly against `T`, never inferring an element type
//! and comparing it afterwards. [`synthesize_array_literal`] is the one
//! exception, and only under `var`: with no target to check against, it types
//! a literal from its elements, `array<T>` when they all have the one type `T`
//! and a reason otherwise (`rule:ide/no-compile-path-calls-the-synthesis`).
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `rule:types/single-value-types`'s producer half: the single-value atom `expected` names that this
/// literal *is*, or `None` where the position names none — in which case the
/// caller returns the base type, exactly as it did before this ADR.
///
/// `expected` is searched one level deep, the atom itself or the members of a
/// union, which is the whole of it: `rule:types/grammar` canonicalizes a union flat,
/// so there is no deeper level for a single-value atom to hide in.
///
/// `is_this_literal` is what makes this shared by three atom kinds — the two
/// [`Ty::SingleValueString`]/[`Ty::SingleValueInt`] callers below and
/// [`super::members`]'s [`Ty::EnumCase`] one — since the only thing that
/// differs between them is how a candidate atom is compared to the value in
/// hand.
pub(crate) fn placed_literal(
    expected: Option<TypeId>,
    interner: &TypeInterner,
    is_this_literal: impl Fn(&Ty) -> bool,
) -> Option<TypeId> {
    let expected = expected?;
    match interner.get(expected) {
        Ty::Union(members) => members
            .iter()
            .copied()
            .find(|id| is_this_literal(interner.get(*id))),
        ty => is_this_literal(ty).then_some(expected),
    }
}

/// [`placed_literal`] for `rule:types/single-value-types`'s string atom.
///
/// Two passes on purpose: the first asks the cheap question — does this
/// position name a single-value string type at all — and only then is the literal
/// cooked. Cooking allocates a `String`, and the answer is `None` for very
/// nearly every string literal in a program, so doing it unconditionally
/// would spend an allocation per literal expression to learn nothing. The
/// cooked value is what [`Ty::SingleValueString`] holds (see its own docs), so
/// `"a\n"` and a literal `"a"` followed by a real newline place identically.
fn placed_string_literal(span: Span, expected: Option<TypeId>, env: &Env<'_>) -> Option<TypeId> {
    placed_literal(expected, env.interner, |ty| {
        matches!(ty, Ty::SingleValueString(_))
    })?;
    let value = crate::string_lit::cook_string_literal(env.src, span);
    placed_literal(
        expected,
        env.interner,
        |ty| matches!(ty, Ty::SingleValueString(v) if *v == value),
    )
}

/// [`placed_literal`] for `rule:types/grammar`'s two `bool` singletons, which
/// [`Ty::True`] records are `rule:types/single-value-types`'s rule read on `bool`'s two values.
///
/// One pass, unlike [`placed_string_literal`]: the value is the token itself,
/// already decoded by the parser, so there is nothing to cook and no reason to
/// ask the cheap question first.
fn placed_bool_literal(value: bool, expected: Option<TypeId>, env: &Env<'_>) -> Option<TypeId> {
    placed_literal(expected, env.interner, |ty| {
        matches!((ty, value), (Ty::True, true) | (Ty::False, false))
    })
}

/// `true`/`false` — [`super::infer`]'s `ExprKind::Bool` arm.
///
/// A bare `bool` everywhere but a position that names this exact value, which
/// is `rule:types/single-value-types`'s placement rule and the reason `var $b = true;` still
/// infers `bool` rather than a type only `true` could ever satisfy.
pub(crate) fn infer_bool_literal(
    value: bool,
    expected: Option<TypeId>,
    env: &mut Env<'_>,
) -> TypeId {
    placed_bool_literal(value, expected, env).unwrap_or_else(|| env.interner.bool_ty())
}

/// Whether this expression is one whose type a position can still change —
/// the question [`super::args::check_generic_args`] asks about an argument it
/// had no parameter type for yet, and which that function's docs answer for.
///
/// The literals of this module's four rules, the negation `rule:types/single-value-types` makes
/// one atom with its operand ([`negated_literal_expectation`]), and `rule:types/arrays`'s array literal, whose elements are checked against the target's `T`
/// rather than inferred and compared afterwards ([`check_array_literal`]) —
/// so `[7, 7]` at a generic `array<uint>` is as unplaced as the `7` in it,
/// and leaving it out would have left D33 intact one level down. Checking one
/// a second time re-walks its elements, which is the price: bounded by the
/// literal's own size, paid only at a generic call, and only where the first
/// walk reported nothing.
///
/// An anonymous object is *not* here, and cannot be: [`check_anon_object`]
/// takes no expected type at all, so a second check would produce the same
/// shape it produced the first time. Every other expression already has the
/// type it will keep, so asking is free and answering `false` is exactly
/// right.
pub(crate) fn is_unplaced_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::ArrayLiteral(_) => true,
        ExprKind::Unary {
            op: UnaryOp::Neg,
            expr: operand,
        } => matches!(operand.kind, ExprKind::Int(_) | ExprKind::Float(_)),
        _ => false,
    }
}

/// The one thing a position with no type yet can still say to a literal:
/// `uint` for a digit run no other type can hold, and `None` for every other
/// literal, whose base type is already the honest answer with nothing to be
/// placed against.
///
/// Worth saying because the alternative is a *diagnostic* rather than a type.
/// [`infer_int_literal`] reports a digit run above `i64::MAX` as legal only
/// where a `uint` is expected, and a generic parameter that binds to `uint`
/// is such a place — but the report is spent by the time the bindings say so,
/// and [`super::args::check_generic_args`] places a literal a second time
/// only where the first pass left its text unreported. Handing the
/// expectation over is therefore what lets
/// `Core\Test::assertSame($big, 18446744073709551615)` reach that second
/// check clean; a run too large for 64 bits gets `None` here and keeps its
/// own one report.
pub(crate) fn unplaced_expectation(expr: &Expr, env: &mut Env<'_>) -> Option<TypeId> {
    let ExprKind::Int(span) = expr.kind else {
        return None;
    };
    let (radix, digits) = int_literal_digits(env.src, span);
    let magnitude = u64::from_str_radix(&digits, radix).ok()?;
    // `Err` is the answer that continues: a run `int` can hold has nothing
    // this function needs to say about it.
    i64::try_from(magnitude).err()?;
    Some(env.interner.uint())
}

/// The singleton type this expression names *on its own* — the placement rule
/// read backwards, for the one caller that needs the value the author wrote
/// rather than the type the position gave it.
///
/// [`super::operators::infer_conversion`]'s § 6 refusal is that caller: by the
/// time it runs, a literal the target does not accept has already widened back
/// to its base, taking the only record of which value it was with it. `None`
/// for every expression that is not one of § 1's two literals or `rule:types/grammar`'s two `bool` ones, including a malformed one — a literal that does not
/// survive its own text has no singleton to be, exactly as
/// [`infer_str_literal`] and [`infer_int_literal`] already decide.
pub(crate) fn literal_self_type(expr: &Expr, env: &mut Env<'_>) -> Option<TypeId> {
    match expr.kind {
        // No text to decode and no range to fail: the token *is* the value, so
        // this arm cannot answer `None` the way the two below can.
        ExprKind::Bool(true) => Some(env.interner.true_ty()),
        ExprKind::Bool(false) => Some(env.interner.false_ty()),
        ExprKind::Str(span) => {
            let value = crate::string_lit::cook_string_literal(env.src, span);
            Some(env.interner.single_value_string(value))
        }
        ExprKind::Int(span) => {
            let (radix, digits) = int_literal_digits(env.src, span);
            let magnitude = u64::from_str_radix(&digits, radix).ok()?;
            Some(
                env.interner
                    .single_value_int(i64::try_from(magnitude).ok()?),
            )
        }
        _ => None,
    }
}

/// The expectation a `-e` operand inherits, and `None` for every other unary
/// operator — which is what [`super::infer`]'s arm passed before `rule:types/single-value-types`.
///
/// `rule:types/single-value-types`'s int single-value atom carries its own sign, so `-1` is *one*
/// atom in type position; a `-1` expression is a negation wrapping the bare
/// digit run `1`. Placing the operand against the negated value is what lets
/// the two meet, and [`negated_literal_result`] puts the sign back on. Without
/// the pair, `-1` would be a type nothing but an `as` could ever produce,
/// while `1` was satisfied by writing it.
///
/// A `decimal` target passes straight through for the same reason one step
/// further out. `rule:types/numeric-literal-placement` places the digit run,
/// and the sign is a node above it, so without this arm `-19.99` at a
/// `decimal` reached [`check_float_literal`] with no expectation at all and
/// came out `float` — leaving a negative `decimal` constant writable only as
/// `-19.99 as decimal` while the positive one needed nothing. `uint` and the
/// single-value types are deliberately not here: a negated value that the target
/// cannot hold is the refusal those targets exist for.
///
/// A union naming `decimal` passes through too, but only to a numeric literal
/// `operand`, which then asks [`wants_decimal_in_union`] itself. That keeps
/// `-e` placed exactly where `e` is: `-3` at `?decimal` is a `decimal`, and a
/// union carrying both a `decimal` and a `float` arm keeps the `float` one for
/// a fractional literal, so `Core\Math::abs(-0.0)` is a `float` as
/// `abs(0.0)` is.
pub(crate) fn negated_literal_expectation(
    op: UnaryOp,
    operand: &Expr,
    expected: Option<TypeId>,
    interner: &mut TypeInterner,
) -> Option<TypeId> {
    if op != UnaryOp::Neg {
        return None;
    }
    if expected.is_some_and(|id| matches!(interner.get(id), Ty::Decimal)) {
        return expected;
    }
    if let Some(placed) = placed_literal(
        expected,
        interner,
        |ty| matches!(ty, Ty::SingleValueInt(v) if *v < 0),
    ) {
        let &Ty::SingleValueInt(value) = interner.get(placed) else {
            return None;
        };
        return Some(interner.single_value_int(value.checked_neg()?));
    }
    let numeric_literal = matches!(
        operand.unparenthesized().kind,
        ExprKind::Int(_) | ExprKind::Float(_)
    );
    // A plain `decimal` returned above, so a match here is a union member.
    let union_names_decimal =
        placed_literal(expected, interner, |ty| matches!(ty, Ty::Decimal)).is_some();
    if numeric_literal && union_names_decimal {
        return expected;
    }
    None
}

/// The type `-e` has when its operand took `rule:types/single-value-types`'s int single-value type —
/// the literal of the negated value, so `-1` placed at the type `-1` stays
/// that type rather than widening to `int` at the operator. Any other operand
/// type, and any other operator, is returned unchanged.
pub(crate) fn negated_literal_result(
    op: UnaryOp,
    inner: TypeId,
    interner: &mut TypeInterner,
) -> TypeId {
    if op != UnaryOp::Neg {
        return inner;
    }
    let &Ty::SingleValueInt(value) = interner.get(inner) else {
        return inner;
    };
    value
        .checked_neg()
        .map_or(inner, |negated| interner.single_value_int(negated))
}

/// `rule:types/conversion`'s "a numeric literal is untyped until placed" applied to the
/// one placement a binary operator offers: its *other* operand.
///
/// `uint` is the only type this changes anything for, and it changes
/// everything about writing one. An unplaced digit run defaults to `int` two
/// functions below, so before this existed every literal beside a `uint` was
/// § 4's mixed-signedness refusal — `$u + 1`, `$u & 3` and `$u << 1` were all
/// `E0407`, and a `uint` operand could meet only a `uint`-declared local. A
/// compound assignment never had the problem, because
/// [`super::assign::check_compound_assign`] already hands its value the
/// target's type as a hint; this is the same hint for the spelling that hint
/// was missing.
///
/// Value-preserving on every row of § 4's table, and that is why it is safe to
/// apply to the whole table rather than to the arithmetic rows alone: a bare
/// digit run is never negative — a leading `-` is a wrapping
/// [`ExprKind::Unary`], deliberately not placed here — so the same value is
/// being named, at the width the neighbour already has. What it also buys is
/// the literal above `i64::MAX`, which [`infer_int_literal`] admits exactly
/// where a `uint` is expected and which therefore had no operand position at
/// all until now.
///
/// Returns `None` for every other operand shape and every other neighbouring
/// type, so that operand is checked with no expectation exactly as before.
pub(crate) fn uint_operand_expectation(
    operand: &Expr,
    other: TypeId,
    interner: &mut TypeInterner,
) -> Option<TypeId> {
    if !matches!(operand.kind, ExprKind::Int(_)) {
        return None;
    }
    if !matches!(interner.get(other), Ty::Uint) {
        return None;
    }
    Some(interner.uint())
}

/// The expectation `$a ?? $b`'s right operand and `$a ?: $b`'s else arm,
/// `operand`, are inferred under: the whole expression's own `expected` where
/// the position names one, and otherwise `left`, the left operand's type
/// without `null`, when `operand` is an array literal.
///
/// Both operands are a value of the whole expression, and the second is the
/// default written for the first. An array literal has no type of its own
/// to offer, so with no position to place it at it is `array<mixed>`, and
/// `$values ?? []` would join into an `array<float>|array<mixed>` that
/// `foreach` cannot drive. Placed at the left operand's type it is an
/// `array<float>`, and its elements are checked against `float`, so
/// `$names ?? [1]` over `?array<string>` is a mismatch at the `1`.
///
/// Any other operand keeps the type it has: `$count ?? "none"` is
/// `int|string`. That is why the hint is limited to an array literal: a
/// `match` and a `catch` check their arms against the expectation they are
/// given, so a hint taken from the left would make them report a default the
/// program means to be different.
pub(crate) fn value_arm_expectation(
    expected: Option<TypeId>,
    operand: &Expr,
    left: TypeId,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    if expected.is_some() {
        return expected;
    }
    matches!(operand.unparenthesized().kind, ExprKind::ArrayLiteral(_))
        .then(|| env.interner.without_null(left))
}

/// Whether a position typed `id` places an integer literal at `uint`.
///
/// `rule:types/numeric-literal-placement`: `expr as T` is a placing position, and an enum's case
/// values are written at the backing type the declaration names, so a
/// `uint`-backed enum takes the digit run above `int`'s half exactly where
/// `as uint` does. Without that row the widest case of such an enum is
/// unwritable under a conversion, which is the loss that rule's placement
/// exists to prevent.
///
/// `?E` is the union `E|null`, and it descends into the one member beside
/// `null` because the null arm is the conversion's *outcome* rather than a
/// second target the literal could take. A union of two numeric types is not
/// descended into: two members would each claim the literal, and the position
/// names no single one of them.
fn wants_uint_placement(id: TypeId, interner: &TypeInterner) -> bool {
    match interner.get(id) {
        Ty::Uint => true,
        Ty::Enum(_, backing) | Ty::EnumCase(_, backing, _) => {
            matches!(backing, crate::enums::EnumBacking::Uint)
        }
        Ty::Union(members) if members.iter().any(|m| matches!(interner.get(*m), Ty::Null)) => {
            let mut beside_null = members
                .iter()
                .filter(|m| !matches!(interner.get(**m), Ty::Null));
            match (beside_null.next(), beside_null.next()) {
                (Some(only), None) => wants_uint_placement(*only, interner),
                _ => false,
            }
        }
        _ => false,
    }
}

/// `123` — [`super::infer`]'s `ExprKind::Int` arm.
///
/// `rule:types/arithmetic`: "An integer literal that does not fit `int` is legal only
/// where a `uint` is expected, and is otherwise a diagnostic saying exactly
/// that." The literal's own digits are never negative — a leading `-` is a
/// separate, wrapping `ExprKind::Unary` node, which already produces an
/// ordinary `int`/`uint` type mismatch on its own when negated and assigned
/// into a `uint` target, with no magnitude check needed for that half. What
/// *does* need one: whether the bare digit run fits `int`'s `0..=i64::MAX`
/// half, `uint`'s full `0..=u64::MAX` range, or neither at all.
///
/// Placed at `decimal` instead, `rule:types/arithmetic` and `rule:types/conversion` apply: an `int`/`uint` is
/// exact in a 96-bit mantissa, so the literal needs only that wider bound
/// checked rather than `int`'s 64-bit one.
pub(crate) fn infer_int_literal(
    span: Span,
    report_span: Span,
    expected: Option<TypeId>,
    env: &mut Env<'_>,
) -> TypeId {
    if wants_decimal(expected, env) {
        check_decimal_int_literal(span, report_span, env);
        return record_decimal_placement(report_span, env);
    }
    let wants_uint = expected.is_some_and(|id| wants_uint_placement(id, env.interner));
    let (radix, digits) = int_literal_digits(env.src, span);
    let parsed = u64::from_str_radix(&digits, radix);
    // `rule:types/single-value-types`, ahead of `uint`'s placement below because no position
    // names both: a single-value type is a singleton, and `uint` is not one. The
    // digit run is never negative here — a leading `-` is the wrapping
    // `ExprKind::Unary` [`negated_literal_expectation`] handles.
    if let Ok(&magnitude) = parsed.as_ref()
        && let Ok(value) = i64::try_from(magnitude)
        && let Some(placed) = placed_literal(
            expected,
            env.interner,
            |ty| matches!(ty, Ty::SingleValueInt(v) if *v == value),
        )
    {
        return placed;
    }
    let int = env.interner.int();
    if !wants_uint && wants_decimal_in_union(expected, int, env) {
        check_decimal_int_literal(span, report_span, env);
        return record_decimal_placement(report_span, env);
    }
    match parsed {
        Ok(n) if i64::try_from(n).is_ok() => {
            if wants_uint {
                env.interner.uint()
            } else {
                env.interner.int()
            }
        }
        Ok(_) if wants_uint => env.interner.uint(),
        Ok(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_INT_LITERAL_OUT_OF_RANGE,
                    "this number is too large for `int`. It is allowed only where a `uint` is \
                     expected",
                )
                .with_primary(report_span, "does not fit `int`"),
            );
            env.interner.int()
        }
        Err(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_INT_LITERAL_OUT_OF_RANGE,
                    "this number is too large for both `int` and `uint`",
                )
                .with_primary(report_span, "too large for a 64-bit integer"),
            );
            if wants_uint {
                env.interner.uint()
            } else {
                env.interner.int()
            }
        }
    }
}

/// `19.99` — [`super::infer`]'s `ExprKind::Float` arm.
///
/// `rule:types/numeric-literal-placement`: a literal carrying a fractional part or an exponent is
/// untyped until placed, and takes `decimal` or `float` from the type of the
/// position it appears in. `float` is the answer everywhere else, including
/// `var $x = 19.99;`, which has no target at all.
pub(crate) fn infer_float_literal(
    span: Span,
    report_span: Span,
    expected: Option<TypeId>,
    env: &mut Env<'_>,
) -> TypeId {
    let float = env.interner.float();
    if wants_decimal(expected, env) || wants_decimal_in_union(expected, float, env) {
        check_decimal_float_literal(span, report_span, env);
        return record_decimal_placement(report_span, env);
    }
    float
}

/// `'a'` / `"a"` / a heredoc or nowdoc body — [`super::infer`]'s
/// `ExprKind::Str` arm.
///
/// A single-quoted literal's own two escapes (`\\`/`\'`) can never produce
/// invalid UTF-8, so it gets no cooking-diagnostic pass at all. A
/// double-quoted literal runs the richer escape grammar
/// [`check_double_quoted_text_issues`] cooks. A heredoc/nowdoc-sourced `Str`
/// (whose span opens with `<`, not a quote) runs [`crate::string_lit`]'s
/// flexible-indentation check first, then the same escape grammar too — unless
/// it is a nowdoc, which (like PHP's) applies no escapes at all.
pub(crate) fn infer_str_literal(span: Span, expected: Option<TypeId>, env: &mut Env<'_>) -> TypeId {
    let raw = span_text(env.src, span);
    if raw.starts_with('"') {
        check_double_quoted_text_issues(inner_quoted_span(span), env);
    } else if raw.starts_with("<<<") {
        let (shape, indent_issues) = crate::string_lit::heredoc_shape(env.src, span);
        report_heredoc_indent_issues(indent_issues, env);
        let run_escapes = !crate::string_lit::heredoc_is_nowdoc(raw);
        check_heredoc_run_issues(&shape.indent, shape.body, true, true, run_escapes, env);
    }
    // `rule:types/single-value-types`, after the escape grammar has had its say: a literal that
    // does not survive its own text has no singleton to be, and reporting the
    // malformed escape once is what this arm is for.
    placed_string_literal(span, expected, env).unwrap_or_else(|| env.interner.string())
}

/// `"a $b c"` — [`super::infer`]'s `ExprKind::Interpolated` arm.
///
/// Only a heredoc/nowdoc can ever reach this with the opening `<<<`-only span
/// it needs its own flexible-indentation strip
/// (`nvs_syntax::parser::collapse_string_parts` never produces a nowdoc
/// `Interpolated` at all: a nowdoc has no interpolation syntax by
/// construction, so it always collapses to `ExprKind::Str`, which
/// [`infer_str_literal`] handles). Each interpolated expression must be
/// `Stringable` (`rule:classes/stringable`) and poisons the result on the `tainted` and
/// `secret` axes independently — see [`super::quals`].
pub(crate) fn infer_interpolated(
    expr: &Expr,
    parts: &[StringPart],
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let raw = span_text(env.src, expr.span);
    let is_heredoc = raw.starts_with("<<<");
    let indent = if is_heredoc {
        let (shape, indent_issues) = crate::string_lit::heredoc_shape(env.src, expr.span);
        report_heredoc_indent_issues(indent_issues, env);
        shape.indent
    } else {
        String::new()
    };
    let last_text_idx = is_heredoc
        .then(|| parts.iter().rposition(|p| matches!(p, StringPart::Text(_))))
        .flatten();
    let mut tainted = false;
    let mut secret = false;
    for (i, part) in parts.iter().enumerate() {
        match part {
            StringPart::Expr(e) => {
                let ty = check_expr(e, None, live, scope, ctx, env);
                require_stringable(ty, e.span, env);
                // `rule:core-classes/html-escape-answers-markup`: an interpolated piece is `.` written the other
                // way, and a carrier flattened into the literal is escaped
                // again by the sink that receives it.
                crate::expr::operators::reject_carrier_as_text(ty, e.span, env);
                // `rule:security/taint-propagation`: text out of `mixed` is
                // tainted, and a `?tainted string` or `?secret string` piece
                // poisons as `.` does.
                tainted |= carries_tainted(ty, env.interner) || carries_unchecked(ty, env.interner);
                secret |= carries_secret(ty, env.interner);
            }
            // A `Text` run's escapes follow exactly the same grammar regardless
            // of whether the overall literal is double-quoted or an
            // interpolated heredoc — see `crate::string_lit`'s own module docs
            // for why one routine cooks both. The span never includes a quote
            // character (`nvs_syntax::parser::parse_string_body` never emits one
            // as part of a `Text` token), so no quote-kind check is needed the
            // way [`infer_str_literal`] needs one — except a heredoc's own
            // flexible indentation, which has to be stripped from each run
            // first (this literal is never a nowdoc, so escapes always run).
            StringPart::Text(span) => {
                if is_heredoc {
                    check_heredoc_run_issues(
                        &indent,
                        *span,
                        i == 0,
                        Some(i) == last_text_idx,
                        true,
                        env,
                    );
                } else {
                    check_double_quoted_text_issues(*span, env);
                }
            }
        }
    }
    qualified_scalar(false, tainted, secret, env.interner)
}

/// ``html`<span>{$name}</span>` `` — [`super::infer`]'s `ExprKind::HtmlTemplate` arm.
///
/// The type is `Core\Html\Markup` whatever the body holds, because the node is
/// what says so (`rule:core-classes/html-template`): a hole-free literal is
/// still a carrier, not the `string` a quoted literal of the same text would
/// be.
///
/// A hole is escaped through `Core\Html::escape` and spliced, so the three
/// questions asked of it are not an interpolation's three. A hole already
/// holding a `Markup` is spliced raw — that is `Markup + Markup` written in
/// interpolation syntax, so [`super::operators::reject_carrier_as_text`]'s
/// refusal deliberately does *not* run here. A `tainted` hole is admitted and
/// does not spread, because escaping neutralises injection structurally and
/// the sink never distinguishes the two
/// (`rule:core-classes/html-auto-escape`). A `secret` hole is refused where it
/// is written, because escaping does nothing for confidentiality
/// (`rule:security/secret-sinks-refuse`).
pub(crate) fn infer_html_template(
    parts: &[StringPart],
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    for part in parts {
        match part {
            StringPart::Expr(e) => {
                let ty = check_expr(e, None, live, scope, ctx, env);
                reject_secret_output(ty, e.span, "an `html` template", env);
                if !is_html_markup(ty, env) {
                    require_stringable(ty, e.span, env);
                }
                // Which of the two a hole is cannot be re-derived a phase down:
                // a carrier and a `Stringable` object both erase to
                // `nvs_ir::ty::Ty::Object`, and the answer decides between
                // splicing the hole raw and escaping it. So it is recorded at
                // the hole's own span, the arrangement
                // [`crate::expr_table::ExprTypeTable::declared_ty`] already
                // serves for an enum-named type atom and a `decimal`
                // placement.
                env.exprs.record_type(e.span, ty);
            }
            // A segment's escapes are the double-quoted grammar's, which is
            // what the lexer ran over it; the two the literal adds, `` \` ``
            // and `\{`, cook to themselves and so need no row of their own.
            StringPart::Text(span) => check_double_quoted_text_issues(*span, env),
        }
    }
    env.interner
        .class(QName::parse(crate::CORE_HTML_MARKUP_CLASS))
}

/// Whether `ty` is the HTML carrier itself, which a markup hole splices raw.
fn is_html_markup(ty: TypeId, env: &Env<'_>) -> bool {
    matches!(env.interner.get(ty), Ty::Class(qname, _)
        if qname.to_string() == crate::CORE_HTML_MARKUP_CLASS)
}

/// `rule:types/decimal`'s mantissa bound: 96 bits, unsigned, with the sign carried
/// beside it rather than in it.
const MAX_DECIMAL_MANTISSA: u128 = (1u128 << 96) - 1;

/// `rule:types/decimal`'s scale bound: the number of digits after the point.
const MAX_DECIMAL_SCALE: i32 = 28;

/// Records that the numeric literal at `span` was placed at `decimal`, and
/// answers that type.
///
/// The recording is what lets `nvs_ir::lower` fold the literal from its own
/// **digits** rather than through an `f64`. It needs it because `rule:types/numeric-literal-placement`'s
/// placing target is not always visible there: a declared type reaches
/// lowering as `nvs_ir::ty::Ty`, which erases an array's element type, so
/// `array<decimal> $prices = [19.99];` would otherwise put a `float` in the
/// array the checker just typed `decimal`. Reading the answer back is the same
/// arrangement [`crate::expr_table::ExprTypeTable::declared_ty`] already
/// serves for an enum-named type atom.
pub(crate) fn record_decimal_placement(span: Span, env: &mut Env<'_>) -> TypeId {
    let decimal = env.interner.decimal();
    env.exprs.record_type(span, decimal);
    decimal
}

/// Whether the position a literal is being placed in wants a `decimal` —
/// `rule:types/numeric-literal-placement`'s "untyped until placed" rule, asked once per literal arm.
pub(crate) fn wants_decimal(expected: Option<TypeId>, env: &Env<'_>) -> bool {
    expected.is_some_and(|id| matches!(env.interner.get(id), Ty::Decimal))
}

/// Whether a union position places a numeric literal at its `decimal` member:
/// the union names `decimal`, and the literal's unplaced type `default` (`int`
/// for a digit run, `float` for a fractional literal) is not assignable to it.
///
/// That makes `?decimal` and `decimal|string` decimal positions for a literal,
/// exactly as a plain `decimal` is, while a union that already accepts the
/// literal keeps the type it takes today: `int|decimal` places `3` at `int`,
/// and `float|decimal` places `3` and `1.5` at `float`. A unioned single-value type
/// such as `1|decimal` is matched by [`placed_literal`] before this is asked.
pub(crate) fn wants_decimal_in_union(
    expected: Option<TypeId>,
    default: TypeId,
    env: &mut Env<'_>,
) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    let Ty::Union(members) = env.interner.get(expected) else {
        return false;
    };
    let names_decimal = members
        .iter()
        .any(|member| matches!(env.interner.get(*member), Ty::Decimal));
    names_decimal && !is_assignable(default, expected, env.interner, env.graph, env.signatures)
}

/// `rule:types/decimal`'s layout, applied to a fractional literal's own text: a 96-bit
/// mantissa and a scale of 0 to 28. Returns the reason it does not fit, or
/// `None` when it does.
///
/// An exponent is folded into the scale rather than rejected — `1.5e3` is
/// mantissa 1500 at scale 0, and `1.5e-30` is a scale-31 value this refuses.
/// Trailing zeros are *kept*, because § 4 makes scale observable in rendering:
/// `19.90 as string` is `"19.90"`, so `19.90` is a scale-2 value and not a
/// second spelling of `19.9`.
pub(crate) fn decimal_literal_overflow(text: &str) -> Option<&'static str> {
    let cleaned: String = text.chars().filter(|&c| c != '_').collect();
    let (numeric, exponent) = match cleaned.split_once(['e', 'E']) {
        Some((numeric, exp)) => match exp.parse::<i32>() {
            Ok(exp) => (numeric, exp),
            // Only reachable from a hand-built AST: the lexer produces a
            // `FloatLiteral` only for an exponent that already parsed.
            Err(_) => return Some("exponent"),
        },
        None => (cleaned.as_str(), 0),
    };
    let (int_part, frac_part) = numeric.split_once('.').unwrap_or((numeric, ""));
    let mut digits = format!("{int_part}{frac_part}");
    let scale = i32::try_from(frac_part.len()).unwrap_or(i32::MAX) - exponent;
    let scale = if scale < 0 {
        // A positive exponent wider than the fractional part is an integer:
        // shift the point right by padding the mantissa instead.
        digits.push_str(&"0".repeat(scale.unsigned_abs() as usize));
        0
    } else {
        scale
    };
    if scale > MAX_DECIMAL_SCALE {
        return Some("scale");
    }
    match digits.trim_start_matches('0').parse::<u128>() {
        Ok(mantissa) if mantissa <= MAX_DECIMAL_MANTISSA => None,
        // An all-zero (or empty) digit run is the value zero, which fits.
        Err(_) if digits.trim_start_matches('0').is_empty() => None,
        _ => Some("mantissa"),
    }
}

/// A numeric literal's text as `rule:types/decimal`'s `(mantissa, scale)`, or
/// `None` where [`decimal_literal_overflow`] names a bound it exceeds.
///
/// Read from the *digits* rather than through an `f64`, so `19.99` placed at
/// `decimal` is exact to the full 29 significant digits. An exponent is folded
/// into the scale and trailing zeros are kept, for the reasons
/// [`decimal_literal_overflow`] gives. A plain decimal integer literal reads as
/// scale 0. Two readers: a folded `decimal` constant
/// ([`crate::defaults::ConstArg::Decimal`]) and `nvs_ir`'s placed literal, so
/// the two cannot disagree about one spelling.
pub fn decimal_literal_parts(text: &str) -> Option<(u128, u8)> {
    if decimal_literal_overflow(text).is_some() {
        return None;
    }
    let cleaned: String = text.chars().filter(|&c| c != '_').collect();
    let (numeric, exponent) = match cleaned.split_once(['e', 'E']) {
        Some((numeric, exponent)) => (numeric, exponent.parse::<i32>().ok()?),
        None => (cleaned.as_str(), 0),
    };
    let (whole, fraction) = numeric.split_once('.').unwrap_or((numeric, ""));
    let mut digits = format!("{whole}{fraction}");
    let scale = i32::try_from(fraction.len()).ok()? - exponent;
    if scale < 0 {
        digits.push_str(&"0".repeat(usize::try_from(scale.unsigned_abs()).ok()?));
    }
    let mantissa = digits.parse::<u128>().ok()?;
    Some((mantissa, u8::try_from(scale.max(0)).ok()?))
}

/// Reports `rule:types/decimal`'s bound for a fractional literal placed at `decimal`.
pub(crate) fn check_decimal_float_literal(span: Span, report_span: Span, env: &mut Env<'_>) {
    let text = span_text(env.src, span).to_owned();
    if let Some(reason) = decimal_literal_overflow(&text) {
        report_decimal_out_of_range(reason, report_span, env);
    }
}

/// The integer-literal half of [`check_decimal_float_literal`]: only the
/// mantissa can overflow, since an integer literal is scale 0 by construction.
pub(crate) fn check_decimal_int_literal(span: Span, report_span: Span, env: &mut Env<'_>) {
    let (radix, digits) = int_literal_digits(env.src, span);
    if !u128::from_str_radix(&digits, radix).is_ok_and(|m| m <= MAX_DECIMAL_MANTISSA) {
        report_decimal_out_of_range("mantissa", report_span, env);
    }
}

pub(crate) fn report_decimal_out_of_range(reason: &str, span: Span, env: &mut Env<'_>) {
    let detail = match reason {
        "scale" => "more than 28 digits after the point",
        _ => "a mantissa wider than 96 bits",
    };
    env.diags.report(
        Diagnostic::error(
            code::E_DECIMAL_LITERAL_OUT_OF_RANGE,
            format!("this number does not fit `decimal`: it has {detail}"),
        )
        .with_primary(span, "outside `decimal`'s range")
        .with_help(
            "`decimal` holds a 96-bit mantissa at a scale of 0 to 28 (`rule:types/decimal`); \
             `Core\\BigInt` is the type for a whole number beyond it",
        ),
    );
}

/// Splits an integer-literal span's cooked text into the radix its prefix
/// names and the digit run to parse against it — the same job
/// `nvs_ir::lower::int_literal_digits` does for lowering, duplicated here
/// rather than shared: this crate has no dependency on `nvs-ir` (the
/// dependency runs the other way), and the magnitude has to be known here,
/// at check time, so [`infer_int_literal`] can report `rule:types/arithmetic`'s
/// diagnostic itself rather than let an out-of-range literal surface only as
/// a lowering-time panic once `nvs-ir` tries to cook the same span. Strips
/// `_` digit separators the same way; a legacy leading-zero octal spelling
/// like PHP's `0755` is deliberately not one of the recognized prefixes (see
/// `nvs_ir`'s own copy of this function for why), so it falls through to the
/// decimal case, matching `nvs-syntax`'s lexer.
pub(crate) fn int_literal_digits(src: &SourceFile, span: Span) -> (u32, String) {
    let cleaned: String = span_text(src, span).chars().filter(|&c| c != '_').collect();
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            return (radix, rest.to_owned());
        }
    }
    (10, cleaned)
}

/// Narrows a double-quoted `ExprKind::Str`'s own span (quote characters
/// included) to the text strictly between them —
/// [`crate::string_lit::cook_double_quoted_text`]'s expected input shape,
/// the same one a `StringPart::Text` span already has natively. `"` is
/// one byte, so trimming exactly one byte off each end is exact, not an
/// approximation.
pub(crate) fn inner_quoted_span(span: Span) -> Span {
    Span::new(span.file, span.start + 1, span.end - 1)
}

/// Cooks `span` (already known to be double-quoted-grammar text — see the two
/// call sites in [`infer`]) purely to surface [`crate::string_lit::CookIssue`]s
/// as diagnostics; the cooked `String` itself is discarded here; `nvs-ir`
/// re-cooks it from the same span when it actually lowers the literal, per
/// `crate::string_lit`'s own module docs on why that duplicate call is safe
/// (one shared implementation) rather than a second, divergent one.
pub(crate) fn check_double_quoted_text_issues(span: Span, env: &mut Env<'_>) {
    let (_, issues) = crate::string_lit::cook_double_quoted_text(env.src, span);
    report_cook_issues(issues, env);
}

pub(crate) fn report_cook_issues(issues: Vec<crate::string_lit::CookIssue>, env: &mut Env<'_>) {
    for issue in issues {
        match issue {
            crate::string_lit::CookIssue::InvalidUnicodeEscape(span) => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_INVALID_UNICODE_ESCAPE,
                        "this `\\u{...}` escape does not name a valid Unicode code point",
                    )
                    .with_primary(span, "outside 0..=0x10FFFF, or a UTF-16 surrogate"),
                );
            }
            crate::string_lit::CookIssue::InvalidUtf8(span) => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_STRING_LITERAL_INVALID_UTF8,
                        "the `\\xHH` and octal escapes in this string do not form valid UTF-8. \
                         A `string` is always valid UTF-8",
                    )
                    .with_primary(span, "not valid UTF-8"),
                );
            }
        }
    }
}

pub(crate) fn report_heredoc_indent_issues(
    issues: Vec<crate::string_lit::HeredocIndentIssue>,
    env: &mut Env<'_>,
) {
    for issue in issues {
        match issue {
            crate::string_lit::HeredocIndentIssue::MixedIndentWhitespace(span) => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_HEREDOC_MIXED_INDENT,
                        "this heredoc/nowdoc's closing marker mixes spaces and tabs in its \
                         indentation",
                    )
                    .with_primary(span, "must be all spaces or all tabs, not both"),
                );
            }
            crate::string_lit::HeredocIndentIssue::InsufficientIndent(span) => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_HEREDOC_INSUFFICIENT_INDENT,
                        "this line has less leading whitespace than the heredoc/nowdoc's \
                         closing marker",
                    )
                    .with_primary(
                        span,
                        "does not start with the closing marker's own indentation",
                    ),
                );
            }
        }
    }
}

/// Cooks one heredoc/nowdoc body run — [`crate::string_lit::HeredocShape::body`]'s whole span for
/// a `Str`-collapsed literal, or one `StringPart::Text` span inside an `Interpolated` one —
/// reporting both indentation and (for a heredoc, never a nowdoc) escape-cooking issues found
/// along the way. The cooked `String` itself is discarded, same as [`check_double_quoted_text_issues`]:
/// `nvs-ir` re-cooks it from the same inputs when it actually lowers the literal.
pub(crate) fn check_heredoc_run_issues(
    indent: &str,
    span: Span,
    body_start: bool,
    is_last_run: bool,
    run_escapes: bool,
    env: &mut Env<'_>,
) {
    let mut issues = Vec::new();
    let dedented = crate::string_lit::dedent_heredoc_run(
        env.src,
        indent,
        span,
        body_start,
        is_last_run,
        &mut issues,
    );
    report_heredoc_indent_issues(issues, env);
    if run_escapes {
        let (_, cook_issues) = crate::string_lit::cook_double_quoted_text_str(&dedented, span);
        report_cook_issues(cook_issues, env);
    }
}

/// `{a: 1, b: $x}` — `rule:types/anonymous-object`'s anonymous object, whose type is the
/// exact-fields shape of its own fields.
///
/// Where the position declares a shape (`rule:types/shape-type`), a field the
/// declaration names is checked against the declared field type and, when its
/// value fits, has that type: `{w: 2}` at `{w: float}` is a `{w: float}`. The
/// literal builds a new value, so lowering can convert each field where it is
/// stored, which an existing value's field cannot be (see
/// [`super::assign::field_fits`]). Every other field infers its own type, and
/// a shape target's width subtyping ([`super::assign::is_assignable`]) lets
/// the literal flow into a narrower shape or a plain `object`.
///
/// **A field name written twice is `E0494`.** A shape's fields are a set —
/// `{a: int}` names one slot `a` — and the two sides of a repeated name would
/// not even agree on which write survives: the interned shape reads the first
/// of the pair, while the class `nvs_ir::lower` synthesizes carries one slot
/// per name and would keep the last. PHP's nearest neighbour is a duplicate
/// *array* key, where the last write wins silently, and that reading is not
/// carried over: an array is a map and a shape is a record. The duplicate's
/// own initializer is still checked, so its errors are reported in the same
/// run, and only the repeat is dropped from the interned shape — which keeps
/// what flows onward a genuine set rather than a shape no reader agrees on.
pub(crate) fn check_anon_object(
    fields: &[AnonObjectField],
    expected: Option<TypeId>,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    // The declared shape, when the position names one: a `?{…}` position
    // declares the shape and `null` is the other thing it may hold, and a union
    // with exactly one shape member declares that member.
    let expected = expected.and_then(|id| declared_shape(id, env));
    let mut out: Vec<ShapeField> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let declared = declared_field_type(expected, &name, env);
        // A field the declaration names is checked with the declared type as
        // its expectation, so `2` in a `float` field is a `float` literal. When
        // the value fits the declared type, the field has the declared type:
        // `{w: $int}` at `{w: float}` is a `{w: float}`, and lowering converts
        // the value where the literal stores it, at the slot type
        // `nvs_ir::lower` reads back from the field name's span. A value that
        // does not fit keeps its own type, and the assignment that follows
        // reports it. So does a value with a `secret` or `tainted` qualifier the
        // declared type does not carry, so the qualifier stays visible.
        let inferred = infer(&field.value, declared, live, scope, ctx, env);
        let field_ty = match declared {
            Some(declared)
                if is_assignable(inferred, declared, env.interner, env.graph, env.signatures)
                    && !drops_qualifier(inferred, declared, env) =>
            {
                env.exprs.record_type(field.name, declared);
                note_float_widening(&field.value, inferred, declared, env);
                declared
            }
            _ => inferred,
        };
        // Unlike an array element, a field's inferred type *keeps* the
        // qualifier, so what this catches is the declared field one step on:
        // `{token: $secret}` at a `{token: mixed}` is where the bit stops
        // being visible. A literal with no shape declared for it is asked
        // nothing — it carries the qualifier onward in its own inferred type,
        // and a position that then widens it to `mixed` is the widening every
        // `mixed` binding in the language allows, not this axis.
        if let Some(declared) = declared {
            reject_secret_into_container(
                &field.value,
                inferred,
                Some(declared),
                &format!("the field `{name}`"),
                env,
            );
        }
        if out.iter().any(|seen| seen.name == name) {
            report_duplicate_shape_field(field, &name, env);
            continue;
        }
        // Every key a literal writes is one the value carries, so the inferred
        // shape names none of them optional — the `?` is a thing a *declared*
        // type says, never a thing a value's own type discovers.
        out.push(ShapeField::required(name, field_ty));
    }
    env.interner.shape(out)
}

/// The shape an anonymous object's position declares: `expected` itself when it
/// is a shape, the shape inside a `?{…}`, or the one shape member of a union.
/// `None` for anything else, including a union with two shape members, where
/// the literal keeps the types its own fields infer.
fn declared_shape(expected: TypeId, env: &mut Env<'_>) -> Option<TypeId> {
    let expected = env.interner.without_null(expected);
    match env.interner.get(expected) {
        Ty::Shape(_) => Some(expected),
        Ty::Union(members) => {
            let mut shapes = members
                .iter()
                .copied()
                .filter(|member| matches!(env.interner.get(*member), Ty::Shape(_)));
            match (shapes.next(), shapes.next()) {
                (Some(only), None) => Some(only),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether a field value of type `inferred` loses a `secret` or `tainted`
/// qualifier when its field takes the type `declared`.
fn drops_qualifier(inferred: TypeId, declared: TypeId, env: &Env<'_>) -> bool {
    let interner = &*env.interner;
    (contains_secret(inferred, interner) && !contains_secret(declared, interner))
        || (carries_tainted(inferred, interner) && !carries_tainted(declared, interner))
}

/// The type a *declared* shape gives the field named `name`, where the
/// position declared a shape at all.
///
/// [`check_anon_object`] checks the field's value against it, and
/// `rule:security/secret-qualifier`'s container question asks it: a field
/// the declared shape does not name has no declaration to carry a qualifier,
/// which reads the same as no expectation at all.
fn declared_field_type(expected: Option<TypeId>, name: &str, env: &Env<'_>) -> Option<TypeId> {
    let Ty::Shape(fields) = env.interner.get(expected?) else {
        return None;
    };
    fields.iter().find(|field| field.name == name).map(|f| f.ty)
}

/// The `E0494` half of [`check_anon_object`], which owns why.
fn report_duplicate_shape_field(field: &AnonObjectField, name: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_DUPLICATE_SHAPE_FIELD,
            format!("the field `{name}` is written twice in this anonymous object"),
        )
        .with_primary(field.name, "already given a value above")
        .with_help(
            "a shape's fields are a set, so there is one slot per name — drop one of the two, \
             or give the second field its own name",
        ),
    );
}

pub(crate) fn check_array_literal(
    items: &[ArrayItem],
    expected: Option<TypeId>,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    // A literal is an array, so of a union expectation it fills the one
    // `array<T>` member: `?array<string> $m = ["k" => "v"];` places an
    // `array<string>`, and `float|array<float> $p = [1, 2];` an
    // `array<float>`. The other members are what the binding may hold
    // instead. Without this the expectation is a `Ty::Union`, no element type
    // is found, every element is checked against nothing, and the literal
    // falls back to `array<mixed>` and fails at the binding. A union with two
    // array members names no single element type, so it keeps that fallback.
    let expected = expected.map(|id| sole_array_member(id, env));
    let elem_expected = expected.and_then(|id| match env.interner.get(id) {
        Ty::Array(elem) => Some(*elem),
        _ => None,
    });
    for item in items {
        if item.by_ref {
            report_by_reference_element(item, env);
        }
        if item.spread {
            check_spread_element(item, elem_expected, live, scope, ctx, env);
            continue;
        }
        if let Some(key) = &item.key {
            let key_ty = check_expr(key, None, live, scope, ctx, env);
            check_array_key_type(key_ty, key.span, env);
        }
        // `rule:security/secret-qualifier`'s container axis, at the element
        // that is about to lose the bit: this literal joins no element types,
        // so the placed `array<T>`'s own `T` is the whole of what carries one
        // onward. Guarded on the diagnostic count for
        // [`check_spread_element`]'s reason — an element that already reported
        // a mismatch against `T` is one mistake, not two.
        let before = env.diags.len();
        let elem_ty = check_expr(&item.value, elem_expected, live, scope, ctx, env);
        if env.diags.len() == before {
            reject_secret_into_container(
                &item.value,
                elem_ty,
                elem_expected,
                "an array element",
                env,
            );
        }
        // The literal builds a new array, so each element can be converted
        // where it is stored, as an anonymous object's field is
        // ([`check_anon_object`]). The element type is recorded at the
        // element's span, and `nvs_ir::lower` reads it back: it lowers a
        // number literal at that type and converts any other value to it, so
        // `1` and `$count` in an `array<float>` literal are stored as floats.
        // An element typed `decimal` keeps the span's own record: a number
        // literal placed at `decimal` inside a union such as `?decimal` is
        // recorded there by [`record_decimal_placement`], and the union would
        // replace it.
        if let Some(elem) = elem_expected
            && !matches!(env.interner.get(elem_ty), Ty::Decimal)
            && is_assignable(elem_ty, elem, env.interner, env.graph, env.signatures)
        {
            env.exprs.record_type(item.value.span, elem);
        }
    }
    match (expected, elem_expected) {
        (Some(id), Some(_)) => id,
        _ => {
            let mixed = env.interner.mixed();
            env.interner.array(mixed)
        }
    }
}

/// The one `array<T>` member of a union `expected`, or `expected` itself
/// where it is not a union or has no member or several — see
/// [`check_array_literal`].
fn sole_array_member(expected: TypeId, env: &Env<'_>) -> TypeId {
    let Ty::Union(members) = env.interner.get(expected) else {
        return expected;
    };
    let mut arrays = members
        .iter()
        .copied()
        .filter(|member| matches!(env.interner.get(*member), Ty::Array(_)));
    match (arrays.next(), arrays.next()) {
        (Some(only), None) => only,
        _ => expected,
    }
}

/// Why [`synthesize_array_literal`] gives a literal no type.
pub(crate) enum NoArrayType {
    /// The elements have two or more types. `union` is their canonical union,
    /// which no binding is ever given, and `distinct` the same types in the
    /// order the elements are written, which is how a help line prints them.
    Differ {
        union: TypeId,
        distinct: Vec<TypeId>,
    },
    /// The literal, or a literal nested in it, has no elements. `at` is the
    /// empty one.
    Empty { at: Span },
    /// An element reported its own error, so its type is a stand-in and a
    /// refusal would be a second diagnostic for one mistake.
    Untyped,
}

/// `rule:types/var-inference`'s one-type rule: the type a literal with no
/// target has, `array<T>` when every element has the same interned type `T`.
///
/// An element's type is what [`check_expr`] gives it with no expectation,
/// which is what `var` gives that element alone, so nothing is widened to a
/// common type: `[1, 2.5]` and a parent beside its child are two types, and so
/// is a `tainted` string beside a plain one. A nested literal is typed by the
/// same rule first, a spread gives its source's element type, and a key is
/// checked as [`check_array_literal`] checks it and takes no part in `T`.
///
/// On success every element is recorded at `T`, as [`check_array_literal`]
/// records it under a written `array<T>` target, so `nvs_ir::lower` lowers the
/// two spellings the same. Every element is checked exactly once, whatever the
/// answer, so an element's own error is reported as it would be anywhere else
/// and nothing an element registers is registered twice.
///
/// Only `crate::locals`'s `var` arm and its `foreach` subject call this, and
/// a guard test in `tests/locals.rs` counts the callers.
pub(crate) fn synthesize_array_literal(
    items: &[ArrayItem],
    span: Span,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Result<TypeId, NoArrayType> {
    synthesize_items(items, span, live, scope, ctx, env)
}

/// [`synthesize_array_literal`]'s body. A nested literal and a spread of one
/// recurse into this rather than into the public function, so that the public
/// one's callers are only the two the guard test counts.
fn synthesize_items(
    items: &[ArrayItem],
    span: Span,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Result<TypeId, NoArrayType> {
    let mut types: Vec<TypeId> = Vec::new();
    let mut empty = items.is_empty().then_some(span);
    let mut untyped = false;
    // A nested literal of two types makes this one two types as well, even
    // when it is the only element; its union is its part of the help line.
    let mut differs = false;
    for item in items {
        if item.by_ref {
            report_by_reference_element(item, env);
            untyped = true;
        }
        if let Some(key) = &item.key {
            let key_ty = check_expr(key, None, live, scope, ctx, env);
            check_array_key_type(key_ty, key.span, env);
        }
        let answer = if item.spread {
            spread_element_ty(item, live, scope, ctx, env)
        } else {
            element_ty(&item.value, live, scope, ctx, env)
        };
        match answer {
            Ok(ty) => types.push(ty),
            Err(NoArrayType::Differ { union, .. }) => {
                differs = true;
                types.push(if item.spread {
                    union
                } else {
                    env.interner.array(union)
                });
            }
            Err(NoArrayType::Empty { at }) => empty = empty.or(Some(at)),
            Err(NoArrayType::Untyped) => untyped = true,
        }
    }
    if untyped {
        return Err(NoArrayType::Untyped);
    }
    if let Some(at) = empty {
        return Err(NoArrayType::Empty { at });
    }
    let first = types[0];
    if differs || types.iter().any(|ty| *ty != first) {
        let mut distinct: Vec<TypeId> = Vec::new();
        for ty in &types {
            if !distinct.contains(ty) {
                distinct.push(*ty);
            }
        }
        let union = env.interner.make_union(types);
        return Err(NoArrayType::Differ { union, distinct });
    }
    if !matches!(env.interner.get(first), Ty::Decimal) {
        for item in items.iter().filter(|item| !item.spread) {
            env.exprs.record_type(item.value.span, first);
        }
    }
    Ok(env.interner.array(first))
}

/// One plain element's type for [`synthesize_items`]: a nested literal's by
/// the same rule, anything else what [`check_expr`] gives it alone.
fn element_ty(
    value: &Expr,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Result<TypeId, NoArrayType> {
    if let ExprKind::ArrayLiteral(items) = &value.kind {
        return synthesize_items(items, value.span, live, scope, ctx, env);
    }
    let errors_before = env.diags.error_count();
    let refused_before = env.refused_exprs;
    let ty = check_expr(value, None, live, scope, ctx, env);
    if env.diags.error_count() != errors_before || env.refused_exprs != refused_before {
        return Err(NoArrayType::Untyped);
    }
    Ok(ty)
}

/// A `...` element's part of `T` for [`synthesize_items`]: its source's
/// element type. A source that is not an array is the spread's own `E0484`.
fn spread_element_ty(
    item: &ArrayItem,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Result<TypeId, NoArrayType> {
    let subject = element_ty(&item.value, live, scope, ctx, env)?;
    match env.interner.get(subject) {
        Ty::Array(elem) => Ok(*elem),
        _ => {
            report_spread_not_an_array(item, subject, env);
            Err(NoArrayType::Untyped)
        }
    }
}

/// `[...$a]` — the subject's own elements have to satisfy the literal's.
///
/// A spread contributes the subject's *entries*, so what it owes is exactly
/// what a written-out element owes, one level up: where the literal knows its
/// element type the subject is checked against `array<that>`, which reports an
/// ordinary `E0401` naming both array types when it does not fit — array reads
/// being element-covariant ([`super::assign::is_assignable`]), spreading an
/// `array<Dog>` into an `array<Animal>` is accepted for the same reason
/// reading one is. This is why the check is an *expectation* rather than a
/// comparison afterwards: it is the same rule as the surrounding loop's, and
/// an inner literal spread into an outer one (`[...["a"]]`) gets the element
/// type placed into it exactly as a nested literal would.
///
/// The subject still has to be an array at all, and where the position named
/// no `array<T>` there is no expectation to catch that — a `mixed` binding or
/// parameter is the reachable one, an `array<mixed>` position still expecting
/// `array<mixed>`. `E0484` is that case and only that case, so `[...$s]` over
/// a `string` is one diagnostic however it is written, never `E0401` and
/// `E0484` together. The `env.diags.len()` guard is the
/// same "a subject that already reported its own error is one mistake, not
/// two" the `ExprKind::Index` arm uses.
///
/// **Nothing is recorded on the [`crate::expr_table`] side for a spread, and
/// that is a decision rather than an omission.** The lowering reads
/// `ArrayItem::spread` off the AST it already walks, and the one type it needs
/// — the subject's — is what its own `lower_expr` hands back beside the value;
/// an entry would be a second copy of two facts the lowering holds already.
/// That is the opposite of `ExprInfo::Index`, which exists because a guarded
/// read's element type is not recoverable below the checker at all.
fn check_spread_element(
    item: &ArrayItem,
    elem_expected: Option<TypeId>,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let expectation = elem_expected.map(|elem| env.interner.array(elem));
    let before = env.diags.len();
    let subject_ty = check_expr(&item.value, expectation, live, scope, ctx, env);
    let reported = env.diags.len() != before;
    if reported || matches!(env.interner.get(subject_ty), Ty::Array(_)) {
        return;
    }
    report_spread_not_an_array(item, subject_ty, env);
}

/// `E0484`, for [`check_spread_element`] and [`spread_element_ty`] alike.
fn report_spread_not_an_array(item: &ArrayItem, subject_ty: TypeId, env: &mut Env<'_>) {
    let rendered = env.interner.describe(subject_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_SPREAD_SUBJECT_NOT_AN_ARRAY,
            "a `...` array element needs an `array<T>` to spread",
        )
        .with_primary(item.value.span, format!("this is `{rendered}`"))
        .with_help(
            "`...` contributes the subject's own entries to the array being built, so the \
             subject has to have entries — write the value as an ordinary element instead",
        ),
    );
}

/// `&value` as an array-literal element is refused here rather than lowered.
///
/// PHP's `[&$x]` makes the element and `$x` the same storage, and Novis has no
/// rule that can own one: `rule:types/implicit-capture` removed by-reference capture, so no
/// binding aliases another, and `rule:classes/two-copy-depths` fixes an element as a copy taken
/// where the literal is evaluated. So this is not a gap in `nvs-ir` — the
/// panic it used to reach was reporting the absence of a feature the language
/// decided against — and the refusal names the decision rather than the
/// missing lowering. Checking continues past it: the element's value is an
/// ordinary expression and its own errors are worth reporting in the same
/// run.
fn report_by_reference_element(item: &ArrayItem, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_ARRAY_ELEMENT_BY_REFERENCE,
            "an array element cannot be taken by reference",
        )
        .with_primary(item.span, "this element is `&value`")
        .with_help(
            "Novis has no references: `rule:types/implicit-capture` removed by-reference capture and `rule:classes/two-copy-depths` \
             makes an element a copy, so drop the `&` — to share one mutable cell, hold it in \
             an object and store that",
        ),
    );
}

/// `rule:types/arrays`: every array key is a `string`, and an `int`/`uint` key
/// normalizes to its own decimal string — key normalization, not a value
/// conversion, so neither needs a diagnostic here. A `float`, `bool`, or
/// `null` key is rejected outright: PHP's silent truncate-to-int/stringify-
/// to-`"1"`/`""` is exactly the kind of implicit conversion that turns a
/// typo into a missing row rather than a diagnostic. An **enum** is rejected
/// beside them, and for the same reason one step further out: `rule:enums/closed-integer-type` makes a
/// case a named integer, so the normalization would silently key the array by
/// a backing value two enums can share — `E0708` refuses `$case as string` on
/// that reading already, and this is the position where no `as` was written at
/// all. Anything else — `mixed`,
/// a union, an object, ... — isn't statically known to be one of these four,
/// so it is left alone here, the same "erase to `mixed` rather than guess"
/// split `division_result`/`bitwise_result` already draw for an operand pair
/// they don't recognize; `rule:types/arrays`'s own runtime normalization/throw
/// covers it once a value arrives through `mixed`.
pub(crate) fn check_array_key_type(key_ty: TypeId, span: Span, env: &mut Env<'_>) {
    if matches!(env.interner.get(key_ty), Ty::Float | Ty::Bool | Ty::Null) {
        env.diags.report(
            Diagnostic::error(
                code::E_ARRAY_KEY_INVALID_TYPE,
                "a `float`, `bool`, or `null` array key is not allowed",
            )
            .with_primary(span, "this key")
            .with_help("array keys are `int`, `uint`, or `string` — convert explicitly with `as`"),
        );
        return;
    }
    let Ty::Enum(qname, _) = env.interner.get(key_ty) else {
        return;
    };
    let qname = qname.clone();
    env.diags.report(
        Diagnostic::error(
            code::E_ARRAY_KEY_INVALID_TYPE,
            format!("`{qname}` is an enum, and an enum case is not an array key"),
        )
        .with_primary(span, "this key")
        .with_help(
            "`rule:enums/closed-integer-type` makes an enum case a named integer, so normalizing one would key the \
             array by a backing value two enums can share — write `$case as int` where that \
             number is the key you mean, or a member of your own that names the case where \
             it is not (`as string` is `E0708` for the same reason)",
        ),
    );
}
