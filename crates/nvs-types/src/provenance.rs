//! Where a `tainted` or `secret` value came from inside its method
//! (`rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`).
//!
//! **Recorded only where the editor asks.** [`Provenance`] lives on the
//! expression table behind [`crate::expr_table::ExprTypeTable::record_provenance`],
//! which `nvs-lsp` alone calls. A compile for running, `nvs check` and `nvs
//! build` start from a table without one, so [`note`] returns at its first
//! test and records nothing.
//!
//! **A path is a list of steps, and each expression's is built from its
//! operands'.** [`note`] runs once per expression, after the expression and
//! everything under it were checked, so an operand's path is already
//! recorded when the expression that uses it is noted. A path starts where
//! the qualifier entered the method: a call that returns it ([`Step::Source`]),
//! a value read out of `mixed` ([`Step::Unchecked`]) or a declared type
//! ([`Step::Declared`]). Each assignment, concatenation, interpolation and call
//! that carried it adds a step. [`Step::Ended`] is the call or conversion that
//! removed it, and the path stays on the value after that, so a hover on the
//! laundered value still shows where it came from.
//!
//! **A variable's path is the last write the walk reached**, keyed by the
//! span of the binding's declaration, so two methods' `$q` never share one.
//! The walk is left to right and does not join branches: after an `if` that
//! assigns in one arm only, a read shows that arm's path. A captured variable
//! of an anonymous function has no declaration in its body, and its reads
//! record no path.
//!
//! The path stops at the method's boundary. A parameter declared with the
//! qualifier is where its value entered, and its declaration is the first
//! step.
//!
//! # What it costs
//!
//! Without the switch, one predicted branch per checked expression. With
//! it, one step list per qualified value, bounded by the statements of its
//! method, held while the editor keeps the analysis.

use nvs_diagnostics::Span;
use nvs_syntax::ast::{AssignOp, BinaryOp, CallArgs, Expr, ExprKind, StringPart};
use rustc_hash::FxHashMap;

use crate::expr::quals::{carries_secret, carries_tainted};
use crate::locals::LocalScope;
use crate::ty::{Ty, TypeId};
use crate::{Env, span_text, strip_sigil};

/// What one step of a path did to the value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// A call returned the qualified value, and none of its arguments carried
    /// the qualifier in: a source, or a method whose return type declares it.
    Source,
    /// Text read out of `mixed`, which is `tainted` (`rule:security/taint-propagation`).
    Unchecked,
    /// A declared type carries the qualifier: a parameter, a local, a
    /// property. The step's span is the declaration where there is one.
    Declared,
    /// The value was assigned to a variable. The step's span is the target.
    Assigned,
    /// The value was one operand of `.` or `.=`.
    Concatenated,
    /// The value was a hole of an interpolated string.
    Interpolated,
    /// A call took the value and returned a result that still carries it.
    Carried,
    /// A call or a conversion took the value and returned a result without
    /// the qualifier: the launderer (`rule:security/launderers-are-sink-named`).
    Ended,
}

/// One step of a path, and the qualifiers the value carries after it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TaintStep {
    /// What the step did.
    pub step: Step,
    /// Where it is written.
    pub span: Span,
    /// Whether the value is `tainted` after this step.
    pub tainted: bool,
    /// Whether the value is `secret` after this step.
    pub secret: bool,
}

impl TaintStep {
    /// Whether the value still carries a qualifier after this step.
    #[must_use]
    pub fn qualified(&self) -> bool {
        self.tainted || self.secret
    }
}

/// Every path the checker recorded, by the span of the expression it ends at.
#[derive(Debug, Default)]
pub struct Provenance {
    paths: FxHashMap<Span, Vec<TaintStep>>,
    bindings: FxHashMap<Span, Vec<TaintStep>>,
}

impl Provenance {
    /// The path recorded for the expression at `span`.
    #[must_use]
    pub fn path(&self, span: Span) -> Option<&[TaintStep]> {
        self.paths.get(&span).map(Vec::as_slice)
    }

