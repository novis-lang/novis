//! Operator mixes whose grouping a reader misreads, and the parentheses that
//! settle each one (`rule:expressions/misread-grouping-warns`).
//!
//! Two shapes warn with [`code::W_MISREAD_GROUPING`], and the list is closed:
//!
//! - a `??` whose right operand is an arithmetic or `.` expression written
//!   without parentheses — `$n ?? 0 + 10` is `$n ?? (0 + 10)`;
//! - a `?:`, long or short, whose condition is an arithmetic, `.` or `??`
//!   expression written without parentheses — `"n=" . $ok ? "x" : "y"` tests
//!   `"n=" . $ok`.
//!
//! `&&` beside `||`, `-2 ** 2` and nested ternaries are left alone, and a
//! parenthesized operand never warns: [`ExprKind::Paren`] is a node of its own,
//! so a written `(…)` is never a bare binary expression here. Every warned
//! shape is valid code, which is why the list stays closed — a warning on valid
//! code earns its noise only where the misreading is common.
//!
//! Each warning carries two edits. The first puts parentheses around what the
//! code does now, which changes nothing, so a fix-all may apply it
//! ([`Diagnostic::with_fix`]). The second is the reading the author most likely
//! meant, which changes what the code does, so only a person may choose it
//! ([`Diagnostic::with_alternative_fix`]). "Most likely" is mechanical: the
//! `??` or `?` takes the operand nearest to it, through the operators this
//! warning covers, as a reader who misjudged the precedence expects.
//!
//! The same module gives `E0105` its help when the target of an `=` is a
//! binary expression whose rightmost operand can be assigned to:
//! `$ok && $row = $next` is `($ok && $row) = $next` in Novis, where PHP reads
//! `$ok && ($row = $next)`. The parse is then recovered as the reading PHP
//! gives it, so the checker does not report a second error about a target that
//! was never meant.
//!
//! Raised by the parser because the shapes are purely syntactic and the parser
//! is the one place that sees each written operator once, with its source text
//! at hand for the edits.
//!
//! # What it spends
//!
//! One `match` per `??`, per `?:` and per invalid assignment target, at compile
//! time. A warned shape formats its edits; nothing else allocates, and the
//! compiled program is unchanged.

use super::*;

/// The operators a `??` default or a `?:` condition may be misread around:
/// the arithmetic operators and `.`.
fn is_arithmetic_or_concat(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow
            | BinaryOp::Concat
    )
}

/// How the operators [`is_arithmetic_or_concat`] admits, and `??`, are
/// written.
fn spelling(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Pow => "**",
        BinaryOp::Concat => ".",
        _ => "??",
    }
}

/// The operator of `e` when `e` is a binary expression written without
/// parentheses.
fn bare_operator(e: &Expr) -> Option<BinaryOp> {
    match &e.kind {
        ExprKind::Binary { op, .. } => Some(*op),
        _ => None,
    }
}

/// The text of `span` for an edit's title, or `None` when it is too long or
/// spans lines, in which case the title names the edit alone.
fn shown(text: &str) -> Option<&str> {
    (text.len() <= 60 && !text.contains('\n')).then_some(text)
}

/// `title`, followed by the code it writes when that code is short enough to
/// read in a menu.
fn titled(title: &str, code: &str) -> String {
    match shown(code) {
        Some(code) => format!("{title}: `{code}`"),
        None => title.to_owned(),
    }
}

impl Parser<'_, '_> {
    /// The text between two byte offsets of the file being parsed.
    fn text_between(&self, start: BytePos, end: BytePos) -> Option<&str> {
        let file = self.file;
        file.span_text(Span::new(file.id(), start, end))
    }

    /// Warns when the right operand of `lhs ?? rhs` is a bare arithmetic or
    /// `.` expression.
    pub(super) fn warn_misread_coalesce(&mut self, lhs: &Expr, rhs: &Expr) {
        let Some(op) = bare_operator(rhs).filter(|op| is_arithmetic_or_concat(*op)) else {
            return;
        };
        let whole = lhs.span.to(rhs.span);
        // The operand nearest the `??`: down the left of every operator this
        // warning covers, so `$n ?? $a * 2 + 1` reads as `($n ?? $a) * 2 + 1`.
        let mut nearest = rhs;
        while let ExprKind::Binary { op, lhs, .. } = &nearest.kind
            && is_arithmetic_or_concat(*op)
        {
            nearest = lhs;
        }
        let mut warning = Diagnostic::warning(
            code::W_MISREAD_GROUPING,
            format!(
                "`??` is applied last, so its default is the whole `{}` expression",
                spelling(op)
            ),
        )
        .with_primary(rhs.span, "this whole expression is the default")
        .with_help("add parentheses to show the order you mean");
        if let (Some(left), Some(right), Some(head), Some(tail)) = (
            self.text_between(whole.start, rhs.span.start),
            self.text_between(rhs.span.start, rhs.span.end),
            self.text_between(whole.start, nearest.span.end),
            self.text_between(nearest.span.end, whole.end),
        ) {
            let kept = format!("{left}({right})");
            let likely = format!("({head}){tail}");
            warning = warning
                .with_fix(
                    rhs.span,
                    format!("({right})"),
                    titled("keep the current meaning", &kept),
                )
                .with_alternative_fix(whole, likely.clone(), titled("apply `??` first", &likely));
        }
        self.diags.report(warning);
    }

