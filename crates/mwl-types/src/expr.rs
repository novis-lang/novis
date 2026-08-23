//! A minimal bidirectional expression checker.
//!
//! [`check_expr`] takes an optional expected type: an [`mwl_syntax::ast::ExprKind::ArrayLiteral`]
//! checked against an `array<T>` target checks every element directly
//! against `T` (ADR 0007 § 5 — "never inferred and then compared"); anything
//! else infers its type bottom-up and, when an expected type was given,
//! reports `E_TYPE_MISMATCH` on a mismatch via [`is_assignable`].
//!
//! Beyond literals, variable reads, the binary-operator result-type table
//! (ADR 0007 § 4, including refusing `int ⊕ uint`), `as` conversions (the
//! only conversion spelling — ADR 0034 rejects PHP's legacy `(T)expr` cast
//! syntax outright) and array literals, a property access, method call,
//! static call/property,
//! `new` and `match`/ternary are now typed too — see [`class_qname_of`] and
//! its callers below. A method/static call not statically resolvable to a
//! known signature (an unresolved receiver, a dynamic member name, a
//! `Core`-namespaced target with no modeled stdlib signature) still falls
//! back to `mixed` with no diagnostic, same as everywhere else this checker
//! only reports what it can be sure of. A closure's body is the one
//! remaining form walked only for nested variable reads and reported as
//! `mixed`; see the crate docs' known gaps for why.
//!
//! ADR 0013's `Comparable` check for the five ordering operators has a
//! sibling now: [`require_stringable`] refuses an object at every implicit
//! string-conversion site (interpolation, concatenation, `echo`/`print`,
//! `as string`/`(string)`) unless it provably implements the reserved
//! global `Stringable` interface (ADR 0028 § 1), and `unset()` on any
//! *declared* object property is refused outright regardless of nullability
//! (ADR 0028 § 3, via [`check_unset_target`]/[`check_property_access`]).
//!
//! **ADR 0024 §§ 2-3 (`tainted` propagation and laundering) and ADR 0033
//! §§ 2-4 (`secret`, the same shape on an independent axis)** are implemented
//! together here, since the two qualifiers share one representation
//! ([`Ty::TaintedString`]/[`Ty::SecretString`]/[`Ty::SecretTaintedString`]/
//! etc. — one atom per combination) and one set of helpers
//! ([`is_tainted`]/[`is_secret`]/[`qualifiable_base`]/[`qualified_scalar`]):
//! concatenation and interpolation poison their result on each axis
//! independently, exactly like ADR 0007's `mixed`-arithmetic precedent,
//! applied in the `Binary`/`Interpolated` arms below;
//! [`apply_qualifier_conversion_rule`] is `ExprKind::Conversion`'s qualifier
//! half — a checked conversion to `uint`/`int`/`float`/`bool`/an enum's
//! backing type launders both qualifiers for free, since none of those
//! targets carry either to begin with (a known, ADR-accepted gap for
//! `secret`: unlike `tainted`, "shape-proof implies safe" doesn't actually
//! transfer, see ADR 0033 § 2's own *Alternatives rejected*), while
//! `bytes`/`string` (including the identity-shaped `tainted string as
//! string`/`secret string as string`, which would otherwise be a silent
//! bypass) keep both qualifiers across either direction, per ADR 0009 § 3.
//! [`is_assignable`] gains one more amendment, generalized over both axes: a
//! same-base value widens freely on either bit (a trusted, non-secret value
//! is always a safe over-approximation of "may be tainted"/"may be secret,"
//! the same direction `mixed` never gets) but never narrows through ordinary
//! assignment. [`reject_non_literal_markup_conversion`] is ADR 0024 § 5's one
//! M2-scoped rule: `as Core\Html\Markup` accepts only a literal string
//! token, `tainted` or not — the rest of § 5 (auto-escaping, `Markup +
//! Markup`) waits on `Core\Html` actually existing.
//! [`reject_secret_markup_conversion`] is ADR 0033 § 4's sibling, giving a
//! `secret` operand there its own specific diagnostic ahead of the generic
//! one (escaping doesn't restore confidentiality, so `secret` gets no
//! auto-escape carve-out even once one exists for `tainted`), and
//! [`reject_secret_throwable_message`] is its other M2-reachable sink: a
//! `Throwable`-shaped class's constructor message argument — see
//! [`is_throwable_shaped`] for how that's decided without a declared
//! `Throwable`/`Exception`/`Error` stdlib to check against.
//!
//! **ADR 0027 (`callable` is closures only)** also lives here:
//! [`report_non_callable_value_if_applicable`] gives a bare string or
//! `[$obj, 'method']`-shaped array literal a targeted diagnostic naming the
//! first-class-callable-syntax replacement wherever `callable` is the
//! expected type, ahead of [`is_assignable`]'s generic mismatch (which would
//! otherwise also fire for the same expression); [`report_call_on_non_callable`]
//! refuses `$obj(...)` for any `$obj` whose static type is a resolved class —
//! MWL has no `__invoke`, so no class ever makes `()` mean anything else.
//!
//! **Diagnosing a missing member is split by receiver, not duplicated:** a
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
//! `isset(...)`/`empty(...)` are a deliberate exception: PHP tolerates an
//! unset operand there by design, and whether that still holds once every
//! local is declared and flow-checked is an open language question beyond
//! this slice, so their operands are left entirely unchecked rather than
//! guessed at.
//!
//! [`is_assignable`] carries ADR 0036's two amendments to ADR 0007 § 6's
//! table: every class or shape type is `<: object` (§ 1), and a shape
//! target is checked structurally by width subtyping plus ordinary field
//! assignability (§ 3, [`shape_satisfied`]) rather than nominally — MWL's
//! one deliberate exception to otherwise fully nominal typing.
//! [`ExprKind::ObjectLiteral`]'s own type is the exact-fields shape its
//! initializers infer, so it flows into a narrower shape or plain `object`
//! target for free through that same rule. [`check_property_access`]'s
//! shape/`object` arms are the M2 half of ADR 0036 § 4: a field a shape
//! names types cleanly with no diagnostic either way; a name it doesn't
//! list, or a plain `object` receiver, is silently `mixed` rather than
//! `E_UNKNOWN_MEMBER` — deferred to ADR 0014 § 5's runtime-checked fallback,
//! which needs M4's IR/codegen to actually throw from and so has no code
//! yet.

use mwl_diagnostics::{Diagnostic, SourceFile, Span, code};
use mwl_hir::{ClassGraph, QName, SymbolKind};
use mwl_syntax::ast::{
    Arg, ArrayItem, AssignOp, BinaryOp, CallArgs, Expr, ExprKind, MemberName, NewTarget,
    StringPart, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::expr_table::{ExprInfo, ResolvedCall};
use crate::locals::LocalScope;
use crate::lower::lower_type;
use crate::signatures::{MethodSig, SignatureTable, resolve_method, resolve_property};
use crate::ty::{Ty, TypeId, TypeInterner};
use crate::{Ctx, Env, span_text, strip_sigil};

/// Builds the [`ExprInfo::Call`] entry [`crate::expr_table::ExprTypeTable`]
/// persists for a resolved method/static call — the one place `qname`/`name`/
/// `sig` (already computed for this call's own type-checking) get bundled
/// into the shape `mwl-ir` reads back, so the `MethodCall`/`StaticCall`/`New`
/// arms below don't each repeat the field list.
fn resolved_call(qname: QName, name: String, sig: &MethodSig) -> ResolvedCall {
    ResolvedCall {
        class: qname,
        method: name,
        param_tys: sig.params.clone(),
        by_ref: sig.by_ref.clone(),
        variadic: sig.variadic,
        is_static: sig.is_static,
        return_ty: sig.return_ty,
        has_body: sig.has_body,
        // Set only by the `StaticCall` arm, and only for an explicitly named
        // class — see the field's own doc comment.
        static_class: None,
    }
}

/// Checks `expr`, optionally against `expected`, returning the type it was
/// found (or, for an array literal checked against a target, declared) to
/// have. Reports `E_TYPE_MISMATCH` when `expected` is given and not
/// satisfied.
pub(crate) fn check_expr(
    expr: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let actual = infer(expr, expected, live, scope, ctx, env);
    if let Some(expected_id) = expected {
        let wants_callable = matches!(env.interner.get(expected_id), Ty::Callable);
        if wants_callable && report_non_callable_value_if_applicable(expr, env) {
            return actual;
        }
        if !is_assignable(actual, expected_id, env.interner, env.graph, env.signatures) {
            report_mismatch(expr.span, expected_id, actual, env);
        }
    }
    actual
}

/// ADR 0027 § 1/§ 3: a bare string or `[$obj, 'method']`-shaped array
/// literal reaching a `callable`-typed position gets a targeted diagnostic
/// naming the first-class-callable-syntax replacement, rather than the
/// generic `E_TYPE_MISMATCH` [`is_assignable`] would otherwise report for
/// the same expression. Returns whether it reported one, so the caller can
/// skip its own generic check for this expression.
fn report_non_callable_value_if_applicable(expr: &Expr, env: &mut Env<'_>) -> bool {
    match &expr.kind {
        ExprKind::Str(_) | ExprKind::Interpolated(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_STRING_UNSUPPORTED,
                    "a string is not callable in MWL; take a reference with first-class \
                     callable syntax instead",
                )
                .with_primary(expr.span, "this string")
                .with_help("e.g. `Class::method(...)` or `$obj->method(...)`"),
            );
            true
        }
        ExprKind::ArrayLiteral(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_ARRAY_UNSUPPORTED,
                    "an array is not callable in MWL; take a reference with first-class \
                     callable syntax instead",
                )
                .with_primary(expr.span, "this array")
                .with_help("e.g. `$obj->method(...)` instead of `[$obj, 'method']`"),
            );
            true
        }
        _ => false,
    }
}