    /// Every recorded path, in no particular order.
    pub fn paths(&self) -> impl Iterator<Item = (Span, &[TaintStep])> {
        self.paths
            .iter()
            .map(|(span, path)| (*span, path.as_slice()))
    }

    /// The path at `span` when its value still carries a qualifier.
    fn live(&self, span: Span) -> Option<&Vec<TaintStep>> {
        self.paths
            .get(&span)
            .filter(|path| path.last().is_some_and(TaintStep::qualified))
    }
}

/// Records the path of `expr`, whose type is `ty`, out of its operands' paths.
/// Returns at once where the table records no provenance.
pub(crate) fn note(expr: &Expr, ty: TypeId, scope: &LocalScope, env: &mut Env<'_>) {
    if !env.exprs.records_provenance() {
        return;
    }
    let tainted = carries_tainted(ty, env.interner);
    let secret = carries_secret(ty, env.interner);
    let ends_nothing = matches!(env.interner.get(ty), Ty::Void | Ty::Never | Ty::Null);
    let src = env.src;
    let Some(prov) = env.exprs.provenance_mut() else {
        return;
    };
    let at = |step: Step, span: Span| TaintStep {
        step,
        span,
        tainted,
        secret,
    };
    let then = |path: &Vec<TaintStep>, step: TaintStep| {
        let mut path = path.clone();
        path.push(step);
        path
    };
    let qualified = tainted || secret;
    let path = match &expr.kind {
        ExprKind::Variable(name) => {
            let name = strip_sigil(span_text(src, *name));
            read(prov, scope, name, expr.span, at, qualified, env.interner)
        }
        ExprKind::Paren(inner) => prov.paths.get(&inner.span).cloned(),
        ExprKind::Ternary {
            then: arm, else_, ..
        } => arm
            .as_deref()
            .and_then(|arm| prov.live(arm.span))
            .or_else(|| prov.live(else_.span))
            .cloned(),
        ExprKind::Binary {
            op: BinaryOp::Coalesce,
            lhs,
            rhs,
        } => prov.live(lhs.span).or_else(|| prov.live(rhs.span)).cloned(),
        ExprKind::Binary {
            op: BinaryOp::Concat,
            lhs,
            rhs,
        } if qualified => Some(match prov.live(lhs.span).or_else(|| prov.live(rhs.span)) {
            Some(path) => then(path, at(Step::Concatenated, expr.span)),
            None => vec![at(Step::Unchecked, expr.span)],
        }),
        ExprKind::Interpolated(parts) if qualified => {
            let carried = parts.iter().find_map(|part| match part {
                StringPart::Expr(hole) => prov.live(hole.span),
                StringPart::Text(_) => None,
            });
            Some(match carried {
                Some(path) => then(path, at(Step::Interpolated, expr.span)),
                None => vec![at(Step::Unchecked, expr.span)],
            })
        }
        ExprKind::Conversion { expr: inner, .. } => match (prov.live(inner.span), qualified) {
            (Some(path), true) => Some(path.clone()),
            (Some(path), false) => Some(then(path, at(Step::Ended, expr.span))),
            (None, true) => Some(vec![at(Step::Unchecked, expr.span)]),
            (None, false) => None,
        },
        ExprKind::Call { args, .. }
        | ExprKind::MethodCall { args, .. }
        | ExprKind::StaticCall { args, .. } => {
            let carried = match args {
                CallArgs::List(args) => args.iter().find_map(|arg| prov.live(arg.value.span)),
                CallArgs::MethodRef => None,
            };
            match (carried, qualified) {
                (Some(path), true) => Some(then(path, at(Step::Carried, expr.span))),
                (None, true) => Some(vec![at(Step::Source, expr.span)]),
                (Some(path), false) if !ends_nothing => {
                    Some(then(path, at(Step::Ended, expr.span)))
                }
                _ => None,
            }
        }
        ExprKind::Index { base, .. } => prov
            .live(base.span)
            .cloned()
            .or_else(|| qualified.then(|| vec![at(Step::Declared, expr.span)])),
        ExprKind::PropertyAccess { .. } | ExprKind::StaticPropertyAccess { .. } if qualified => {
            Some(vec![at(Step::Declared, expr.span)])
        }
        ExprKind::Assign {
            op, target, value, ..
        } => assign(prov, scope, *op, target, value.span, src, at, qualified),
        _ => None,
    };
    if let Some(path) = path {
        prov.paths.insert(expr.span, path);
    }
}

