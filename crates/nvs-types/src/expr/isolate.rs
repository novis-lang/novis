//! ADR 0006's two constructs — `spawn script … with(…)` and `await` — and what
//! each is typed as.
//!
//! Both arms still *refuse*, because nothing below this crate compiles either
//! yet: `crates/nvs-ir/src/lower/expr.rs`'s roster comment is the proof that no
//! lowering arm exists, and `docs/plan/m5.md` is the schedule. The refusal is
//! what keeps that roster true, so it stays until the lowering lands rather
//! than until the type does — `await` already answers with the type below, and
//! `spawn script` answers `mixed` until the class its handle is has a registry
//! row (item 22).
//!
//! # The handle is a `Core` class and the result is a shape
//!
//! `$job = spawn script …;` and `$result = await $job;` are the two halves of
//! ADR 0006's § *Decision*, and they are answered by opposite mechanisms. One
//! fact decides both: **a `Core` instance has no property a program can reach**
//! — [`CoreTy::Instance`](nvs_stdlib::registry::CoreTy::Instance) is a class
//! whose members are called, and `CoreClass::slots` is a layout helper bodies
//! read by index, never source.
//!
//! - **The handle is a registered `Core` class**, `Core\Script\Handle`. It is
//!   exactly a value with no readable part: § *Failure is a value, not an
//!   exception* gives a program nothing to do with a handle but `await` it, and
//!   the property such a class cannot have is the property this one must not
//!   have. `Core\Task` beside `Core\Task\Channel<T>` is the same pairing
//!   already on disk, so `Core\Script::args()` (item 22) and this class sit
//!   together under ADR 0011 exactly as those two do.
//! - **The result is an ADR 0036 § 3 shape**, not a class. `$result->ok` is
//!   read on the next line, and a class read with `->` is the one thing this
//!   cannot be without new machinery in three crates — a property resolution
//!   against `CoreClass::slots` in this crate, a slot read in `nvs-ir`, and a
//!   per-slot type the registry has no spelling for. A shape needs none of it:
//!   `Core\Task::all` answers with one, [`ExprInfo::ShapeProperty`] records the
//!   read, and `nvs-ir` lowers that to a `SlotGet` today.
//!
//! ## What the shape is
//!
//! ```text
//! {ok: bool, value: mixed, output: string, error: ?{class: string, message: string}}
//! ```
//!
//! One field per member of `nvs_host::Completion`, which is the native half
//! of the same value and the reason the field set is this and not ADR 0006's
//! full table: `code`, `trace` and a limit breach's `error->limit` are named by
//! § *Failure is a value, not an exception* and are **item 22's**, together
//! with the top-level `return` contract that fills `value`. ADR 0018's
//! coverage data lands on the same shape later, and costs nothing structurally
//! — ADR 0036 § 3's width subtyping makes a field added to the answer
//! invisible to every call site that does not read it.
//!
//! `error` is nullable because the native half makes it `Some` exactly when
//! `ok` is false, which costs ADR 0006's own `$result->error->message` a `?->`
//! or a narrowing until the checker can read one off the `ok` test. That is the
//! honest type and the alternative is a shape whose fields are empty strings on
//! the success path, which is the state PHP's `errno`/`error_get_last` pair
//! already proves is read wrong.
//!
//! `output` is typed `string` here, and that is the one field whose type is not
//! settled: ADR 0088 §§ 3, 5 make captured output carry the *parent sink's*
//! carrier type — `Core\Html\Markup` under a request, `Cli\Text` everywhere
//! else — which is a property of the running process rather than of the
//! compilation. Item 24 owns `output: capture|inherit` and owns that question
//! with it.
//!
//! ## What it costs
//!
//! **A shape has no methods, so `$result->valueOrThrow()` cannot exist.** ADR
//! 0006's § *Alternatives rejected* names that spelling as the terse form for a
//! caller who wants the child's failure to propagate. It becomes
//! `Core\Script::valueOrThrow($result)` — a static member on the class item 22
//! introduces anyway — which is the ordinary way this language spells a verb
//! (ADR 0011), and the ADR marks its whole surface provisional pending the
//! spelling `docs/spec/` fixes at M5. Nothing else in that ADR's semantics
//! moves.
//!
//! Against that: no new checker or lowering machinery, one object of four slots
//! per awaited child either way, and a result whose fields are read with the
//! same two instructions a shape literal's already are. Simplicity of the
//! language surface is priority 4 and the machinery a class would need buys
//! nothing above it (`AGENTS.md` § *The priority ordering*).
//!
//! [`ExprInfo::ShapeProperty`]: crate::expr_table::ExprInfo::ShapeProperty

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::{Expr, SpawnOption};
use rustc_hash::FxHashSet;

use crate::locals::LocalScope;
use crate::ty::TypeId;
use crate::{Ctx, Env};

use super::check_expr;

/// `spawn script <path> with(<options>)` — ADR 0006's isolate spawn.
///
/// Its operands are still checked, because a typo in the path expression is
/// worth reporting alongside, and then the construct itself is refused:
/// refusing it where it is written is the only reading that cannot silently do
/// nothing.
pub(crate) fn check_spawn_script(
    expr: &Expr,
    path: &Expr,
    options: &[SpawnOption],
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_expr(path, None, live, scope, ctx, env);
    for opt in options {
        check_expr(&opt.value, None, live, scope, ctx, env);
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SPAWN_SCRIPT_UNLOWERED,
            "`spawn script` is not compiled yet",
        )
        .with_primary(expr.span, "no isolate is created here")
        .with_help(
            "ADR 0006's isolates arrive with `docs/plan/m5.md`, together with the \
             value-crossing copy a spawn boundary needs. There is no same-frame \
             spelling of this to fall back on",
        ),
    );
    env.interner.mixed()
}

/// `await <operand>` — the prefix half of the same surface, refused for the
/// same reason and in the same shape.
///
/// Its operand is checked first: `await $handel` is an undefined variable
/// whether or not the construct compiles. The answer is [`script_result`] all
/// the same, because the type is decided even though the lowering is not: a
/// program that reads `$result->ok` hears about a `string` binding on the next
/// line rather than only about `await` itself, and the refusal is what stops
/// the compilation either way.
pub(crate) fn check_await(
    expr: &Expr,
    operand: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_expr(operand, None, live, scope, ctx, env);
    env.diags.report(
        Diagnostic::error(code::E_AWAIT_UNLOWERED, "`await` is not compiled yet")
            .with_primary(expr.span, "nothing is awaited here")
            .with_help(
                "ADR 0006's isolates arrive with `docs/plan/m5.md`, and `await` is what \
                 turns the handle `spawn script` hands back into a `ScriptResult`. There \
                 is no synchronous spelling of this to fall back on",
            ),
    );
    script_result(env)
}

/// ADR 0006's `ScriptResult`, interned: the shape this module's doc decides on,
/// with one field per member of the native `Completion` that fills it.
pub(crate) fn script_result(env: &mut Env<'_>) -> TypeId {
    let bool_ty = env.interner.bool_ty();
    let mixed = env.interner.mixed();
    let string = env.interner.string();
    let failure = env.interner.shape(vec![
        ("class".to_owned(), string),
        ("message".to_owned(), string),
    ]);
    let null = env.interner.null();
    let maybe_failure = env.interner.make_union([failure, null]);
    env.interner.shape(vec![
        ("ok".to_owned(), bool_ty),
        ("value".to_owned(), mixed),
        ("output".to_owned(), string),
        ("error".to_owned(), maybe_failure),
    ])
}