/// Whether a value of type `from` may be used where `to` is declared —
/// `to == mixed` always accepts; `from == mixed` never implicitly satisfies
/// a non-`mixed` target (ADR 0007 § 6: "`mixed` never absorbs implicitly in
/// the other direction"); otherwise `from` must equal `to`, or `to` must be
/// a union `from` is (or, if `from` is itself a union, every member is) a
/// member of. ADR 0036 § 1 amends this with real `object` subtyping (every
/// class or shape is `<: object`), and § 3 with a shape target's structural
/// check (see [`shape_satisfied`]) — the two amendments this ADR makes to
/// ADR 0007 § 6's table, needing `graph`/`signatures` only to resolve a
/// class receiver's own property types against a shape target. ADR 0024 § 2
/// and ADR 0033 § 2 add one more: a same-base `string`/`bytes` value widens
/// freely on its `tainted`/`secret` axes (see the qualifier check just above
/// [`shape_satisfied`]'s call), never narrows.
#[must_use]
pub(crate) fn is_assignable(
    from: TypeId,
    to: TypeId,
    interner: &TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    if from == to {
        return true;
    }
    if matches!(interner.get(to), Ty::Mixed) {
        return true;
    }
    if matches!(interner.get(from), Ty::Mixed) {
        return false;
    }
    if let Ty::Union(members) = interner.get(to) {
        return match interner.get(from) {
            Ty::Union(from_members) => from_members.iter().all(|m| members.contains(m)),
            _ => members.contains(&from),
        };
    }
    if matches!(interner.get(to), Ty::Object)
        && matches!(interner.get(from), Ty::Class(..) | Ty::Shape(_))
    {
        return true;
    }
    if let Ty::Shape(to_fields) = interner.get(to) {
        return shape_satisfied(from, to_fields, interner, graph, signatures);
    }
    // ADR 0024 § 2 / ADR 0033 § 2: `tainted` and `secret` are two independent
    // bits on the same `string`/`bytes` base, and each may only ever widen
    // through ordinary assignment — a plain value is always a safe
    // over-approximation of "may be tainted"/"may be secret," but never the
    // reverse. `from` is assignable to a same-base `to` exactly when every
    // qualifier bit `from` carries, `to` carries too (a strict superset is
    // fine; a missing bit is this whole mechanism's point).
    if let (Some(from_is_bytes), Some(to_is_bytes)) = (
        qualifiable_base(from, interner),
        qualifiable_base(to, interner),
    ) {
        let tainted_ok = !is_tainted(from, interner) || is_tainted(to, interner);
        let secret_ok = !is_secret(from, interner) || is_secret(to, interner);
        if from_is_bytes == to_is_bytes && tainted_ok && secret_ok {
            return true;
        }
    }
    false
}

/// ADR 0036 § 3's structural check for a shape target: `from` must have at
/// least every field `to_fields` names, each satisfying the field's declared
/// type by this same [`is_assignable`] rule (width subtyping — an extra
/// field on `from` is never a problem). A class receiver's field types come
/// from [`resolve_property`], the same ancestor walk an ordinary `$obj->prop`
/// access already uses; any other `from` (a scalar, `object`, a mismatched
/// shape) never satisfies a shape target.
fn shape_satisfied(
    from: TypeId,
    to_fields: &[(String, TypeId)],
    interner: &TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    match interner.get(from) {
        Ty::Shape(from_fields) => to_fields.iter().all(|(name, field_ty)| {
            from_fields
                .iter()
                .find(|(n, _)| n == name)
                .is_some_and(|(_, from_field_ty)| {
                    is_assignable(*from_field_ty, *field_ty, interner, graph, signatures)
                })
        }),
        Ty::Class(qname, _) => to_fields.iter().all(|(name, field_ty)| {
            resolve_property(qname, name, signatures, graph).is_some_and(|from_field_ty| {
                is_assignable(from_field_ty, *field_ty, interner, graph, signatures)
            })
        }),
        _ => false,
    }
}

/// Checks a `return expr;`'s value against the method's declared return
/// type, reporting `E_BAD_RETURN_TYPE` — distinct wording from the generic
/// `E_TYPE_MISMATCH` [`check_expr`] itself reports, for what is structurally
/// the same assignability question.
pub(crate) fn check_return(
    expr: &Expr,
    return_ty: TypeId,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let actual = infer(expr, Some(return_ty), live, scope, ctx, env);
    if !is_assignable(actual, return_ty, env.interner, env.graph, env.signatures) {
        let expected_desc = env.interner.describe(return_ty);
        let actual_desc = env.interner.describe(actual);
        env.diags.report(
            Diagnostic::error(
                code::E_BAD_RETURN_TYPE,
                format!("this method declares `{expected_desc}` but returns `{actual_desc}`"),
            )
            .with_primary(expr.span, format!("this is `{actual_desc}`")),
        );
    }
}

fn report_mismatch(span: Span, expected: TypeId, actual: TypeId, env: &mut Env<'_>) {
    let expected_desc = env.interner.describe(expected);
    let actual_desc = env.interner.describe(actual);
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_MISMATCH,
            format!("expected `{expected_desc}`, found `{actual_desc}`"),
        )
        .with_primary(span, format!("this is `{actual_desc}`")),
    );
}