/// The path of a read of the variable `name` at `span`: the last write the
/// walk reached, or the declaration where the binding's own type carries
/// the qualifier, or `mixed` where a test narrowed the read to text.
fn read(
    prov: &Provenance,
    scope: &LocalScope,
    name: &str,
    span: Span,
    at: impl Fn(Step, Span) -> TaintStep,
    qualified: bool,
    interner: &crate::ty::TypeInterner,
) -> Option<Vec<TaintStep>> {
    let (declared_span, declared_ty) = scope.binding(name)?;
    if let Some(path) = prov.bindings.get(&declared_span) {
        return Some(path.clone());
    }
    if !qualified {
        return None;
    }
    let declared = carries_tainted(declared_ty, interner) || carries_secret(declared_ty, interner);
    Some(vec![if declared {
        at(Step::Declared, declared_span)
    } else {
        at(Step::Unchecked, span)
    }])
}

/// Records a write of the value at `value` into the variable `target`, and
/// returns the path of the assignment expression.
#[expect(
    clippy::too_many_arguments,
    reason = "the one write every assignment shape funnels through"
)]
fn assign(
    prov: &mut Provenance,
    scope: &LocalScope,
    op: AssignOp,
    target: &Expr,
    value: Span,
    src: &nvs_diagnostics::SourceFile,
    at: impl Fn(Step, Span) -> TaintStep,
    qualified: bool,
) -> Option<Vec<TaintStep>> {
    let ExprKind::Variable(name) = &target.kind else {
        return None;
    };
    let (key, _) = scope.binding(strip_sigil(span_text(src, *name)))?;
    let path = match op {
        AssignOp::Assign => prov.paths.get(&value).map(|path| {
            let last = path.last().copied();
            let mut path = path.clone();
            path.push(TaintStep {
                step: Step::Assigned,
                span: target.span,
                tainted: last.is_some_and(|step| step.tainted),
                secret: last.is_some_and(|step| step.secret),
            });
            path
        }),
        AssignOp::ConcatAssign | AssignOp::CoalesceConcatAssign if qualified => {
            let old = prov
                .bindings
                .get(&key)
                .filter(|path| path.last().is_some_and(TaintStep::qualified));
            Some(match old.or_else(|| prov.live(value)) {
                Some(path) => {
                    let mut path = path.clone();
                    path.push(at(Step::Concatenated, target.span));
                    path
                }
                None => vec![at(Step::Unchecked, target.span)],
            })
        }
        _ => None,
    };
    match &path {
        Some(path) => {
            prov.bindings.insert(key, path.clone());
            prov.paths.insert(target.span, path.clone());
        }
        None => {
            prov.bindings.remove(&key);
        }
    }
    path
}

/// Records the initializer of a local declared at `name`, as an assignment.
pub(crate) fn declared(name: Span, value: Span, scope: &LocalScope, env: &mut Env<'_>) {
    let binding = strip_sigil(span_text(env.src, name));
    let Some((key, _)) = scope.binding(binding) else {
        return;
    };
    let Some(prov) = env.exprs.provenance_mut() else {
        return;
    };
    match prov.paths.get(&value) {
        Some(path) => {
            let last = path.last().copied();
            let mut path = path.clone();
            path.push(TaintStep {
                step: Step::Assigned,
                span: name,
                tainted: last.is_some_and(|step| step.tainted),
                secret: last.is_some_and(|step| step.secret),
            });
            prov.paths.insert(name, path.clone());
            prov.bindings.insert(key, path);
        }
        None => {
            prov.bindings.remove(&key);
        }
    }
}
