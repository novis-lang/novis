//! A parameter's default value — the constant a call site materializes when
//! it leaves that parameter out.
//!
//! # Why a constant, and not the expression
//!
//! PHP evaluates a parameter default *in the callee*, once per call, from an
//! arbitrary constant expression. MWL evaluates it **here, once, at signature
//! collection**, and records the resulting [`ConstArg`] in
//! [`MethodSig::defaults`](crate::signatures::MethodSig::defaults) so the
//! *caller* can emit it as an ordinary literal argument. Three things fall out
//! of that, all of them the reason to do it this way:
//!
//! * **The callee needs no second entry point.** Every compiled function keeps
//!   exactly one arity, so nothing in `mwl-ir`, `mwl-codegen` or the ADR 0002
//!   call ABI learns about defaults at all — `mwl_ir::lower::lower_call_args`
//!   pushes one more `ConstInt`/`ConstStr` and stops.
//! * **A default cannot observe anything.** It is a constant, so it cannot
//!   read a global (there are none — ADR 0008), call a function, or differ
//!   between two calls that both omitted it.
//! * **A `Core` member's default and a user-declared one are one mechanism.**
//!   `mwl_stdlib::registry` states a `Core` default as data
//!   (`mwl_stdlib::registry::Const`); [`crate::core_lib`] translates it into
//!   the same [`ConstArg`] this module evaluates a written `= expr` into, the
//!   same way it already translates `CoreTy` into [`crate::ty::Ty`].
//!
//! # What a default may be
//!
//! A literal of the parameter's own declared type — `bool`, `int`, `uint`,
//! `float`, `string` — optionally negated (`= -1`). Anything else is
//! `E_PARAM_DEFAULT_NOT_LITERAL`, naming what is accepted.
//!
//! **Known gap:** `= null` is refused along with the rest. `null` has no IR
//! constant to lower to yet (`mwl_ir::ir::InstKind` has `ConstInt` … `ConstStr`
//! and nothing for it), and a `?T` parameter defaulting to `null` is the single
//! most common shape in
//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md), so
//! this is the first thing to widen — a `ConstArg::Null` variant plus the IR
//! constant under it, not a second design. An enum case (`Mode $m =
//! Mode::Fast`) is the second: it is already a compile-time integer constant
//! (ADR 0010 § 6), it just needs `crate::enums` consulted from here.

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_syntax::ast::{Expr, ExprKind, UnaryOp};

use crate::Env;
use crate::ty::{Ty, TypeId};

/// A parameter default's already-evaluated value, in the parameter's own
/// declared type.
///
/// Deliberately one variant per `mwl_ir::ir::InstKind` constant rather than a
/// general value: this exists to be *emitted*, and a shape with no instruction
/// under it could be recorded here and then fail at lowering, which is exactly
/// the class of failure `mwl_stdlib::registry::CoreTy`'s own docs give as the
/// reason that enum is closed too.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstArg {
    /// `bool`
    Bool(bool),
    /// `int`
    Int(i64),
    /// `uint`
    Uint(u64),
    /// `float`
    Float(f64),
    /// `string`, already cooked — escapes resolved by
    /// [`crate::string_lit::cook_string_literal`], the same routine every
    /// other string literal in the program goes through, so a default is never
    /// a second escape grammar.
    Str(String),
    /// ADR 0063 R2's options bag, wholly omitted at the call site: one entry
    /// per declared option, in the bag's own declared order, each holding that
    /// option's default.
    ///
    /// The one variant with no single `mwl_ir::ir::InstKind` constant under
    /// it, and deliberately so — a bag has no runtime representation at all.
    /// `mwl_ir::lower::lower_call_args` expands it into one ordinary constant
    /// per entry, which is why the "one variant per instruction" rule above
    /// still holds one level down. Never produced by
    /// [`eval_param_default`]: user code cannot declare a bag, so this only
    /// ever comes from [`crate::core_lib`].
    Options(Vec<(String, ConstArg)>),
}