/// Splits an integer-literal span's cooked text into the radix its prefix
/// names and the digit run to parse against it — the same job
/// `mwl_ir::lower::int_literal_digits` does for lowering, duplicated here
/// rather than shared: this crate has no dependency on `mwl-ir` (the
/// dependency runs the other way), and the magnitude has to be known here,
/// at check time, so [`infer`]'s `ExprKind::Int` arm can report ADR 0007 § 4's
/// diagnostic itself rather than let an out-of-range literal surface only as
/// a lowering-time panic once `mwl-ir` tries to cook the same span. Strips
/// `_` digit separators the same way; a legacy leading-zero octal spelling
/// like PHP's `0755` is deliberately not one of the recognized prefixes (see
/// `mwl_ir`'s own copy of this function for why), so it falls through to the
/// decimal case, matching `mwl-syntax`'s lexer.
fn int_literal_digits(src: &SourceFile, span: Span) -> (u32, String) {
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
fn inner_quoted_span(span: Span) -> Span {
    Span::new(span.file, span.start + 1, span.end - 1)
}

/// Cooks `span` (already known to be double-quoted-grammar text — see the two
/// call sites in [`infer`]) purely to surface [`crate::string_lit::CookIssue`]s
/// as diagnostics; the cooked `String` itself is discarded here; `mwl-ir`
/// re-cooks it from the same span when it actually lowers the literal, per
/// `crate::string_lit`'s own module docs on why that duplicate call is safe
/// (one shared implementation) rather than a second, divergent one.
fn check_double_quoted_text_issues(span: Span, env: &mut Env<'_>) {
    let (_, issues) = crate::string_lit::cook_double_quoted_text(env.src, span);
    report_cook_issues(issues, env);
}

fn report_cook_issues(issues: Vec<crate::string_lit::CookIssue>, env: &mut Env<'_>) {
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

fn report_heredoc_indent_issues(
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
fn check_heredoc_run_issues(
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

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines"
)]
fn infer(
    expr: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match &expr.kind {
        ExprKind::Null => env.interner.null(),
        ExprKind::Bool(_) => env.interner.bool_ty(),
        // ADR 0007 § 4: "An integer literal that does not fit `int` is legal
        // only where a `uint` is expected, and is otherwise a diagnostic
        // saying exactly that." The literal's own digits are never negative —
        // a leading `-` is a separate, wrapping `ExprKind::Unary` node (see
        // that arm below), which already produces an ordinary `int`/`uint`
        // type mismatch on its own when negated and assigned into a `uint`
        // target, with no magnitude check needed for that half. What *does*
        // need one: whether the bare digit run fits `int`'s `0..=i64::MAX`
        // half, `uint`'s full `0..=u64::MAX` range, or neither at all.
        ExprKind::Int(span) => {
            let wants_uint = expected.is_some_and(|id| matches!(env.interner.get(id), Ty::Uint));
            let (radix, digits) = int_literal_digits(env.src, *span);
            match u64::from_str_radix(&digits, radix) {
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
                            "this integer literal is too large for `int`; it is only legal \
                             where a `uint` is expected",
                        )
                        .with_primary(expr.span, "does not fit `int`"),
                    );
                    env.interner.int()
                }
                Err(_) => {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_INT_LITERAL_OUT_OF_RANGE,
                            "this integer literal is too large to represent in either `int` or \
                             `uint`",
                        )
                        .with_primary(expr.span, "too large for a 64-bit integer"),
                    );
                    if wants_uint {
                        env.interner.uint()
                    } else {
                        env.interner.int()
                    }
                }
            }
        }
        ExprKind::Float(_) => env.interner.float(),
        ExprKind::Str(span) => {
            // A single-quoted literal's own two escapes (`\\`/`\'`) can
            // never produce invalid UTF-8, so it gets no cooking-diagnostic
            // pass at all. A double-quoted literal runs the richer escape
            // grammar `check_double_quoted_text_issues` cooks. A
            // heredoc/nowdoc-sourced `Str` (whose span opens with `<`, not a
            // quote) runs `crate::string_lit`'s flexible-indentation check
            // first, then the same escape grammar too — unless it's a
            // nowdoc, which (like PHP's) applies no escapes at all.
            let raw = span_text(env.src, *span);
            if raw.starts_with('"') {
                check_double_quoted_text_issues(inner_quoted_span(*span), env);
            } else if raw.starts_with("<<<") {
                let (shape, indent_issues) = crate::string_lit::heredoc_shape(env.src, *span);
                report_heredoc_indent_issues(indent_issues, env);
                let run_escapes = !crate::string_lit::heredoc_is_nowdoc(raw);
                check_heredoc_run_issues(&shape.indent, shape.body, true, true, run_escapes, env);
            }
            env.interner.string()
        }
        ExprKind::Interpolated(parts) => {
            // Only a heredoc/nowdoc can ever reach this arm with the
            // opening `<<<`-only span it needs its own flexible-indentation
            // strip (`mwl_syntax::parser::collapse_string_parts` never
            // produces a nowdoc `Interpolated` at all: a nowdoc has no
            // interpolation syntax by construction, so it always collapses
            // to `ExprKind::Str`, whose arm above already handles it).
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
                        tainted |= is_tainted(ty, env.interner);
                        secret |= is_secret(ty, env.interner);
                    }
                    // A `Text` run's escapes follow exactly the same grammar
                    // regardless of whether the overall literal is
                    // double-quoted or an interpolated heredoc — see
                    // `crate::string_lit`'s own module docs for why one
                    // routine cooks both. The span never includes a quote
                    // character (`mwl_syntax::parser::parse_string_body`
                    // never emits one as part of a `Text` token), so no
                    // quote-kind check is needed here the way `Str` above
                    // needs one — except a heredoc's own flexible
                    // indentation, which has to be stripped from each run
                    // first (`is_heredoc`'s own doc comment above: this
                    // literal is never a nowdoc, so escapes always run).
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
        ExprKind::Variable(span) => {
            let name = strip_sigil(span_text(env.src, *span)).to_owned();
            check_read(&name, expr.span, live, scope, env)
        }
        ExprKind::ConstFetch(_) => env.interner.mixed(),
        ExprKind::SelfExpr | ExprKind::StaticExpr => class_of_ctx(ctx, env),
        ExprKind::ParentExpr => env.interner.mixed(),
        ExprKind::ArrayLiteral(items) => {
            check_array_literal(items, expected, live, scope, ctx, env)
        }
        // ADR 0036 § 2: each field's type is inferred from its own
        // initializer (same idea as an `array<T>` literal's element type),
        // and the literal's precise type is the exact-fields shape those
        // infer to — `is_assignable`'s width subtyping is what lets it flow
        // into a narrower shape or plain `object` target on its own.
        ExprKind::ObjectLiteral(fields) => {
            let mut out = Vec::with_capacity(fields.len());
            for field in fields {
                let name = span_text(env.src, field.name).to_owned();
                let field_ty = check_expr(&field.value, None, live, scope, ctx, env);
                out.push((name, field_ty));
            }
            env.interner.shape(out)
        }
        ExprKind::Unary { op, expr: inner } => {
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            match op {
                UnaryOp::Not => env.interner.bool_ty(),
                UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot | UnaryOp::Suppress => inner_ty,
                _ => inner_ty,
            }
        }
        ExprKind::PreIncDec { expr: inner, .. } | ExprKind::PostIncDec { expr: inner, .. } => {
            check_expr(inner, None, live, scope, ctx, env)
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let lhs_ty = check_expr(lhs, None, live, scope, ctx, env);
            let rhs_ty = check_expr(rhs, None, live, scope, ctx, env);
            if *op == BinaryOp::Concat {
                require_stringable(lhs_ty, lhs.span, env);
                require_stringable(rhs_ty, rhs.span, env);
            }
            binary_result(*op, lhs_ty, rhs_ty, expr.span, env)
        }
        ExprKind::Assign {
            op, target, value, ..
        } => check_assign(*op, target, value, live, scope, ctx, env),
        ExprKind::Ternary { cond, then, else_ } => {
            let cond_ty = check_expr(cond, None, live, scope, ctx, env);
            // `$a ?: $b` (`then` omitted) evaluates to `$a` itself on the
            // truthy path — its type joins the union the same way an
            // explicit `then` branch would.
            let then_ty = match then {
                Some(then) => check_expr(then, None, live, scope, ctx, env),
                None => cond_ty,
            };
            let else_ty = check_expr(else_, None, live, scope, ctx, env);
            env.interner.make_union([then_ty, else_ty])
        }
        ExprKind::Conversion { expr: inner, ty } => {
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            let result = lower_type(ty, ctx, env);
            if matches!(env.interner.get(result), Ty::String) {
                require_stringable(inner_ty, inner.span, env);
            }
            reject_enum_to_enum_conversion(inner_ty, result, expr.span, env);
            reject_secret_markup_conversion(inner_ty, result, expr.span, env);
            reject_non_literal_markup_conversion(inner, result, expr.span, env);
            apply_qualifier_conversion_rule(inner_ty, result, env.interner)
        }
        ExprKind::InstanceOf { expr: inner, class } => {
            check_expr(inner, None, live, scope, ctx, env);
            // A bare `Foo` on the right of `instanceof` is a class name, not a
            // constant read — recorded here so `mwl-ir` never has to resolve
            // one (see `crate::expr_table::ExprInfo::InstanceOf`). Anything
            // else is the dynamic form, which still checks as an ordinary
            // expression and records nothing.
            if let ExprKind::ConstFetch(name) = &class.kind {
                let text = span_text(env.src, name.span);
                let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);
                if env.symbols.get(&qname).is_some() || qname.is_reserved_global_class() {
                    env.exprs
                        .record(expr.span, ExprInfo::InstanceOf { class: qname });
                }
            } else {
                check_expr(class, None, live, scope, ctx, env);
            }
            env.interner.bool_ty()
        }
        ExprKind::Call { callee, args } => {
            let callee_ty = check_expr(callee, None, live, scope, ctx, env);
            check_args(args, live, scope, ctx, env);
            if matches!(args, CallArgs::FirstClassCallable) {
                return env.interner.callable();
            }
            report_call_on_non_callable(callee_ty, expr.span, env);
            env.interner.mixed()
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            let object_ty = check_expr(object, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            // Unlike a static call, `mwl_hir::members` never checks an
            // instance method call's existence for any receiver — including
            // `$this` — so this is the first and only place it's diagnosed.
            let resolved = match (class_qname_of(object_ty, env.interner), method) {
                (Some(qname), MemberName::Ident(name_span)) => {
                    let name = span_text(env.src, *name_span).to_owned();
                    let found = resolve_method(&qname, &name, env.signatures, env.graph);
                    if found.is_none() && !qname.is_core() && !qname.is_reserved_global_class() {
                        report_unknown_member(object.span, &qname, &name, "method", env);
                    }
                    if let Some((owner, sig)) = &found {
                        check_interface_private_visibility(owner, &name, sig, *name_span, ctx, env);
                    }
                    // The *declaring* class, not the receiver's: that is what
                    // `ResolvedCall::class` promises, and `mwl-ir` renders the
                    // call's target label from it — `$dog->name()` on a `Dog`
                    // that inherits `name` must name `Animal::name`, the
                    // symbol that actually exists.
                    found.map(|(owner, sig)| (owner, name, sig))
                }
                _ => None,
            };
            let sig = resolved
                .as_ref()
                .map(|(owner, _, sig)| substitute_receiver_args(object_ty, owner, sig, env));
            let (_, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
            // ADR 0027: `$obj->method(...)` (first-class callable syntax)
            // names a `Closure` value, not the method's return type — the
            // sentinel `CallArgs::FirstClassCallable` marks exactly this
            // shape, ahead of the ordinary-call typing below.
            if matches!(args, CallArgs::FirstClassCallable) {
                return env.interner.callable();
            }
            // `mwl-ir` needs this call's resolved target (not just its return
            // type) to lower an eventual instance-call instruction — see
            // `crate::expr_table`'s own module docs.
            // The *substituted* signature, never the one `resolve_method`
            // returned: `crate::generics` guarantees a type variable never
            // survives a call site, and this record is the one thing that
            // carries a signature past it.
            if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
                env.exprs.record(
                    expr.span,
                    ExprInfo::Call(resolved_call(qname.clone(), name.clone(), sig)),
                );
            }
            sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty)
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
        } => {
            check_expr(class, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            // `mwl_hir::members` already checks this reference's existence
            // (`self::`/`static::`/`parent::`/an explicit class name) — this
            // only recovers the call's *type* when a signature resolves, and
            // adds no second diagnostic when it doesn't.
            let resolved = match method {
                MemberName::Ident(name_span) => {
                    resolve_class_expr(class, ctx, env).and_then(|qname| {
                        let name = span_text(env.src, *name_span).to_owned();
                        resolve_method(&qname, &name, env.signatures, env.graph).map(
                            |(owner, sig)| {
                                check_interface_private_visibility(
                                    &owner, &name, &sig, *name_span, ctx, env,
                                );
                                // The declaring class — see the `MethodCall`
                                // arm above for why the receiver's own is the
                                // wrong label.
                                (owner, name, sig)
                            },
                        )
                    })
                }
                _ => None,
            };
            let sig = resolved.as_ref().map(|(_, _, sig)| sig.clone());
            let (_, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
            // See the `MethodCall` arm above: first-class callable syntax
            // names a `Closure`, not the resolved method's return type.
            if matches!(args, CallArgs::FirstClassCallable) {
                return env.interner.callable();
            }
            // See the `MethodCall` arm above: persisted for `mwl-ir` to read
            // back a resolved static call's target.
            // The *substituted* signature, never the one `resolve_method`
            // returned: `crate::generics` guarantees a type variable never
            // survives a call site, and this record is the one thing that
            // carries a signature past it.
            if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
                let mut call = resolved_call(qname.clone(), name.clone(), sig);
                // Late static binding: an explicitly named class *sets* the
                // called class, while `self`/`static`/`parent` forward the
                // caller's. See `ResolvedCall::static_class`.
                if matches!(class.kind, ExprKind::ConstFetch(_)) {
                    call.static_class = resolve_class_expr(class, ctx, env);
                }
                env.exprs.record(expr.span, ExprInfo::Call(call));
            }
            sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty)
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => check_property_access(object, property, false, live, scope, ctx, env),
        ExprKind::StaticPropertyAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            let text = span_text(env.src, *name);
            let prop_name = strip_sigil(text).to_owned();
            resolve_class_expr(class, ctx, env)
                .and_then(|qname| resolve_property(&qname, &prop_name, env.signatures, env.graph))
                .unwrap_or_else(|| env.interner.mixed())
        }
        // ADR 0010 § 4: `EnumName::CaseName` is the one `Class::CONST`-shaped
        // access this checker can already type precisely — `mwl_hir::members`
        // stores a case alongside an ordinary constant in the same
        // `MemberTable` slot and has already checked it exists, so this only
        // recovers the case's type as `Ty::Enum`, same split-by-receiver
        // shape every other static reference in this module uses. An
        // ordinary class constant's own type is unmodeled (`mixed`) either
        // way — see the crate docs' known gaps.
        ExprKind::ClassConstAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            let enum_qname = resolve_class_expr(class, ctx, env).filter(
                |qname| matches!(env.symbols.get(qname), Some(sym) if sym.kind == SymbolKind::Enum),
            );
            match enum_qname {
                Some(qname) => {
                    // ADR 0010 § 3: the case *is* its integer constant, so
                    // `mwl-ir` needs the value, not just the type — see
                    // `ExprInfo::EnumCase`. A name `mwl_hir::members` already
                    // reported as undeclared records nothing.
                    let case = span_text(env.src, *name).to_owned();
                    if let Some(value) = env.enums.case(&qname, &case) {
                        env.exprs.record(expr.span, ExprInfo::EnumCase { value });
                    }
                    let backing = env.enums.backing_of(&qname);
                    env.interner.enum_(qname, backing)
                }
                None => env.interner.mixed(),
            }
        }
        ExprKind::ClassNameConst { class } => {
            check_expr(class, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        // `mwl-ir` needs the element's declared type to lower an eventual
        // indexed read/write instruction — see `crate::expr_table`'s own
        // module docs. Recorded only when `base_ty` statically resolved to a
        // known `Ty::Array` element type, never when it erased to `mixed`
        // (an untyped/unresolved array) — the same "nothing compile-time-
        // known to read" split `check_property_access` already draws for a
        // shape/plain-`object` receiver. `check_assign`'s general (non-plain-
        // local) arm routes an assignment target back through this same
        // function, so a write records exactly the entry a read would, keyed
        // by this `Index` expression's own span either way.
        ExprKind::Index { base, index } => {
            let base_ty = check_expr(base, None, live, scope, ctx, env);
            if let Some(index) = index {
                let index_ty = check_expr(index, None, live, scope, ctx, env);
                check_array_key_type(index_ty, index.span, env);
            }
            let elem_ty = match env.interner.get(base_ty) {
                Ty::Array(elem) => Some(*elem),
                _ => None,
            };
            match elem_ty {
                Some(elem_ty) => {
                    env.exprs.record(expr.span, ExprInfo::Index { elem_ty });
                    elem_ty
                }
                None => env.interner.mixed(),
            }
        }
        ExprKind::New { target, args } => {
            let target_ty = check_new_target(target, live, scope, ctx, env);
            let target_qname = class_qname_of(target_ty, env.interner);
            // A class with no explicit `constructor` accepts a bare `new
            // Foo()` in PHP; not diagnosing an arity mismatch against zero
            // parameters here is deliberate — see the crate docs' known gaps.
            // The *declaring* class is kept, not the constructed one: `new
            // Dog(...)` on a `Dog extends Animal` that declares no constructor
            // of its own invokes `Animal::constructor`, and `mwl-ir` cannot
            // re-walk the hierarchy to find that out (see
            // `crate::expr_table::ExprInfo::New::ctor`).
            let resolved = target_qname
                .clone()
                .and_then(|qname| resolve_method(&qname, "constructor", env.signatures, env.graph));
            let ctor_owner = resolved.as_ref().map(|(owner, _)| owner.clone());
            let sig = resolved.map(|(_, sig)| sig);
            let (arg_types, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
            if let Some(qname) = &target_qname {
                reject_secret_throwable_message(qname, arg_types.first().copied(), expr.span, env);
                // `mwl-ir` needs the constructed class and its resolved
                // constructor (if any) to lower `new` — see
                // `crate::expr_table`'s own module docs.
                let ctor = sig
                    .as_ref()
                    .zip(ctor_owner)
                    .map(|(s, owner)| resolved_call(owner, "constructor".to_owned(), s));
                env.exprs.record(
                    expr.span,
                    ExprInfo::New {
                        class: qname.clone(),
                        ctor,
                        ty: target_ty,
                    },
                );
            }
            target_ty
        }
        ExprKind::Clone(inner) => check_expr(inner, None, live, scope, ctx, env),
        ExprKind::Fn(_) => env.interner.callable(),
        ExprKind::Match { subject, arms } => {
            check_expr(subject, None, live, scope, ctx, env);
            let mut arm_types = Vec::with_capacity(arms.len());
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for c in conds {
                        check_expr(c, None, live, scope, ctx, env);
                    }
                }
                arm_types.push(check_expr(&arm.body, None, live, scope, ctx, env));
            }
            if arm_types.is_empty() {
                env.interner.mixed()
            } else {
                env.interner.make_union(arm_types)
            }
        }
        ExprKind::Yield { key, value } => {
            if let Some(k) = key {
                check_expr(k, None, live, scope, ctx, env);
            }
            if let Some(v) = value {
                check_expr(v, None, live, scope, ctx, env);
            }
            env.interner.mixed()
        }
        ExprKind::YieldFrom(inner) => {
            check_expr(inner, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::Print(inner) => {
            let ty = check_expr(inner, None, live, scope, ctx, env);
            require_stringable(ty, inner.span, env);
            env.interner.int()
        }
        ExprKind::Throw(inner) => {
            check_expr(inner, None, live, scope, ctx, env);
            env.interner.never()
        }
        ExprKind::Isset(_) | ExprKind::Empty(_) => env.interner.bool_ty(),
        ExprKind::Exit(opt) => {
            if let Some(e) = opt {
                check_expr(e, None, live, scope, ctx, env);
            }
            env.interner.never()
        }
        ExprKind::SpawnScript { path, options } => {
            check_expr(path, None, live, scope, ctx, env);
            for opt in options {
                check_expr(&opt.value, None, live, scope, ctx, env);
            }
            env.interner.mixed()
        }
        ExprKind::Require { path } => {
            check_expr(path, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::Paren(inner) => check_expr(inner, expected, live, scope, ctx, env),
        ExprKind::Error => env.interner.mixed(),
        _ => env.interner.mixed(),
    }
}

/// `self`/`static`/`$this`'s type, resolved against the enclosing
/// declaration. ADR 0010: an enum has no methods to reach this from in a
/// well-formed program, but the parser still recovers a member it rejected
/// with `E_ENUM_MEMBER_UNSUPPORTED` (see `mwl-syntax::parser::parse_enum_body`)
/// and hands it to this checker anyway — so this must resolve the same way
/// [`crate::lower::lower_type`]'s `self`/`static` atom already does, an
/// enum-declared `qname` interning to `Ty::Enum` rather than `Ty::Class`.
pub(crate) fn class_of_ctx(ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match ctx.current_class {
        Some(qname) => match env.symbols.get(qname) {
            Some(sym) if sym.kind == SymbolKind::Enum => {
                let backing = env.enums.backing_of(qname);
                env.interner.enum_(qname.clone(), backing)
            }
            _ => env.interner.class(qname.clone()),
        },
        None => env.interner.mixed(),
    }
}

fn check_read(
    name: &str,
    span: Span,
    live: &FxHashSet<String>,
    scope: &LocalScope,
    env: &mut Env<'_>,
) -> TypeId {
    match scope.by_name.get(name) {
        Some(info) if live.contains(name) => info.ty,
        Some(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_VARIABLE,
                    format!("`${name}` is read before any assignment reaches it"),
                )
                .with_primary(span, "not definitely assigned here"),
            );
            env.interner.mixed()
        }
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_VARIABLE,
                    format!("`${name}` is not declared"),
                )
                .with_primary(span, "no declaration for this name"),
            );
            env.interner.mixed()
        }
    }
}

