//! `rule:classes/unset-is-refused-on-a-property`'s `isset(...)` and its neighbour `empty(...)`: what their
//! operands may be, and why an absent array key answers `false` in one and
//! `true` in the other rather than throwing in either.
//!
//! `isset($x)` is `$x != null` and `isset($a, $b)` is the conjunction — the
//! whole semantics, with no magic method to consult (`rule:classes/unset-is-refused-on-a-property` removed
//! `__isset` along with the ambient fallback it existed to intercept).
//! `empty($x)` is `!$x`, `rule:expressions/truthy-table`'s truthy table negated, and it takes
//! exactly one operand. So the only rules this module carries are about the
//! *operand*, and there are two — one shared, one `isset`'s alone.
//!
//! **An `isset` operand names storage, or it is `E0498`.** PHP refuses the
//! identical shape at compile time, naming `null !== expression` as the
//! replacement, and the reason survives the port: an operand that is already a
//! value rather than a place has nothing `isset` can ask that `!= null` does
//! not ask more plainly. The accepted set is PHP's — a variable, a subscript,
//! a property (`?->` included), a static property, and any of those in
//! parentheses. **`empty` has no such rule**: PHP has accepted any expression
//! there since 5.5, `!$x` being defined for a value as much as for a place, so
//! `empty(Foo::bar())` is an ordinary truthy test.
//!
//! **A read that can be absent is a guarded read under either.**
//! `isset($a["k"])` over an array with no `"k"` is `false` in PHP and
//! `empty($a["k"])` is `true`, in neither case a warning and in neither a
//! throw, and `rule:types/shape-type`'s optional field asks the same question
//! of a shape — so every `Index` **and** `PropertyAccess` level of an operand
//! is marked in [`Env::coalesce_guarded`] by [`mark_guarded_places`], which is
//! also the walk `??` fills that same set with. `rule:types/absent-storage-is-never-a-zero-value`'s
//! throw is what the marking turns off; without it these two would report
//! absence by raising the very error they exist to avoid.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context.
//!

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{Expr, ExprKind};

use crate::locals::{Live, LocalScope};
use crate::ty::TypeId;
use crate::{Ctx, Env};

use super::check_expr;

/// Checks one `isset(...)` operand and records its [`ExprInfo`] entries, so
/// `nvs-ir` has something to lower a null test against.
///
/// [`ExprInfo`]: crate::expr_table::ExprInfo
pub(crate) fn check_isset_operand(
    operand: &Expr,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if !names_storage(operand) {
        report_not_a_variable(operand, env);
    }
    mark_guarded_places(operand, env);
    // Checked even when it was refused above, so a mistake inside the operand
    // arrives in the same run rather than one edit later.
    check_expr(operand, None, live, scope, ctx, env)
}

/// Checks `empty(...)`'s one operand and records its [`ExprInfo`] entries.
///
/// This is [`check_isset_operand`] without the shape check: `empty` is `!$x`
/// and PHP has accepted any expression there since 5.5, so only the
/// guarded-read half carries over. What `nvs-ir` then lowers is `rule:expressions/truthy-table`'s table plus a `Not`, which is `!` exactly.
///
/// [`ExprInfo`]: crate::expr_table::ExprInfo
pub(crate) fn check_empty_operand(
    operand: &Expr,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    mark_guarded_places(operand, env);
    check_expr(operand, None, live, scope, ctx, env)
}

/// Marks every level of `operand` that can be absent — a subscript, and a
/// property whose receiver may be a shape with an optional field — as a guarded
/// read, so absence answers `null` there instead of taking
/// `rule:types/absent-storage-is-never-a-zero-value`'s throw.
///
/// This is the whole walk behind [`Env::coalesce_guarded`], shared with `??`'s
/// own left operand ([`super::check_expr`]'s `BinaryOp::Coalesce` arm) so that
/// `isset($p->a)` and `$p->a ?? 0` cannot come to answer two different
/// questions. Called *before* the operand is checked, because the arms that
/// read the mark are inside that check — [`Env::coalesce_guarded`]'s own doc
/// comment owns the ordering, and `$a["k"]["j"]` needs every level marked for
/// the same reason `$a["k"]["j"] ?? "d"` does.
///
/// A property level is marked whatever the receiver turns out to be: this runs
/// before any type is known, and the arms that read the mark are the ones that
/// know which levels the mark means anything for.
pub(super) fn mark_guarded_places(operand: &Expr, env: &mut Env<'_>) {
    for_each_place_level(operand, |span| {
        env.coalesce_guarded.insert(span);
    });
}

/// Marks every level of a `??=` target that [`mark_guarded_places`] would mark
/// under a `??`, as a read `nvs-ir` lowers guarded and a write it does not
/// (`rule:expressions/defaulting-assignment`).
///
/// The marks go into the typed-expression table, never into
/// [`Env::coalesce_guarded`]: the read and the write share one entry per span,
/// and the entry has to stay the write's.
/// [`crate::expr_table::ExprTypeTable::is_guarded_target_read`] says why.
pub(crate) fn mark_guarded_target_reads(target: &Expr, env: &mut Env<'_>) {
    for_each_place_level(target, |span| env.exprs.record_guarded_target_read(span));
}

/// Calls `each` with the span of every subscript and property level of
/// `operand`, outermost first, looking through parentheses.
fn for_each_place_level(operand: &Expr, mut each: impl FnMut(Span)) {
    let mut level = operand;
    loop {
        match &level.kind {
            ExprKind::Paren(inner) => level = inner,
            ExprKind::Index { base, .. } => {
                each(level.span);
                level = base;
            }
            ExprKind::PropertyAccess { object, .. } => {
                each(level.span);
                level = object;
            }
            _ => break,
        }
    }
}

/// Whether `e` names a place rather than a value — PHP's own `isset` operand
/// grammar, plus the parentheses PHP also accepts (`isset(($a))` is `true`).
///
/// [`ExprKind::Error`] is in the accepted set on purpose: the parser already
/// reported whatever produced it, and a second diagnostic on the same edit is
/// noise.
fn names_storage(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Paren(inner) => names_storage(inner),
        ExprKind::Variable(_)
        | ExprKind::Index { .. }
        | ExprKind::PropertyAccess { .. }
        | ExprKind::StaticPropertyAccess { .. }
        | ExprKind::Error(_) => true,
        _ => false,
    }
}

fn report_not_a_variable(operand: &Expr, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_ISSET_NOT_A_VARIABLE,
            "`isset` asks about storage, and this operand is a value",
        )
        .with_primary(operand.span, "not a variable, element or property")
        .with_help(
            "`isset($x)` is `$x != null` (`rule:classes/unset-is-refused-on-a-property`), so write the test that way when \
             the thing being tested is an expression — `Foo::bar() != null`",
        ),
    );
}
