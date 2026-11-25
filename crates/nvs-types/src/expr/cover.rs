//! `rule:expressions/a-match-over-an-enum-covers-every-case`: a `match` whose subject is an enum
//! `E`, or `?E`, and that has no `default` arm names every case of `E`, and `null` too for `?E`.
//!
//! A statically typed `E` is always one of its cases, because every write into it is checked
//! (`rule:enums/representation`). So a `match` that names every case cannot run out of arms, and
//! one that misses a case is the same mistake as a missing `return`: the checker says so where the
//! `match` is written. [`check_match_covers`] reports the missing cases, and records
//! [`ExprInfo::MatchCoversEveryCase`] for a `match` that names them all, so `nvs-ir` lowers no
//! fallback throw for it (`rule:programs/less-work-is-taken`).
//!
//! An arm condition covers a case when it is the case itself ([`ExprInfo::EnumCase`]) or a class
//! constant that folds to a case of `E` ([`ExprInfo::ClassConst`]). Any other condition covers
//! nothing. A subject of `mixed`, of a union with anything but `null`, or of `bool` is not checked.
//!
//! Cost: check time only, one set of case names per `match` over an enum.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::MatchArm;
use rustc_hash::FxHashSet;

use crate::Env;
use crate::defaults::ConstArg;
use crate::enums::EnumValue;
use crate::expr_table::ExprInfo;
use crate::ty::{Ty, TypeId};

/// The span the verdict of [`check_match_covers`] is recorded at: the `match` keyword alone.
/// No other expression starts there with that length, so no other entry replaces it.
#[must_use]
pub fn match_keyword_span(match_span: Span) -> Span {
    Span::new(
        match_span.file,
        match_span.start,
        match_span.start.saturating_add(5).min(match_span.end),
    )
}

/// The enum a `match` subject of type `subject_ty` must cover, and whether `null` must be
/// covered too. `None` for every subject the rule leaves alone.
fn covered_enum(subject_ty: TypeId, env: &Env<'_>) -> Option<(QName, bool)> {
    match env.interner.get(subject_ty) {
        Ty::Enum(qname, _) => Some((qname.clone(), false)),
        Ty::Union(members) => {
            let mut found = None;
            let mut nullable = false;
            for &member in members {
                match env.interner.get(member) {
                    Ty::Null => nullable = true,
                    Ty::Enum(qname, _) if found.is_none() => found = Some(qname.clone()),
                    _ => return None,
                }
            }
            found.filter(|_| nullable).map(|qname| (qname, true))
        }
        _ => None,
    }
}

/// The case of `enum_` that the arm condition at `span` names, if it names one.
fn covered_case(span: Span, enum_: &QName, env: &Env<'_>) -> Option<String> {
    let value = match env.exprs.lookup(span)? {
        ExprInfo::EnumCase {
            enum_: named, case, ..
        } => return (named == enum_).then(|| case.clone()),
        ExprInfo::ClassConst {
            value,
            enum_backing: Some(_),
            ..
        } => match value {
            ConstArg::EnumInt(v) | ConstArg::Int(v) => EnumValue::Int(*v),
            ConstArg::EnumUint(v) | ConstArg::Uint(v) => EnumValue::Uint(*v),
            _ => return None,
        },
        _ => return None,
    };
    env.enums
        .get(enum_)?
        .cases
        .iter()
        .find(|(_, v)| **v == value)
        .map(|(name, _)| name.clone())
}

/// Checks that a `match` over an enum with no `default` arm names every case, as the module doc
/// says. `conditions` is each arm condition's span and type, in source order. Reports
/// `E_MATCH_MISSES_A_CASE` at the `match` keyword, naming every missing case, or records
/// [`ExprInfo::MatchCoversEveryCase`] when nothing is missing.
pub(crate) fn check_match_covers(
    match_span: Span,
    subject_ty: TypeId,
    arms: &[MatchArm],
    conditions: &[(Span, TypeId)],
    env: &mut Env<'_>,
) {
    if arms.iter().any(|a| a.conditions.is_none()) {
        return;
    }
    let Some((enum_, nullable)) = covered_enum(subject_ty, env) else {
        return;
    };
    let Some(info) = env.enums.get(&enum_) else {
        return;
    };
    let mut cases: Vec<(EnumValue, String)> = info
        .cases
        .iter()
        .map(|(name, value)| (*value, name.clone()))
        .collect();
    let mut named: FxHashSet<String> = FxHashSet::default();
    let mut null_named = false;
    for &(span, ty) in conditions {
        if matches!(env.interner.get(ty), Ty::Null) {
            null_named = true;
        } else if let Some(case) = covered_case(span, &enum_, env) {
            named.insert(case);
        }
    }
    // The table keeps no declaration order, so the missing cases are listed by value, which is
    // the declaration order for every enum that counts its cases on.
    cases.sort_by_key(|(value, _)| match value {
        EnumValue::Int(v) => i128::from(*v),
        EnumValue::Uint(v) => i128::from(*v),
    });
    let short = enum_.to_string();
    let short = short.rsplit('\\').next().unwrap_or(&short).to_owned();
    let mut missing: Vec<String> = cases
        .iter()
        .filter(|(_, name)| !named.contains(name))
        .map(|(_, name)| format!("`{short}::{name}`"))
        .collect();
    if nullable && !null_named {
        missing.push("`null`".to_owned());
    }
    let keyword = match_keyword_span(match_span);
    if missing.is_empty() {
        env.exprs.record(keyword, ExprInfo::MatchCoversEveryCase);
        return;
    }
    let list = join_with_and(&missing);
    env.diags.report(
        Diagnostic::error(
            code::E_MATCH_MISSES_A_CASE,
            format!("this `match` does not handle {list}"),
        )
        .with_primary(keyword, "a case of the enum has no arm here")
        .with_help("Add an arm for each missing case, or add a `default` arm."),
    );
}

/// `a`, `a and b`, `a, b and c`.
fn join_with_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}