fn check_assign(
    op: AssignOp,
    target: &Expr,
    value: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if let (AssignOp::Assign, ExprKind::Variable(span)) = (op, &target.kind) {
        let name = strip_sigil(span_text(env.src, *span)).to_owned();
        let declared = scope.by_name.get(&name).map(|info| info.ty);
        let value_ty = check_expr(value, declared, live, scope, ctx, env);
        match declared {
            Some(ty) => {
                live.insert(name);
                ty
            }
            None => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNDEFINED_VARIABLE,
                        format!("`${name}` is assigned to but was never declared"),
                    )
                    .with_primary(target.span, "no `LocalDecl` for this name")
                    .with_help("declare it first: `T $name = ...;`"),
                );
                value_ty
            }
        }
    } else {
        let target_ty = check_expr(target, None, live, scope, ctx, env);
        check_expr(value, Some(target_ty), live, scope, ctx, env);
        target_ty
    }
}

fn check_array_literal(
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
fn check_array_key_type(key_ty: TypeId, span: Span, env: &mut Env<'_>) {
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

fn check_new_target(
    target: &NewTarget,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match target {
        NewTarget::Name(name) => {
            let text = span_text(env.src, name.span);
            let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);
            if env.symbols.get(&qname).is_some()
                || qname.is_core()
                || qname.is_reserved_global_class()
            {
                env.interner.class(qname)
            } else {
                // Diagnosed rather than erased to `mixed`: `mwl-ir` has no
                // class to allocate and panics naming the missing table entry,
                // which is a worse report of the same fact. The spelling this
                // most often catches is PHP's `new Exception(…)` — spec § 10
                // has no such class, so the ordinary undeclared-class
                // diagnostic is exactly the right answer.
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNDEFINED_CLASS,
                        format!("`{qname}` is not declared"),
                    )
                    .with_primary(name.span, "no matching declaration"),
                );
                env.interner.mixed()
            }
        }
        NewTarget::SelfTy | NewTarget::StaticTy => class_of_ctx(ctx, env),
        NewTarget::ParentTy => {
            let parent = ctx
                .current_class
                .and_then(|c| env.graph.get(c))
                .and_then(|links| links.extends.first())
                .cloned();
            match parent {
                Some(parent) => env.interner.class(parent),
                None => env.interner.mixed(),
            }
        }
        NewTarget::Expr(e) => {
            check_expr(e, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        NewTarget::AnonClass(_) => env.interner.mixed(),
        _ => env.interner.mixed(),
    }
}

fn check_member_name(
    member: &MemberName,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = member {
        check_expr(e, None, live, scope, ctx, env);
    }
}

fn check_args(
    args: &CallArgs,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    for Arg { value, .. } in list {
        check_expr(value, None, live, scope, ctx, env);
    }
}

/// `sig` with the receiver's own type arguments substituted in — the
/// receiver-driven half of [`crate::generics`]' two binding sites.
///
/// `$cursor->current()` on a receiver typed `Iterator<int>` resolves to
/// `current(): T`, and there is no argument list to read `T` out of: the
/// binding is the receiver's. So the declaring interface's parameter names
/// (from [`mwl_hir::interfaces`], the one roster) are zipped against the
/// receiver's written arguments and the signature is rewritten concrete
/// before a single argument is checked — the same guarantee the argument-side
/// path already gives, that a type variable never survives a call site.
///
/// Everything else is returned untouched, which is every call in a program
/// that does not name one of ADR 0053 § 2's two interfaces: `owner` must be
/// exactly the class the receiver is typed as, so an inherited member reached
/// through an implementing class is deliberately *not* substituted here.
/// Fixing `Counter`'s `T` from its `implements Iterable<int>` clause needs
/// [`crate::signatures::ClassSignature::implements`], and is the iteration
/// lowering's own work rather than this call site's.
fn substitute_receiver_args(
    receiver: TypeId,
    owner: &QName,
    sig: &MethodSig,
    env: &mut Env<'_>,
) -> MethodSig {
    let Ty::Class(qname, args) = env.interner.get(receiver).clone() else {
        return sig.clone();
    };
    if args.is_empty() || &qname != owner {
        return sig.clone();
    }
    let Some(params) = mwl_hir::interfaces::type_params(qname.short_name()) else {
        return sig.clone();
    };
    let bindings: crate::generics::Bindings = params
        .iter()
        .map(|name| (*name).to_owned())
        .zip(args)
        .collect();
    sig.clone().substituted(&bindings, env.interner)
}

/// The class or enum a resolved type names, if it names one at all — the
/// receiver-type question every member-access/call arm below needs answered
/// before it can look anything up in a [`crate::signatures::SignatureTable`].
fn class_qname_of(ty: TypeId, interner: &TypeInterner) -> Option<QName> {
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
fn resolve_class_expr(class_expr: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
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

/// Shared body for a property access, whether it appears as an ordinary
/// expression (`$obj->prop`, `is_unset` false) or as `unset()`'s operand
/// (`is_unset` true) — the receiver/member resolution is identical either
/// way; only what happens once a *declared* property is found differs (ADR
/// 0028 § 3: `unset()` on one is refused outright, per ADR 0022's guarantee
/// that a declared property can never become uninitialized again).
fn check_property_access(
    object: &Expr,
    property: &MemberName,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let object_ty = check_expr(object, None, live, scope, ctx, env);
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
    // `E_UNKNOWN_MEMBER` below — nothing is diagnosed here. The actual
    // checked-throw fallback this defers to is M4 work (no IR/codegen exists
    // yet to throw from); see the crate docs' known gaps.
    match env.interner.get(object_ty).clone() {
        Ty::Shape(fields) => {
            return fields
                .iter()
                .find(|(n, _)| *n == name)
                .map_or_else(|| env.interner.mixed(), |(_, ty)| *ty);
        }
        Ty::Object => return env.interner.mixed(),
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
                if !qname.is_core()
                    && !qname.is_reserved_global_class()
                    && !is_this_receiver(object, env.src)
                {
                    report_unknown_member(object.span, &qname, &name, "property", env);
                }
                env.interner.mixed()
            }
        },
        None => env.interner.mixed(),
    }
}

/// `unset()`'s operand: refuses a declared object property (ADR 0028 § 3)
/// via [`check_property_access`], and otherwise checks the operand exactly
/// like any other expression — an array element or a local variable is
/// untouched, since that section is scoped to object properties only.
pub(crate) fn check_unset_target(
    expr: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if let ExprKind::PropertyAccess {
        object, property, ..
    } = &expr.kind
    {
        check_property_access(object, property, true, live, scope, ctx, env);
    } else {
        check_expr(expr, None, live, scope, ctx, env);
    }
}

fn report_unset_on_property(span: Span, qname: &QName, name: &str, env: &mut Env<'_>) {
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

/// Whether `ty` carries ADR 0024 § 1's `tainted` qualifier — on its own
/// (`tainted string`/`tainted bytes`) or composed with `secret`
/// (`secret tainted string`/`secret tainted bytes`, ADR 0033 § 1). The one
/// question every taint propagation/laundering rule in this module reduces
/// to.
fn is_tainted(ty: TypeId, interner: &TypeInterner) -> bool {
    matches!(
        interner.get(ty),
        Ty::TaintedString | Ty::TaintedBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// Whether `ty` carries ADR 0033 § 1's `secret` qualifier — on its own or
/// composed with `tainted`. The `secret`-axis counterpart of [`is_tainted`];
/// the two are independent bits, so a caller checking one never implies
/// anything about the other.
fn is_secret(ty: TypeId, interner: &TypeInterner) -> bool {
    matches!(
        interner.get(ty),
        Ty::SecretString | Ty::SecretBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// Whether `ty` is one of the eight `string`/`bytes`-shaped atoms `tainted`/
/// `secret` apply to — every combination of the two qualifier bits over the
/// same base. Returns `Some(true)` for a `bytes`-based atom, `Some(false)`
/// for a `string`-based one, `None` for anything else (a scalar this axis
/// pair never touches, a class, `mixed`, ...).
fn qualifiable_base(ty: TypeId, interner: &TypeInterner) -> Option<bool> {
    match interner.get(ty) {
        Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString => Some(false),
        Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes => Some(true),
        _ => None,
    }
}

/// Interns whichever of the eight `string`/`bytes`-shaped atoms `is_bytes`/
/// `tainted`/`secret` name — the one place that maps the two independent
/// qualifier bits back onto [`Ty`]'s one-atom-per-combination representation.
fn qualified_scalar(
    is_bytes: bool,
    tainted: bool,
    secret: bool,
    interner: &mut TypeInterner,
) -> TypeId {
    match (is_bytes, tainted, secret) {
        (false, false, false) => interner.string(),
        (false, true, false) => interner.tainted_string(),
        (false, false, true) => interner.secret_string(),
        (false, true, true) => interner.secret_tainted_string(),
        (true, false, false) => interner.bytes(),
        (true, true, false) => interner.tainted_bytes(),
        (true, false, true) => interner.secret_bytes(),
        (true, true, true) => interner.secret_tainted_bytes(),
    }
}

/// ADR 0024 § 2 / ADR 0033 § 2: what an `as` conversion's result carries on
/// the `tainted`/`secret` axes. A checked conversion that already throws on
/// a malformed shape — `uint`, `int`, `float`, `bool`, an enum's backing
/// type — removes both qualifiers on success for free, since none of those
/// targets carry either to begin with (ADR 0033 § 2's known, accepted gap:
/// this strips `secret` too, even though "shape-proof implies safety" only
/// ever justified it for `tainted`). Otherwise `to` is itself one of the
/// eight `string`/`bytes` atoms, and `from`'s qualifiers cross into it
/// unconditionally — including the identity-shaped `tainted string as
/// string`/`secret string as string`, which are not themselves
/// shape-proving conversions and must not silently launder; that would be
/// exactly the bypass this whole mechanism exists to close.
fn apply_qualifier_conversion_rule(
    from: TypeId,
    to: TypeId,
    interner: &mut TypeInterner,
) -> TypeId {
    let from_tainted = is_tainted(from, interner);
    let from_secret = is_secret(from, interner);
    if !from_tainted && !from_secret {
        return to;
    }
    let Some(to_is_bytes) = qualifiable_base(to, interner) else {
        return to;
    };
    let to_tainted = from_tainted || is_tainted(to, interner);
    let to_secret = from_secret || is_secret(to, interner);
    qualified_scalar(to_is_bytes, to_tainted, to_secret, interner)
}

/// ADR 0033 § 4: a `secret`-qualified value converted `as Core\Html\Markup`
/// is refused with a diagnostic naming the qualifier specifically, ahead of
/// [`reject_non_literal_markup_conversion`]'s generic "must be a literal"
/// one — escaping (this ADR's whole reason for diverging from `tainted`'s
/// auto-escape default) neutralizes injection risk, not exposure, so it is
/// the wrong tool here regardless of how the value was produced. In
/// practice a `secret` value is never a literal token to begin with (nothing
/// grants `secret` ambiently — ADR 0033 § 1 — so it only ever reaches this
/// point through a declared binding), meaning the generic literal check
/// alone would already refuse it; this exists to give that refusal its own,
/// more specific reason. Scoped to a conversion whose target actually
/// resolves to `Core\Html\Markup`, same as its sibling; the inline
/// `<?= expr ?>`/templating-helper interpolation position ADR 0033 § 4 also
/// names waits on `Core\Html` actually existing (see the crate docs' known
/// gaps).
fn reject_secret_markup_conversion(inner_ty: TypeId, to: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(to) else {
        return;
    };
    if qname.to_string() != "Core\\Html\\Markup" {
        return;
    }
    if !is_secret(inner_ty, env.interner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_MARKUP_UNSUPPORTED,
            "a `secret`-qualified value cannot be converted `as Markup`; escaping neutralizes \
             injection risk, not confidentiality, so it is refused outright rather than \
             auto-escaped",
        )
        .with_primary(span, "converted here")
        .with_help("reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`"),
    );
}

/// ADR 0024 § 5: `as Core\Html\Markup` trusts only a source-literal string —
/// a `tainted` value, or any other runtime-computed one, can never become
/// trusted markup this way, closing "compute the escape-defeating payload at
/// runtime, then cast it." Scoped to a conversion whose target actually
/// resolves to `Core\Html\Markup`; every other target is untouched. The rest
/// of § 5 (auto-escaping a non-`Markup` interpolation, `Markup + Markup`)
/// waits on `Core\Html` actually existing as a stdlib class.
fn reject_non_literal_markup_conversion(inner: &Expr, to: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(to) else {
        return;
    };
    if qname.to_string() != "Core\\Html\\Markup" {
        return;
    }
    if is_literal_string(inner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_MARKUP_REQUIRES_LITERAL,
            "only a source-literal string may be converted `as Markup`; a runtime-computed or \
             `tainted` value can never become trusted markup this way",
        )
        .with_primary(span, "converted here")
        .with_help("build markup from literal fragments, or escape via a `Core\\Html` helper"),
    );
}

/// Whether `expr` is a literal string token, unwrapping any enclosing
/// parentheses — `("text")` is exactly as trusted as `"text"` for
/// [`reject_non_literal_markup_conversion`]'s purposes.
fn is_literal_string(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Str(_) => true,
        ExprKind::Paren(inner) => is_literal_string(inner),
        _ => false,
    }
}

/// ADR 0027 § 1: `$obj(...)` is refused whenever `$obj`'s static type
/// resolves to a class — MWL has no `__invoke`, so no class ever makes `()`
/// mean anything else, regardless of what methods it declares. A `Ty::Mixed`
/// callee (nothing statically known) and an already-`Ty::Callable` one are
/// both left alone.
fn report_call_on_non_callable(callee_ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(callee_ty).clone() else {
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_NOT_CALLABLE,
            format!(
                "`{qname}` is not callable; MWL has no `__invoke` — call a named method \
                 instead, e.g. `$obj->methodName(...)`"
            ),
        )
        .with_primary(span, "called with `(...)` here"),
    );
}

/// ADR 0010 § 5: "`EnumName` → a different `EnumName`, even with the same
/// underlying type — **rejected**, even via `as`." Two enums sharing an
/// underlying type are not the same closed set, so this refuses the
/// conversion outright rather than letting [`ExprKind::Conversion`]'s
/// ordinary `lower_type` result stand unchecked; converting the same enum to
/// itself, or to/from anything that isn't `Ty::Enum` (its underlying type,
/// `mixed`, a checked-throw source) is untouched.
fn reject_enum_to_enum_conversion(from: TypeId, to: TypeId, span: Span, env: &mut Env<'_>) {
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

/// ADR 0028 § 1: every implicit string-conversion site — interpolation,
/// concatenation, `echo`/`print`, `as string`/`(string)` — accepts an object
/// only when its static type provably implements the reserved global
/// `Stringable` interface. Returns without diagnosing for any non-`Ty::Class`
/// operand (including `Ty::Enum`, `mixed`, and a scalar) and for an
/// unmodeled `Core` class, the same scoping [`object_comparison_result`] and
/// [`check_property_access`] already use.
pub(crate) fn require_stringable(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(ty).clone() else {
        return;
    };
    if qname.is_core() {
        return;
    }
    let stringable = QName::parse("Stringable");
    if !mwl_hir::implements_interface(&qname, &stringable, env.graph) {
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
    }
}

/// Reports `E_UNKNOWN_MEMBER` for a property/method access this module
/// resolved a receiver class for, but found nothing declared under `name` on
/// it or any ancestor.
fn report_unknown_member(span: Span, qname: &QName, name: &str, kind: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no {kind} named `{name}`"),
        )
        .with_primary(span, "referenced here"),
    );
}

/// ADR 0043 § 3: a `private` interface method is an internal helper, never
/// part of that interface's contract — visible only from inside its own
/// declaring interface's method bodies (a default or another private
/// method), never through an implementing class, a subinterface, or any
/// other interface. `owner` is the [`QName`] [`resolve_method`] found `sig`
/// declared on, which may differ from the receiver's own static type when
/// the method was inherited — exactly the case this check cares about.
fn check_interface_private_visibility(
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

/// Checks a call's arguments against a resolved [`MethodSig`], when one was
/// found: reports `E_ARITY_MISMATCH` for a wrong non-variadic argument count,
/// then checks each positional argument against its parameter's type the
/// same way an ordinary assignment is checked. Falls back to the old
/// "just walk nested expressions, `mixed` throughout" behaviour when no
/// signature resolved, and also when any argument is named or spread — PHP's
/// named/variadic call resolution isn't a straight positional mapping, and
/// modeling that is out of scope for this slice. Returns each positional
/// argument's own checked type, in call order — [`ExprKind::New`]'s arm reads
/// the first one back to feed [`reject_secret_throwable_message`] without a
/// second, diagnostic-duplicating pass over the same expression.
fn check_args_typed(
    args: &CallArgs,
    sig: Option<MethodSig>,
    call_span: Span,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Vec<TypeId>, Option<MethodSig>) {
    let CallArgs::List(list) = args else {
        return (Vec::new(), sig);
    };
    let Some(sig) = sig else {
        let types = list
            .iter()
            .map(|Arg { value, .. }| check_expr(value, None, live, scope, ctx, env))
            .collect();
        return (types, None);
    };
    if list.iter().any(|a| a.name.is_some() || a.spread) {
        let types = list
            .iter()
            .map(|Arg { value, .. }| check_expr(value, None, live, scope, ctx, env))
            .collect();
        return (types, Some(sig));
    }
    if !sig.variadic && list.len() != sig.params.len() {
        env.diags.report(
            Diagnostic::error(
                code::E_ARITY_MISMATCH,
                format!(
                    "expected {} argument(s), found {}",
                    sig.params.len(),
                    list.len()
                ),
            )
            .with_primary(call_span, "called here"),
        );
    }
    if sig.is_generic(env.interner) {
        return check_generic_args(list, sig, live, scope, ctx, env);
    }
    let last_param_index = sig.params.len().saturating_sub(1);
    let mut arg_types = Vec::with_capacity(list.len());
    for (i, arg) in list.iter().enumerate() {
        let expected = if sig.variadic && i >= last_param_index {
            sig.params.last().copied()
        } else {
            sig.params.get(i).copied()
        };
        let actual = check_expr(&arg.value, expected, live, scope, ctx, env);
        if sig.is_by_ref(i) {
            check_by_ref_arg(arg, actual, expected, env);
        }
        arg_types.push(actual);
    }
    (arg_types, Some(sig))
}

/// The two extra obligations an argument at a `&$x` parameter position
/// carries, beyond the assignability [`check_args_typed`] already checked for
/// every argument.
///
/// 1. **It must be a writable place.** The callee writes back through the
///    reference, so the argument has to name storage that survives the call.
///    Two shapes do: a bare local, and a compile-time-known property. Those
///    are exactly the two `mwl_ir::lower::Lowering::write_back_ref` can
///    re-point, and the same two `mwl_ir::lower::is_aliasing_read` already
///    recognises as durable storage.
/// 2. **Its type must be exactly the parameter's.** An ordinary argument may
///    widen on the way in (`int` into a `float` parameter); a by-reference one
///    may not, because the callee writes back at the *declared* type and the
///    caller's storage would then have to narrow on the way out — silently,
///    and lossily. ADR 0007 § 1's "no type ever changes by itself" leaves no
///    room for that, so the two sides must agree exactly.
///
/// Obligation 2 is only reported when the argument would otherwise have been
/// accepted: a type that is not assignable at all already produced
/// `E_TYPE_MISMATCH` at the same span, and saying it twice helps nobody.
fn check_by_ref_arg(arg: &Arg, actual: TypeId, expected: Option<TypeId>, env: &mut Env<'_>) {
    // ADR 0014 § 1 makes a hooked property's read a call and its write a
    // second one, so it has no address to hand out — and no rule for what a
    // callee writing through one would even mean. Checked before the shape
    // match so `$obj->hooked` is refused for the right reason.
    let hooked = matches!(
        env.exprs.lookup(arg.value.span),
        Some(ExprInfo::HookedProperty { .. })
    );
    if hooked {
        env.diags.report(
            Diagnostic::error(
                code::E_BY_REF_ARG_NOT_A_PLACE,
                "a property with hooks cannot be passed to a `&` parameter",
            )
            .with_primary(arg.value.span, "passed by reference here")
            .with_help(
                "reading it runs its `get` hook and writing it runs its `set` hook (ADR 0014 \
                 § 1) — read it into a local, pass that, and assign the result back",
            ),
        );
        return;
    }
    match &arg.value.kind {
        ExprKind::Variable(_) => {}
        ExprKind::PropertyAccess { nullsafe, .. } if !nullsafe => {}
        ExprKind::Index { .. } => {
            env.diags.report(
                Diagnostic::error(
                    code::E_BY_REF_ARG_NOT_A_PLACE,
                    "an array element cannot be passed to a `&` parameter yet",
                )
                .with_primary(arg.value.span, "passed by reference here")
                .with_help(
                    "ADR 0007 § 5's copy-on-write separation gives an element no stable \
                     address — read it into a local, pass that, and write it back",
                ),
            );
        }
        _ => {
            env.diags.report(
                Diagnostic::error(
                    code::E_BY_REF_ARG_NOT_A_PLACE,
                    "only a variable or a property can be passed to a `&` parameter",
                )
                .with_primary(arg.value.span, "passed by reference here")
                .with_help(
                    "the callee writes back through the reference, so this argument has to \
                     name storage that outlives the call",
                ),
            );
        }
    }
    if let Some(declared) = expected
        && declared != actual
        && is_assignable(actual, declared, env.interner, env.graph, env.signatures)
    {
        let (want, got) = (
            env.interner.describe(declared),
            env.interner.describe(actual),
        );
        env.diags.report(
            Diagnostic::error(
                code::E_BY_REF_ARG_TYPE_NOT_EXACT,
                format!("a `&` parameter declared `{want}` needs an argument of exactly that type, not `{got}`"),
            )
            .with_primary(arg.value.span, format!("this is `{got}`"))
            .with_help(
                "a by-reference argument is written back at the parameter's declared type, so \
                 widening on the way in would mean narrowing on the way out",
            ),
        );
    }
}

/// [`check_args_typed`] for a signature that mentions a type variable, which
/// today means a `Core` member and nothing else ([`crate::generics`] owns
/// why).
///
/// The ordering is the whole content: a variable's value *is* an argument's
/// type, so there is nothing to check an argument against until every
/// argument has been inferred. Each one is therefore checked with no
/// expectation first -- which is also the honest expectation for a position
/// whose declared type is still open -- then the bindings are read off, the
/// signature is rewritten concrete, and only then is each argument checked for
/// assignability against its now-known parameter type. One pass over the
/// arguments, so nothing is diagnosed twice.
fn check_generic_args(
    list: &[Arg],
    sig: MethodSig,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Vec<TypeId>, Option<MethodSig>) {
    let arg_types: Vec<TypeId> = list
        .iter()
        .map(|Arg { value, .. }| check_expr(value, None, live, scope, ctx, env))
        .collect();

    let mut bindings = crate::generics::Bindings::default();
    for (index, actual) in arg_types.iter().enumerate() {
        if let Some(declared) = sig.param_at(index) {
            crate::generics::bind(declared, *actual, env.interner, &mut bindings);
        }
    }
    let sig = sig.substituted(&bindings, env.interner);

    for (index, (arg, actual)) in list.iter().zip(&arg_types).enumerate() {
        if let Some(declared) = sig.param_at(index)
            && !is_assignable(*actual, declared, env.interner, env.graph, env.signatures)
        {
            report_mismatch(arg.value.span, declared, *actual, env);
        }
    }
    (arg_types, Some(sig))
}

/// Whether `qname` is `Throwable` or reaches it by walking its `extends`
/// chain — the same reachability question [`mwl_hir::implements_interface`]
/// already answers for `Comparable`/`Stringable`.
///
/// One root is enough because spec § 10 makes `Throwable` the only one:
/// every other exception class, seeded or user-declared, descends from it
/// through links `mwl_hir::seed_exception_tree` put in the graph.
fn is_throwable_shaped(qname: &QName, graph: &ClassGraph) -> bool {
    let root = QName::parse(mwl_hir::errors::ROOT);
    *qname == root || mwl_hir::implements_interface(qname, &root, graph)
}

/// ADR 0033 § 4: a `Throwable`-shaped class's constructor message argument
/// (its first positional argument) refuses a `secret`-qualified value —
/// closing the common real-world leak of a credential ending up in a stack
/// trace or an error page, the same "sink requires the plain type" shape ADR
/// 0024 § 4 already gives `Core\Db`'s query text. `first_arg_ty` is the
/// already-checked type [`check_args_typed`] computed for that argument, so
/// this never re-walks (and re-diagnoses) the expression itself. `Throwable`/
/// `Exception`/`Error` have no declared stdlib member table yet, so this is
/// scoped to the constructor call shape alone — "anywhere a message is later
/// composed" (e.g. through a setter) is not modeled this slice.
fn reject_secret_throwable_message(
    qname: &QName,
    first_arg_ty: Option<TypeId>,
    span: Span,
    env: &mut Env<'_>,
) {
    if !is_throwable_shaped(qname, env.graph) {
        return;
    }
    let Some(ty) = first_arg_ty else {
        return;
    };
    if !is_secret(ty, env.interner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_THROWABLE_MESSAGE,
            "a `secret`-qualified value cannot be passed as a `Throwable` message; it would \
             surface in a stack trace or an error page",
        )
        .with_primary(span, "secret value used as the message here")
        .with_help("reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`"),
    );
}

/// The binary-operator result-type table, ADR 0007 § 4, amended by ADR 0013
/// § 6 for `< <= > >= <=>` when both operands are objects. Beyond that one
/// amendment, only `int`/`uint`/`float` operands are modeled this slice —
/// anything else (`mixed`, an unresolved call result) falls back to `mixed`
/// rather than diagnosing, since no general operator-overload rule is
/// implemented yet.
fn binary_result(op: BinaryOp, lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    match op {
        // ADR 0024 § 2 / ADR 0033 § 2: concatenating a qualified operand with
        // an unqualified one poisons the result on that axis, the same
        // "poisoned" shape ADR 0007 already uses for mixed-type arithmetic —
        // `tainted` and `secret` poison independently of each other.
        BinaryOp::Concat => {
            let tainted = is_tainted(lhs, env.interner) || is_tainted(rhs, env.interner);
            let secret = is_secret(lhs, env.interner) || is_secret(rhs, env.interner);
            qualified_scalar(false, tainted, secret, env.interner)
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Pow | BinaryOp::Mod => {
            arithmetic_result(lhs, rhs, span, env)
        }
        BinaryOp::Div => division_result(lhs, rhs, span, env),
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            bitwise_result(lhs, rhs, span, env)
        }
        BinaryOp::Cmp => {
            object_comparison_result(op, lhs, rhs, span, env).unwrap_or_else(|| env.interner.int())
        }
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
            object_comparison_result(op, lhs, rhs, span, env)
                .unwrap_or_else(|| env.interner.bool_ty())
        }
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Identical
        | BinaryOp::NotIdentical
        | BinaryOp::And
        | BinaryOp::Or => env.interner.bool_ty(),
        BinaryOp::Coalesce => env.interner.make_union([lhs, rhs]),
        _ => env.interner.mixed(),
    }
}

