//! How a literal takes its type from the position it appears in, and what its
//! own text has to survive first.
//!
//! Three rules meet here, one per literal shape. ADR 0007 § 4 bounds an
//! integer literal's magnitude: the bare digit run either fits `int`'s
//! `0..=i64::MAX` half, `uint`'s full range where a `uint` is expected, or
//! neither ([`int_literal_digits`]). ADR 0054 §§ 1-4 place a fractional
//! literal at `decimal` or `float` by the position rather than by its own
//! spelling ([`wants_decimal`], [`record_decimal_placement`]) and bound the
//! mantissa and scale it may carry. A string literal's escape grammar and a
//! heredoc's flexible indentation are cooked by [`crate::string_lit`] and
//! their complaints reported here, once per literal, so a malformed escape is
//! a diagnostic rather than a lowering-time surprise.
//!
//! [`check_array_literal`] is ADR 0007 § 5's half of the same idea for the one
//! composite literal: an array literal checked against an `array<T>` target
//! checks every element directly against `T`, never inferring an element type
//! and comparing it afterwards.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// ADR 0054 § 1's mantissa bound: 96 bits, unsigned, with the sign carried
/// beside it rather than in it.
const MAX_DECIMAL_MANTISSA: u128 = (1u128 << 96) - 1;

/// ADR 0054 § 1's scale bound: the number of digits after the point.
const MAX_DECIMAL_SCALE: i32 = 28;

/// Records that the numeric literal at `span` was placed at `decimal`, and
/// answers that type.
///
/// The recording is what lets `mwl_ir::lower` fold the literal from its own
/// **digits** rather than through an `f64`. It needs it because ADR 0054 § 2's
/// placing target is not always visible there: a declared type reaches
/// lowering as `mwl_ir::ty::Ty`, which erases an array's element type, so
/// `array<decimal> $prices = [19.99];` would otherwise put a `float` in the
/// array the checker just typed `decimal`. Reading the answer back is the same
/// arrangement [`crate::expr_table::ExprTypeTable::declared_ty`] already
/// serves for an enum-named type atom.
pub(super) fn record_decimal_placement(span: Span, env: &mut Env<'_>) -> TypeId {
    let decimal = env.interner.decimal();
    env.exprs.record_type(span, decimal);
    decimal
}

/// Whether the position a literal is being placed in wants a `decimal` —
/// ADR 0054 § 2's "untyped until placed" rule, asked once per literal arm.
pub(super) fn wants_decimal(expected: Option<TypeId>, env: &Env<'_>) -> bool {
    expected.is_some_and(|id| matches!(env.interner.get(id), Ty::Decimal))
}

/// ADR 0054 § 1's layout, applied to a fractional literal's own text: a 96-bit
/// mantissa and a scale of 0 to 28. Returns the reason it does not fit, or
/// `None` when it does.
///
/// An exponent is folded into the scale rather than rejected — `1.5e3` is
/// mantissa 1500 at scale 0, and `1.5e-30` is a scale-31 value this refuses.
/// Trailing zeros are *kept*, because § 4 makes scale observable in rendering:
/// `19.90 as string` is `"19.90"`, so `19.90` is a scale-2 value and not a
/// second spelling of `19.9`.
pub(super) fn decimal_literal_overflow(text: &str) -> Option<&'static str> {
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

/// Reports ADR 0054 § 1's bound for a fractional literal placed at `decimal`.
pub(super) fn check_decimal_float_literal(span: Span, report_span: Span, env: &mut Env<'_>) {
    let text = span_text(env.src, span).to_owned();
    if let Some(reason) = decimal_literal_overflow(&text) {
        report_decimal_out_of_range(reason, report_span, env);
    }
}

/// The integer-literal half of [`check_decimal_float_literal`]: only the
/// mantissa can overflow, since an integer literal is scale 0 by construction.
pub(super) fn check_decimal_int_literal(span: Span, report_span: Span, env: &mut Env<'_>) {
    let (radix, digits) = int_literal_digits(env.src, span);
    if !u128::from_str_radix(&digits, radix).is_ok_and(|m| m <= MAX_DECIMAL_MANTISSA) {
        report_decimal_out_of_range("mantissa", report_span, env);
    }
}

pub(super) fn report_decimal_out_of_range(reason: &str, span: Span, env: &mut Env<'_>) {
    let detail = match reason {
        "scale" => "more than 28 digits after the point",
        _ => "a mantissa wider than 96 bits",
    };
    env.diags.report(
        Diagnostic::error(
            code::E_DECIMAL_LITERAL_OUT_OF_RANGE,
            format!("this literal does not fit `decimal`: it has {detail}"),
        )
        .with_primary(span, "outside `decimal`'s range")
        .with_help(
            "`decimal` holds a 96-bit mantissa at a scale of 0 to 28 (ADR 0054 § 1); \
             `Core\\BigDecimal` is the type for a value beyond it",
        ),
    );
}

