//! A zone written as text (`rule:core-classes/a-time-class-builds-its-own-values`
//! § *A zone written as text*).
//!
//! A string literal at a parameter typed `Core\Time\Zone` is that zone. It is
//! checked here, against the names `nvs_stdlib::time::written_zone` accepts,
//! and recorded under the literal's span with the id the zone is stored under.
//! `nvs-ir` lowers a recorded literal to one `InstKind::ConstZone`, an immortal
//! instance in the unit's data section, so a written zone costs nothing per
//! use. `Zone::of` with a written name and `Zone::fixed` with a written
//! duration are recorded the same way, under the call's span, and `Zone::UTC`
//! folds in the lowering, where it is a built constant.
//!
//! Only a literal: a `string` variable at a `Zone` parameter stays a type
//! error, so text that arrives at run time becomes a zone through `Zone::of`
//! alone, which is the one place that throws. An unknown name is `E0769` at
//! the literal, the code every other literal read while compiling gets, with
//! the nearest names as help.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, Expr, ExprKind};

use crate::ty::{Ty, TypeId};
use crate::{Env, span_text};

/// The `Zone` type where `ty`, a parameter's declared type, is `Zone` or
/// `?Zone`, and `None` for every other type.
pub(crate) fn zone_parameter(ty: TypeId, env: &mut Env<'_>) -> Option<TypeId> {
    let inner = env.interner.without_null(ty);
    match env.interner.get(inner) {
        Ty::Class(qname, args) if args.is_empty() && is_zone(qname) => Some(inner),
        _ => None,
    }
}

fn is_zone(qname: &QName) -> bool {
    qname.to_string() == nvs_stdlib::time::ZONE_NAME
}

/// Checks the string literal at `literal`, which fills a `Zone` parameter,
/// and records the id it names. An unknown name is one diagnostic at the
/// literal.
pub(crate) fn check_written(literal: Span, env: &mut Env<'_>) {
    let text = nvs_syntax::string_lit::cook_string_literal(env.src, literal);
    match nvs_stdlib::time::written_zone(&text) {
        Ok(id) => env.exprs.record_written_zone(literal, id),
        Err(why) => report_unknown(literal, &text, &why, env),
    }
}

/// Whether `owner::member` is a `Zone` member [`fold_builder`] reads.
pub(crate) fn is_builder(owner: &QName, member: &str) -> bool {
    is_zone(owner) && matches!(member, "of" | "fixed")
}

/// Records the id `Zone::of('…')` or `Zone::fixed(<duration>)` builds, under
/// the call's span, where its argument is written in the source. A name
/// `Zone::of` would throw on is reported at the literal. A written offset
/// `Zone::fixed` would throw on is left to the call, which throws.
pub(crate) fn fold_builder(call: &Expr, member: &str, args: &CallArgs, env: &mut Env<'_>) {
    let CallArgs::List(list) = args else {
        return;
    };
    let [arg] = list.as_slice() else {
        return;
    };
    if arg.spread {
        return;
    }
    match (member, &arg.value.unparenthesized().kind) {
        ("of", ExprKind::Str(literal)) => {
            let text = nvs_syntax::string_lit::cook_string_literal(env.src, *literal);
            // `Zone::of` takes a name, and `Zone::fixed` takes an offset.
            let read = if text.starts_with('+') || text.starts_with('-') {
                Err(format!(
                    "`{}` is an offset, and `Core\\Time\\Zone::of` needs a time zone name",
                    nvs_runtime::quoted(&text)
                ))
            } else {
                nvs_stdlib::time::written_zone(&text)
            };
            match read {
                Ok(id) => env.exprs.record_written_zone(call.span, id),
                Err(why) => report_unknown(*literal, &text, &why, env),
            }
        }
        ("fixed", ExprKind::Duration(span)) => {
            let Ok(nanos) = nvs_syntax::duration::parse(span_text(env.src, *span)) else {
                return;
            };
            if let Some(id) = nvs_stdlib::time::fixed_zone_id(nanos) {
                env.exprs.record_written_zone(call.span, id);
            }
        }
        _ => {}
    }
}

/// `E0769` at a literal that names no zone, with the nearest names as help.
fn report_unknown(literal: Span, text: &str, why: &str, env: &mut Env<'_>) {
    let nearest = nearest_names(text);
    let help = if nearest.is_empty() {
        "A time zone is written as its name, such as `Europe/Berlin` or `UTC`, or as an \
         offset, such as `+02:00`."
            .to_owned()
    } else {
        let names: Vec<String> = nearest.iter().map(|name| format!("`{name}`")).collect();
        format!("Time zone names close to this text: {}.", names.join(", "))
    };
    env.diags.report(
        Diagnostic::error(
            code::E_INTRINSIC_LITERAL_MALFORMED,
            format!("this text is not valid for this function: {why}"),
        )
        .with_primary(
            literal,
            "a time zone written in quotes is checked while compiling",
        )
        .with_help(help),
    );
}

/// Up to three zone names within a few edits of `text`, case ignored,
/// nearest first. Read only on a failed compile.
fn nearest_names(text: &str) -> Vec<&'static str> {
    let wanted = text.trim().to_ascii_lowercase();
    if wanted.is_empty() || wanted.starts_with('+') || wanted.starts_with('-') {
        return Vec::new();
    }
    let bound = (wanted.len() / 3).clamp(1, 4);
    let mut near: Vec<(usize, &'static str)> = nvs_stdlib::time::zone_names()
        .iter()
        .filter_map(|name| {
            let distance = nvs_hir::members::edit_distance(&wanted, &name.to_ascii_lowercase());
            (distance <= bound).then_some((distance, name.as_str()))
        })
        .collect();
    near.sort_unstable();
    near.into_iter().take(3).map(|(_, name)| name).collect()
}
