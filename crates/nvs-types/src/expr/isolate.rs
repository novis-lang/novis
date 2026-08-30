//! ADR 0006's two constructs — `spawn script … with(…)` and `await` — and what
//! each is typed as.
//!
//! Both arms still *refuse*, because nothing below this crate compiles either
//! yet: `crates/nvs-ir/src/lower/expr.rs`'s roster comment is the proof that no
//! lowering arm exists, and `docs/plan/m5.md` is the schedule. The refusal is
//! what keeps that roster true, so it stays until the lowering lands rather
//! than until the types do — and the types are now both here: `spawn script`
//! answers with the handle class and `await` answers with the shape below,
//! which is what lets a program hear about the *rest* of a line it wrote
//! rather than only about the construct itself.
//!
//! That is also what makes `await`'s operand checkable, and it is checked:
//! `await 5` is an ordinary `E_TYPE_MISMATCH` naming `Core\Script\Handle`,
//! reported by [`check_expr`] against the expected type rather than by a rule
//! of this module's own. There is nothing else in the language that produces a
//! handle, so an operand that is not one cannot have come from a spawn.
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
//! - **The handle is a registered `Core` class**, `Core\Script\Handle` —
//!   [`nvs_stdlib::script`] is the row, and the one home of why it declares no
//!   members. It is exactly a value with no readable part: § *Failure is a value, not an
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
use nvs_hir::QName;
use nvs_syntax::ast::{Expr, SpawnOption, SpawnOptionKey};
use rustc_hash::FxHashSet;

use crate::locals::LocalScope;
use crate::ty::TypeId;
use crate::{Ctx, Env};

use super::{check_expr, reject_secret_crossing};

/// `spawn script <path> with(<options>)` — ADR 0006's isolate spawn.
///
/// Its operands are still checked, because a typo in the path expression is
/// worth reporting alongside, and then the construct itself is refused:
/// refusing it where it is written is the only reading that cannot silently do
/// nothing. The answer is [`script_handle`] regardless, for [`check_await`]'s
/// reason — a refused construct with an honest type reports the mistakes on
/// the *other* lines, and the one thing a program can write with a handle is
/// the one thing that has a type to check it against.
pub(crate) fn check_spawn_script(
    path: &Expr,
    options: &[SpawnOption],
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let string = env.interner.string();
    check_expr(path, Some(string), live, scope, ctx, env);
    for opt in options {
        let expected = match opt.key {
            // The value that crosses. No expected type, because ADR 0023 § 2's
            // walk is what decides whether a given graph may cross and that is
            // a run-time question for everything a declared type does not
            // already settle.
            SpawnOptionKey::Args => None,
            SpawnOptionKey::Output => Some(string),
            SpawnOptionKey::Limits | SpawnOptionKey::Grants | SpawnOptionKey::On => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_SPAWN_OPTION_UNSUPPORTED,
                        format!(
                            "`spawn script`'s `{}:` is not enforced yet",
                            spelling(opt.key)
                        ),
                    )
                    .with_primary(opt.span, "this option would be accepted and ignored")
                    .with_help(
                        "ADR 0006 specifies all five options and this compiler enforces \
                         `args:` and `output:`. Refusing the other three is deliberate: a \
                         `grants:` narrowing that were silently dropped would hand the \
                         child the parent's authority",
                    ),
                );
                None
            }
        };
        let ty = check_expr(&opt.value, expected, live, scope, ctx, env);
        // ADR 0033 § 4's second carrier. The bullet refuses a `secret` value at
        // the graph copy "for both its callers alike", so this is the same
        // refusal `Core\Serialize::encode` reports and not a spawn-specific
        // rule — `super::quals::reject_secret_crossing` owns the sentence, and
        // only the clause naming where the copy went differs.
        if opt.key == SpawnOptionKey::Args {
            reject_secret_crossing(
                &opt.value,
                ty,
                "`spawn script`'s `args:` copies it into a child whose arena this request \
                 cannot reach into",
                env,
            );
        }
    }
    script_handle(env)
}

/// One option key as the program spells it.
fn spelling(key: SpawnOptionKey) -> &'static str {
    match key {
        SpawnOptionKey::Args => "args",
        SpawnOptionKey::Limits => "limits",
        SpawnOptionKey::Grants => "grants",
        SpawnOptionKey::Output => "output",
        SpawnOptionKey::On => "on",
    }
}

/// `await <operand>` — the prefix half of the same surface, refused for the
/// same reason and in the same shape.
///
/// Its operand is checked first, and **against [`script_handle`]**: `await
/// $handel` is an undefined variable whether or not the construct compiles,
/// and `await 5` is a value that no `spawn script` produced. That second one is
/// reported by [`check_expr`] itself, naming the expected class — this module
/// adds no rule of its own for it, because there is nothing about the mismatch
/// that a declared parameter of the same type would not already say.
///
/// The answer is [`script_result`] all the same, because the type is decided
/// even though the lowering is not: a program that reads `$result->ok` hears
/// about a `string` binding on the next line rather than only about `await`
/// itself, and the refusal is what stops the compilation either way.
pub(crate) fn check_await(
    operand: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let handle = script_handle(env);
    check_expr(operand, Some(handle), live, scope, ctx, env);
    script_result(env)
}

/// `Core\Script\Handle`, interned: the class a spawn answers with and the only
/// type an `await` accepts.
///
/// Named through [`nvs_stdlib::script::HANDLE_NAME`] rather than written out,
/// because the registry row and this are the two halves of one name.
/// [`crate::core_lib`]'s `seed` has already put the class in the very table
/// [`TypeInterner::class`](crate::ty::TypeInterner::class) reads, so nothing
/// here registers anything — a `Core` instance type is an ordinary class type
/// from the moment its row exists.
pub(crate) fn script_handle(env: &mut Env<'_>) -> TypeId {
    env.interner
        .class(QName::parse(nvs_stdlib::script::HANDLE_NAME))
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