/// Evaluates a written `= expr` parameter default against the parameter's own
/// declared type `declared`, reporting `E_PARAM_DEFAULT_NOT_LITERAL` and
/// returning `None` for anything this module does not accept (see its docs).
///
/// The declared type drives the decoding rather than the literal's own shape,
/// which is what makes `uint $n = 3` and `float $f = 1` work without a
/// widening rule of their own: the digits are read once, into the type the
/// parameter actually holds.
pub(crate) fn eval_param_default(
    expr: &Expr,
    declared: TypeId,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let (negated, inner) = match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Neg,
            expr: inner,
        } => (true, &**inner),
        ExprKind::Unary {
            op: UnaryOp::Plus,
            expr: inner,
        } => (false, &**inner),
        _ => (false, expr),
    };
    let value = match (env.interner.get(declared).clone(), &inner.kind) {
        (Ty::Bool, ExprKind::Bool(b)) if !negated => Some(ConstArg::Bool(*b)),
        (Ty::Int, ExprKind::Int(span)) => int_magnitude(*span, env)
            .and_then(|m| {
                if negated {
                    negate_int(m)
                } else {
                    i64::try_from(m).ok()
                }
            })
            .map(ConstArg::Int),
        (Ty::Uint, ExprKind::Int(span)) if !negated => {
            int_magnitude(*span, env).map(ConstArg::Uint)
        }
        (Ty::Float, ExprKind::Float(span)) => {
            float_value(*span, env).map(|f| ConstArg::Float(if negated { -f } else { f }))
        }
        // An integer literal in a `float` position is the one cross-type
        // spelling accepted, for ADR 0007 § 4's reason: `int` widens to
        // `float` at any ordinary assignment, and a default is one.
        (Ty::Float, ExprKind::Int(span)) => int_magnitude(*span, env).map(|m| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the same widening ADR 0007 § 4 already allows at an \
                          ordinary int-to-float assignment, and the literal is \
                          written by hand"
            )]
            let f = m as f64;
            ConstArg::Float(if negated { -f } else { f })
        }),
        (
            Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString,
            ExprKind::Str(span),
        ) if !negated => Some(ConstArg::Str(crate::string_lit::cook_string_literal(
            env.src, *span,
        ))),
        _ => None,
    };
    if value.is_none() {
        let want = env.interner.describe(declared);
        env.diags.report(
            Diagnostic::error(
                code::E_PARAM_DEFAULT_NOT_LITERAL,
                format!("a parameter default must be a `{want}` literal"),
            )
            .with_primary(expr.span, "not a literal of the declared type")
            .with_help(
                "a default is evaluated once, at the call site that omits it — write a \
                 `bool`/`int`/`uint`/`float`/`string` literal, optionally negated",
            ),
        );
    }
    value
}

/// An integer literal's magnitude, in whatever radix it was written —
/// [`crate::expr::int_literal_digits`]'s job, reused so a default never grows
/// a second integer grammar. `None` for a magnitude that does not fit `u64`,
/// which the caller reports as "not a literal of the declared type" along with
/// every other rejection: an out-of-range default is the same authoring
/// mistake, at the same span.
fn int_magnitude(span: Span, env: &Env<'_>) -> Option<u64> {
    let (radix, digits) = crate::expr::int_literal_digits(env.src, span);
    u64::from_str_radix(&digits, radix).ok()
}

/// `-magnitude` as an `i64`, or `None` when the magnitude is too large to
/// negate — `i64::MIN`'s own magnitude included, which `i64::try_from` alone
/// would refuse.
fn negate_int(magnitude: u64) -> Option<i64> {
    if magnitude == (i64::MAX as u64) + 1 {
        return Some(i64::MIN);
    }
    i64::try_from(magnitude).ok().and_then(i64::checked_neg)
}

/// A float literal's value. PHP's own `strtod`-shaped grammar is a superset of
/// Rust's, but every spelling the *lexer* produces a `Float` token for parses
/// here; a `_` digit separator is stripped first, exactly as
/// [`crate::expr::int_literal_digits`] strips one.
fn float_value(span: Span, env: &Env<'_>) -> Option<f64> {
    crate::span_text(env.src, span)
        .replace('_', "")
        .parse()
        .ok()
}