/// Splits an integer-literal span's cooked text into the radix its prefix
/// names and the digit run to parse against it — the same job
/// `mwl_ir::lower::int_literal_digits` does for lowering, duplicated here
/// rather than shared: this crate has no dependency on `mwl-ir` (the
/// dependency runs the other way), and the magnitude has to be known here,
/// at check time, so [`infer_int_literal`] can report ADR 0007 § 4's
/// diagnostic itself rather than let an out-of-range literal surface only as
/// a lowering-time panic once `mwl-ir` tries to cook the same span. Strips
/// `_` digit separators the same way; a legacy leading-zero octal spelling
/// like PHP's `0755` is deliberately not one of the recognized prefixes (see
/// `mwl_ir`'s own copy of this function for why), so it falls through to the
/// decimal case, matching `mwl-syntax`'s lexer.
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
pub(super) fn inner_quoted_span(span: Span) -> Span {
    Span::new(span.file, span.start + 1, span.end - 1)
}

/// Cooks `span` (already known to be double-quoted-grammar text — see the two
/// call sites in [`infer`]) purely to surface [`crate::string_lit::CookIssue`]s
/// as diagnostics; the cooked `String` itself is discarded here; `mwl-ir`
/// re-cooks it from the same span when it actually lowers the literal, per
/// `crate::string_lit`'s own module docs on why that duplicate call is safe
/// (one shared implementation) rather than a second, divergent one.
pub(super) fn check_double_quoted_text_issues(span: Span, env: &mut Env<'_>) {
    let (_, issues) = crate::string_lit::cook_double_quoted_text(env.src, span);
    report_cook_issues(issues, env);
}

pub(super) fn report_cook_issues(issues: Vec<crate::string_lit::CookIssue>, env: &mut Env<'_>) {
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
                        "this string literal's `\\xHH`/octal byte escapes do not form valid \
                         UTF-8 once assembled — `string` is guaranteed-valid UTF-8, see ADR 0009",
                    )
                    .with_primary(span, "not valid UTF-8"),
                );
            }
        }
    }
}

pub(super) fn report_heredoc_indent_issues(
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
/// `mwl-ir` re-cooks it from the same inputs when it actually lowers the literal.
pub(super) fn check_heredoc_run_issues(
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

pub(super) fn check_array_literal(
    items: &[ArrayItem],
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let elem_expected = expected.and_then(|id| match env.interner.get(id) {
        Ty::Array(elem) => Some(*elem),
        _ => None,
    });
    for item in items {
        if let Some(key) = &item.key {
            let key_ty = check_expr(key, None, live, scope, ctx, env);
            check_array_key_type(key_ty, key.span, env);
        }
        check_expr(&item.value, elem_expected, live, scope, ctx, env);
    }
    match (expected, elem_expected) {
        (Some(id), Some(_)) => id,
        _ => {
            let mixed = env.interner.mixed();
            env.interner.array(mixed)
        }
    }
}

/// ADR 0007 § 5: every array key is a `string`, and an `int`/`uint` key
/// normalizes to its own decimal string — key normalization, not a value
/// conversion, so neither needs a diagnostic here. A `float`, `bool`, or
/// `null` key is rejected outright: PHP's silent truncate-to-int/stringify-
/// to-`"1"`/`""` is exactly the kind of implicit conversion that turns a
/// typo into a missing row rather than a diagnostic. Anything else — `mixed`,
/// a union, an object, ... — isn't statically known to be one of these four,
/// so it is left alone here, the same "erase to `mixed` rather than guess"
/// split `division_result`/`bitwise_result` already draw for an operand pair
/// they don't recognize; ADR 0007 § 5's own runtime normalization/throw
/// covers it once a value arrives through `mixed`.
pub(super) fn check_array_key_type(key_ty: TypeId, span: Span, env: &mut Env<'_>) {
    if matches!(env.interner.get(key_ty), Ty::Float | Ty::Bool | Ty::Null) {
        env.diags.report(
            Diagnostic::error(
                code::E_ARRAY_KEY_INVALID_TYPE,
                "a `float`, `bool`, or `null` array key is not allowed",
            )
            .with_primary(span, "this key")
            .with_help("array keys are `int`, `uint`, or `string` — convert explicitly with `as`"),
        );
    }
}