    /// Warns when the condition of a `?:` whose arms end at `end` is a bare
    /// arithmetic, `.` or `??` expression. `short` is the `?:` form with no
    /// middle arm.
    pub(super) fn warn_misread_ternary(&mut self, cond: &Expr, end: Span, short: bool) {
        let warned = |op: BinaryOp| is_arithmetic_or_concat(op) || op == BinaryOp::Coalesce;
        let Some(op) = bare_operator(cond).filter(|op| warned(*op)) else {
            return;
        };
        let whole = cond.span.to(end);
        // The operand nearest the `?`: down the right of every operator this
        // warning covers, so `"n=" . $ok ? …` reads as `"n=" . ($ok ? …)`.
        let mut nearest = cond;
        while let ExprKind::Binary { op, rhs, .. } = &nearest.kind
            && warned(*op)
        {
            nearest = rhs;
        }
        let question = if short { "?:" } else { "?" };
        let mut warning = Diagnostic::warning(
            code::W_MISREAD_GROUPING,
            format!(
                "the `{question}` tests the whole `{}` expression",
                spelling(op)
            ),
        )
        .with_primary(cond.span, "this whole expression is the condition")
        .with_help("add parentheses to show the order you mean");
        if let (Some(condition), Some(arms), Some(head), Some(tail), Some(operand)) = (
            self.text_between(cond.span.start, cond.span.end),
            self.text_between(cond.span.end, whole.end),
            self.text_between(whole.start, nearest.span.start),
            self.text_between(nearest.span.start, whole.end),
            self.text_between(nearest.span.start, nearest.span.end),
        ) {
            let kept = format!("({condition}){arms}");
            let likely = format!("{head}({tail})");
            let test_only = format!("test only `{operand}`");
            warning = warning
                .with_fix(
                    cond.span,
                    format!("({condition})"),
                    titled("keep the current meaning", &kept),
                )
                .with_alternative_fix(whole, likely.clone(), titled(&test_only, &likely));
        }
        self.diags.report(warning);
    }

    /// Whether `target` is a bare binary expression whose rightmost operand
    /// can be assigned to — the shape PHP reads as an assignment to that
    /// operand alone.
    pub(super) fn misread_assignment_target(target: &Expr) -> bool {
        if bare_operator(target).is_none() {
            return false;
        }
        let mut rightmost = target;
        while let ExprKind::Binary { rhs, .. } = &rightmost.kind {
            rightmost = rhs;
        }
        is_assignable(rightmost) && !matches!(rightmost.kind, ExprKind::Error(_))
    }

    /// Reports `E0105` for a target [`Self::misread_assignment_target`]
    /// accepted, with the parentheses PHP's reading needs, and answers the
    /// tree that reading gives: the assignment moved onto the rightmost
    /// operand.
    pub(super) fn regroup_misread_assignment(
        &mut self,
        target: Expr,
        op: AssignOp,
        value: Expr,
        by_ref: bool,
    ) -> Expr {
        let mut rightmost = &target;
        while let ExprKind::Binary { rhs, .. } = &rightmost.kind {
            rightmost = rhs;
        }
        let whole = target.span.to(value.span);
        let mut error = Diagnostic::error(
            code::E_INVALID_ASSIGN_TARGET,
            "this expression cannot be assigned to",
        )
        .with_primary(target.span, "not a valid assignment target")
        .with_help(
            "put parentheses around the assignment, as in `$a && ($b = 1)`. PHP adds them for \
             you, and Novis does not",
        );
        if let (Some(head), Some(tail)) = (
            self.text_between(whole.start, rightmost.span.start),
            self.text_between(rightmost.span.start, whole.end),
        ) {
            let regrouped = format!("{head}({tail})");
            error = error.with_unsafe_fix(
                whole,
                regrouped.clone(),
                titled("put parentheses around the assignment", &regrouped),
            );
        }
        self.diags.report(error);
        graft_assignment(target, op, value, by_ref)
    }
}

/// `target` with its rightmost operand replaced by `operand op= value`, every
/// binary node on the way widened to end where `value` does.
fn graft_assignment(target: Expr, op: AssignOp, value: Expr, by_ref: bool) -> Expr {
    match target.kind {
        ExprKind::Binary {
            op: binary,
            lhs,
            rhs,
        } => {
            let rhs = graft_assignment(*rhs, op, value, by_ref);
            Expr {
                span: lhs.span.to(rhs.span),
                kind: ExprKind::Binary {
                    op: binary,
                    lhs,
                    rhs: Box::new(rhs),
                },
            }
        }
        kind => {
            let operand = Expr {
                span: target.span,
                kind,
            };
            Expr {
                span: operand.span.to(value.span),
                kind: ExprKind::Assign {
                    op,
                    target: Box::new(operand),
                    value: Box::new(value),
                    by_ref,
                },
            }
        }
    }
}