/// ADR 0013 §§ 2-4: `< <= > >= <=>` lower to a `compareTo` call when both
/// operands are objects, so ordering them requires both sides to be the same
/// class and that class to (transitively) implement the reserved global
/// `Comparable` interface — returns `None` when either operand isn't a class
/// at all, leaving [`binary_result`]'s ordinary scalar/`mixed` fallback in
/// place untouched, since this ADR only amends ADR 0007 § 4's table with a
/// new object-operand row rather than replacing it. An enum operand
/// (`Ty::Enum`) is deliberately not treated as an object here either — ADR
/// 0010's own item (still unimplemented) is what would say whether an enum
/// can ever be `Comparable`.
fn object_comparison_result(
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
    let comparable = QName::parse("Comparable");
    if !mwl_hir::implements_interface(&lhs_q, &comparable, env.graph) {
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
    // ADR 0013 § 2: the comparison *is* a `compareTo` call, so `mwl-ir` needs
    // its resolved target the same way an ordinary `$a->compareTo($b)` does —
    // recorded under the *binary expression's* own span, since there is no
    // call node in the AST to key it by. `Comparable::compareTo` is bodiless,
    // so `has_body` is `false` and the call dispatches on the receiver's
    // runtime class, exactly like any other call to an interface method.
    if let Some((owner, sig)) = resolve_method(&lhs_q, "compareTo", env.signatures, env.graph) {
        let call = resolved_call(owner, "compareTo".to_owned(), &sig);
        env.exprs.record(span, ExprInfo::Call(call));
    }
    Some(match op {
        BinaryOp::Cmp => env.interner.int(),
        _ => env.interner.bool_ty(),
    })
}

fn report_comparable_diagnostic(span: Span, message: String, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_COMPARISON_REQUIRES_COMPARABLE, message)
            .with_primary(span, "compared here")
            .with_help("implement `Comparable`'s `compareTo(self $other): int` on the class"),
    );
}

fn arithmetic_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_enum_operand(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
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

/// ADR 0010 § 5: "No arithmetic or bitwise operator is defined on an enum
/// type directly" — `Permission::Read | Permission::Write` must be diagnosed
/// naming `as uint`/`as int` as the fix rather than silently falling through
/// to [`arithmetic_result`]/[`bitwise_result`]'s existing `_ => mixed` arm,
/// which would otherwise swallow the mistake with no diagnostic at all.
/// Returns `Some(mixed)` when either operand is `Ty::Enum` (already
/// diagnosed), `None` for every other operand pair so the caller's own table
/// runs unchanged.
fn reject_enum_operand(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> Option<TypeId> {
    let lhs_enum = matches!(env.interner.get(lhs), Ty::Enum(..));
    let rhs_enum = matches!(env.interner.get(rhs), Ty::Enum(..));
    if !lhs_enum && !rhs_enum {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ENUM_ARITHMETIC_UNSUPPORTED,
            "no arithmetic or bitwise operator is defined on an enum type directly",
        )
        .with_primary(span, "enum operand used here")
        .with_help("convert to the underlying type first: `... as int`/`... as uint`"),
    );
    Some(env.interner.mixed())
}

fn division_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_enum_operand(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
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

fn bitwise_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_enum_operand(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        (Ty::Int, Ty::Int) => env.interner.int(),
        (Ty::Uint, Ty::Uint) => env.interner.uint(),
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        _ => env.interner.mixed(),
    }
}

fn report_int_uint(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_INT_UINT_ARITHMETIC,
            "`int` and `uint` have no representable common type in arithmetic",
        )
        .with_primary(span, "mixed-signedness operand")
        .with_help("convert one side explicitly with `as int`/`as uint`"),
    );
}
