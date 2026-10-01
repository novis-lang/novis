//! `rule:security/isolate-shares-nothing`'s two constructs — `spawn script … with(…)` and `await` — and what
//! each is typed as.
//!
//! Both arms compile, and so do both entry forms: `nvs_ir::lower`'s
//! `lower_spawn_script` and `lower_await` are the two `CoreCall`s they become.
//! All five of `rule:security/isolate-shares-nothing`'s options are checked
//! here, and what refuses is named where it is reported — a placement that is
//! neither `"worker"` nor `"here"` (`E0818`), a `limits:` key that is not a
//! sub-cap a program may narrow (`E0454`), and an operand that is neither a
//! path nor a static method (`E0802`). [`check_entry`] owns the operand rule
//! itself, and the one thing it does not yet ask; [`check_spawn_script`] owns
//! what each option is typed as.
//!
//! **The operand rule has a second site and no second implementation.** ADR
//! 0083 § 2 opens a persistent connection with "0006's operand", so a `Core`
//! row that marks a parameter
//! [`CoreTy::Entry`](nvs_stdlib::registry::CoreTy::Entry) reaches
//! [`entry_operand`] through [`check_core_isolate_call`] and gets the same
//! three outcomes with the construct's own name in them — as does `rule:security/secret-sinks-refuse`
//! 's refusal over what crosses beside it. That is why this module is the
//! home of two rules whose second caller is a member call rather than a
//! construct.
//!
//! `spawn script` answers with the handle class and `await` answers with the
//! shape below, which is what lets a program hear about the *rest* of a line it
//! wrote rather than only about the construct itself.
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
//! `rule:security/isolate-shares-nothing`'s § *Decision*, and they are answered by opposite mechanisms. One
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
//!   together under `rule:classes/no-free-functions-or-constants` exactly as those two do.
//! - **The result is an `rule:types/shape-type` shape**, not a class. `$result->ok` is
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
//! of the same value and the reason the field set is this and not `rule:security/isolate-shares-nothing`'s
//! full table: `code`, `trace` and a limit breach's `error->limit` are named by
//! § *Failure is a value, not an exception* and are **item 22's**, together
//! with the top-level `return` contract that fills `value`. `rule:testing/debug-probes`'s
//! coverage data lands on the same shape later, and costs nothing structurally
//! — `rule:types/shape-type`'s width subtyping makes a field added to the answer
//! invisible to every call site that does not read it.
//!
//! `error` is nullable because the native half makes it `Some` exactly when
//! `ok` is false, which costs `rule:security/isolate-shares-nothing`'s own `$result->error->message` a `?->`
//! or a narrowing until the checker can read one off the `ok` test. That is the
//! honest type and the alternative is a shape whose fields are empty strings on
//! the success path, which is the state PHP's `errno`/`error_get_last` pair
//! already proves is read wrong.
//!
//! `output` is typed `string` here, and that is the one field whose type is not
//! settled: `rule:tooling/echo-always-has-a-sink` and `rule:security/capture-answers-the-carrier` make captured output carry the *parent sink's*
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
//! (`rule:classes/no-free-functions-or-constants`), and the ADR marks its whole surface provisional pending the
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

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, Expr, ExprKind, SpawnOption, SpawnOptionKey};
use rustc_hash::FxHashSet;

use crate::expr_table::ArgSlot;
use crate::locals::LocalScope;
use crate::ty::{ShapeField, Ty, TypeId};
use crate::{Ctx, Env};

use super::assign::{is_assignable, report_mismatch};
use super::{check_expr, reject_secret_crossing};

/// `spawn script <path> with(<options>)` — `rule:security/isolate-shares-nothing`'s isolate spawn.
///
/// Two checks that do not know about each other: [`check_entry`] on the
/// operand, and one pass over the options. The answer is [`script_handle`] on
/// every path, including the ones that reported — a construct with an honest
/// type reports the mistakes on the *other* lines too, and the one thing a
/// program can write with a handle is the one thing that has a type to check
/// it against.
///
/// # What each option is typed as
///
/// `args:` is the value that crosses and has no expected type, for the reason
/// the arm below gives. `output:` is the spelling that chooses a sink. The
/// three that narrow a child take the shapes their enforcement reads:
///
/// - **`limits:` is a shape of [sub-caps](SUB_CAP_SETTINGS)**, each optional,
///   because a spawn site narrows the ceilings it names and inherits the rest
///   (`rule:security/isolate-budget-is-the-trees`). A key that is not one of
///   them is `E0454` rather than an extra field `rule:types/shape-type`'s width
///   subtyping would accept: a misspelled ceiling that compiles is a narrowing
///   the program asked for and did not get, which is the failure the whole
///   option exists to prevent.
/// - **`grants:` is an `array<string>`** — capability names in the spelling
///   `nvs.toml` grants them under, which is the one spelling a reader has to
///   learn and the one the door already asks against
///   (`rule:security/capability-check-at-the-door`). What the names *mean* is
///   not a question this crate can answer: a deployment's overlay decides
///   which authorities exist, so a name is checked where it is asked for and
///   never here.
/// - **`on:` is a written placement**, checked by [`check_placement`] against
///   the two `rule:concurrency/on-worker-runs-the-child-on-another-core` names
///   rather than by an expected type, because `string` accepts every spelling
///   that is not one of them.
pub(crate) fn check_spawn_script(
    path: &Expr,
    options: &[SpawnOption],
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_entry(path, live, scope, ctx, env);
    let string = env.interner.string();
    for opt in options {
        let expected = match opt.key {
            // The value that crosses. No expected type, because `rule:classes/graph-copy`'s
            // walk is what decides whether a given graph may cross and that is
            // a run-time question for everything a declared type does not
            // already settle.
            SpawnOptionKey::Args => None,
            SpawnOptionKey::Output => Some(string),
            SpawnOptionKey::Limits => Some(sub_caps(env)),
            SpawnOptionKey::Grants => Some(env.interner.array(string)),
            // Checked below, against the two placements rather than against a
            // type — see this function's own doc.
            SpawnOptionKey::On => None,
        };
        let ty = check_expr(&opt.value, expected, live, scope, ctx, env);
        match opt.key {
            // `rule:security/secret-sinks-refuse`'s second carrier. The bullet refuses a `secret` value at
            // the graph copy "for both its callers alike", so this is the same
            // refusal `Core\Serialize::encode` reports and not a spawn-specific
            // rule — `super::quals::reject_secret_crossing` owns the sentence, and
            // only the clause naming where the copy went differs.
            SpawnOptionKey::Args => reject_secret_crossing(
                &opt.value,
                ty,
                "`spawn script`'s `args:` copies it into a child whose arena this request \
                 cannot reach into",
                None,
                env,
            ),
            SpawnOptionKey::Limits => reject_unknown_sub_cap(ty, opt.span, env),
            SpawnOptionKey::On => check_placement(&opt.value, env),
            SpawnOptionKey::Grants | SpawnOptionKey::Output => {}
        }
    }
    script_handle(env)
}

/// ADR 0006 § *Decision*'s operand rule: the entry is a path **or** a static
/// method, decided syntactically at the spawn site, and nothing else is one.
///
/// Syntactically is the load-bearing word, and it is why this is a match on the
/// operand's shape before it is a question about its type. A
/// `Class::method(...)` reference and a variable holding the callable that
/// reference produces have the same type and are not the same operand: the
/// first names a function the compiler can see, and the second names a value
/// whose provenance — and therefore whether it captures — is not knowable here.
/// A rule phrased over types could not tell them apart, and the qualifier that
/// would let it is the one the ADR declines to add.
///
/// So there are three outcomes and each reports at most one diagnostic:
///
/// - **A path**, which is anything else, checked against `string` exactly as it
///   was before the method form existed. The assignability question is asked
///   here rather than by passing an expected type to [`check_expr`], because
///   this position accepts two unrelated shapes and an expected type is a claim
///   that it accepts one — a `callable` reaching a `string` parameter would
///   otherwise be reported as an ordinary mismatch, which describes half the
///   rule.
/// - **An `fn` literal or a `callable` value**, refused under
///   [`code::E_SPAWN_ENTRY_NOT_A_PATH_OR_METHOD`] with the way out the ADR
///   names.
/// - **A method reference**, which is the specified form and has nothing left
///   to refuse: it lowers to its own `Core` symbol (`nvs_ir::lower`'s
///   `lower_spawn_script`), carrying the entry's parameter names, and
///   `nvs_stdlib::script` binds `args:`'s entries to them by name.
///
/// **Known gap.** ADR 0006 § *Decision* reports a name `args:` holds that the
/// entry does not declare, or a parameter it omits, *at compile time when
/// `args:` is a literal* and at the spawn otherwise. Only the second half is
/// asked: the spawn raises the named-argument error for either mismatch, and a
/// literal map is not compared against the signature here yet. What that costs
/// is when the error arrives, never whether it does.
fn check_entry(
    path: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    // Checked first and on every path: a typo *inside* the operand is worth
    // reporting whichever of the three it turns out to be, and with no expected
    // type for the reason above.
    let ty = check_expr(path, None, live, scope, ctx, env);
    entry_operand(path, ty, "`spawn script`", env);
    // The path form is a path position
    // (`rule:programs/path-literals-resolve-from-their-file`): a relative
    // literal names a script beside this file. A method reference is not a
    // string literal, so this records nothing for it.
    crate::paths::resolve_literal(path, env);
}

/// The rule itself, over an operand that has already been checked — the half
/// [`check_entry`] and [`check_core_entry_argument`] share.
///
/// Split out rather than duplicated because `rule:concurrency/an-upgrade-is-spawn-shaped` opens a connection
/// with "0006's operand", so the second site is the *same* rule and not a rule
/// like it: a reader who has seen `E0802` at a `spawn script` sees the same
/// three outcomes, the same labels and the same help at `Core\Socket::upgrade`,
/// with only the construct's name differing. `form` is that name, and it is the
/// only thing either caller contributes.
///
/// It takes the operand's type rather than checking the expression itself, so
/// the `Core` caller — where the argument has already been walked against the
/// parameter — reports nothing twice.
fn entry_operand(path: &Expr, ty: TypeId, form: &str, env: &mut Env<'_>) {
    if is_method_reference(path) {
        return;
    }

    // `Ty::ShapeOfCallables` is a `Core` signature's spelling for "a shape of
    // callables answering this" (`crate::ty`), and `Ty::CallableSig` is a
    // closure whose signature is written out, so all three are the same
    // operand as far as this rule is concerned — none of them is a name the
    // spawn site can see through, and a written signature says how the value
    // is called rather than where its code came from.
    if matches!(
        env.interner.get(ty),
        Ty::Callable | Ty::ShapeOfCallables(_) | Ty::CallableSig { .. }
    ) {
        env.diags.report(refuse_entry(path.span, path, form));
        return;
    }

    let string = env.interner.string();
    if !is_assignable(ty, string, env.interner, env.graph, env.signatures) {
        report_mismatch(path.span, string, ty, env);
    }
}

/// A `Core` member that **opens an isolate** — one whose registry row marks a
/// parameter [`CoreTy::Entry`](nvs_stdlib::registry::CoreTy::Entry), which is
/// `rule:concurrency/an-upgrade-is-spawn-shaped`'s `Core\Socket::upgrade` and § 5's `Core\Sse` when it lands.
///
/// The hook [`super::calls::infer_static_call`] reaches after the target has
/// resolved, beside the other refusals over a `Core` call's own arguments.
/// **Which parameter is an entry is read off the row**, never off a roster kept
/// here: `nvs_stdlib::registry::entry_parameter` is the one home of that
/// question, and a member that opens an isolate is checked by declaring the
/// mark. A row marking an *instance* parameter would go unchecked, which is why
/// `nvs-stdlib` holds the marked rows to static ones rather than this function
/// growing a second call site for a member that does not exist.
///
/// **Two rules, because such a call is two things.** The entry takes `rule:security/isolate-shares-nothing`'s
/// operand rule, [`entry_operand`]'s. Every *other* argument is what crosses
/// into the child, so it takes `rule:security/secret-sinks-refuse`'s refusal at the graph copy —
/// [`reject_secret_crossing`], the same one `spawn script`'s `args:` reaches
/// from [`check_spawn_script`], with only the clause naming the carrier
/// differing. The mark identifies the member for both: an isolate opener is
/// what a row declaring an entry *is*, so the second rule needs no second
/// mark, and a `Core\Socket::upgrade` whose `args:` quietly carried a `secret`
/// where the sibling construct refuses one would be a hole in a boundary
/// rather than a missing convenience.
///
/// The entry's declared type is `mixed`, so nothing below the argument walk can
/// tell what filled it — the written expression is the last place the shape is
/// still visible, exactly as it is for [`reject_secret_crossing`]'s carriers.
/// The mapping is the call's own [`ArgSlot`]s rather than a position, because a
/// `name:` argument fills the entry as surely as a positional one and a `...`
/// spread fills no single parameter at all.
pub(crate) fn check_core_isolate_call(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    env: &mut Env<'_>,
) {
    let owner = qname.to_string();
    let Some(index) = nvs_stdlib::registry::entry_parameter(&owner, member) else {
        return;
    };
    let CallArgs::List(list) = args else {
        return;
    };
    let form = format!("`{owner}::{member}`");
    let carrier =
        format!("{form} copies it into a root isolate whose arena this request cannot reach into");
    for ((arg, &ty), &slot) in list.iter().zip(arg_types).zip(slots) {
        if slot == ArgSlot::Param(index) {
            entry_operand(&arg.value, ty, &form, env);
            continue;
        }
        reject_secret_crossing(&arg.value, ty, &carrier, None, env);
    }
}

/// Whether the operand is `Class::method(...)` — the reference the ADR accepts,
/// and not a call.
///
/// [`CallArgs::FirstClassCallable`] is the parser's mark for the literal `(...)`
/// argument list, so this is the written shape and not a type test: a
/// `Class::method()` with an empty argument list is an ordinary static call
/// whose *result* is the operand, and it takes the path branch as any other
/// expression does.
fn is_method_reference(path: &Expr) -> bool {
    matches!(
        &path.kind,
        ExprKind::StaticCall {
            args: CallArgs::FirstClassCallable,
            ..
        }
    )
}

/// The refusal for the two spellings ADR 0006 § *Decision* names as forbidden,
/// which differ only in whether there is a mechanical way out.
///
/// An `fn` literal has one — name it — and the diagnostic says so, because the
/// ADR's reason for refusing it is that `fn() => …` one keyword away in `spawn
/// worker` *does* capture, and one spelling with two meanings is what the
/// refusal exists to prevent. A variable has none to offer: what it holds is
/// not visible here, so the help can only name the two forms that are.
///
/// `form` names the construct the entry was written for and is the only part
/// either site contributes — see [`entry_operand`]. The help below says "here"
/// rather than "at the spawn site" for the same reason: the rule is that the
/// entry is written where it is used rather than carried to it, which is as
/// true of a call's argument as of a `spawn script`'s operand.
fn refuse_entry(span: Span, path: &Expr, form: &str) -> Diagnostic {
    let diag = Diagnostic::error(
        code::E_SPAWN_ENTRY_NOT_A_PATH_OR_METHOD,
        format!("a {form} entry is a path or a static method"),
    );
    if matches!(path.kind, ExprKind::Fn(_)) {
        diag.with_primary(span, "this is an `fn` literal")
            .with_help(
                "give it a name: a `static` method, spawned as `Class::method(...)`. \
                 An `fn` literal is refused because the same literal in `spawn worker` \
                 captures its enclosing scope, and an isolate shares nothing but compiled code",
            )
    } else {
        diag.with_primary(span, "this is a `callable`").with_help(
            "write the entry here rather than passing it in — a path, or \
                 `Class::method(...)`. Whether a `callable` in a variable captures is not \
                 knowable here, and an isolate shares nothing but compiled code",
        )
    }
}

/// The `[limits]` keys a spawn site may narrow, written as `nvs.toml` writes
/// them — `memory: "64M"`, `cpu_time: "2s"`.
///
/// The `Runtime`-class directives of that block (`nvs_config::directive`) and
/// no others: those are the ceilings a request may already set for itself, and
/// `limits:` asks the same question one level down, against what the tree has
/// left (`rule:security/isolate-budget-is-the-trees`). A `System` key —
/// `limits.max_script_depth`, the fatal reserves — is the operator's, and a
/// program narrowing the ceiling that exists to bound it is the direction that
/// class refuses.
///
/// One spelling rather than two: a size and a duration are strings here because
/// they are strings in the file the same value is configured in, so the
/// enforcement half reads either through `nvs_config`'s own parser rather than
/// growing a second one for the numbers a spawn site writes.
const SUB_CAP_SETTINGS: [&str; 4] = ["cpu_time", "max_output", "memory", "wall_time"];

/// The sub-caps of [`SUB_CAP_SETTINGS`]' block that are a plain count, and so
/// are written as an `int`.
const SUB_CAP_COUNTS: [&str; 1] = ["max_tasks"];

/// The two placements `rule:concurrency/on-worker-runs-the-child-on-another-core`
/// names, in the spelling `on:` takes.
const PLACEMENTS: [&str; 2] = ["here", "worker"];

/// `limits:`'s expected type: every sub-cap, each optional.
///
/// Optional because a spawn narrows the ceilings it names and inherits the
/// rest — a required field would make `{memory: "64M"}` mean "and no bound on
/// anything else", which is the opposite of what a sub-cap is.
fn sub_caps(env: &mut Env<'_>) -> TypeId {
    let string = env.interner.string();
    let int = env.interner.int();
    let mut fields: Vec<ShapeField> = SUB_CAP_SETTINGS
        .iter()
        .map(|name| optional_field(name, string))
        .collect();
    fields.extend(SUB_CAP_COUNTS.iter().map(|name| optional_field(name, int)));
    env.interner.shape(fields)
}

/// One field of [`sub_caps`]' shape.
fn optional_field(name: &str, ty: TypeId) -> ShapeField {
    ShapeField {
        name: name.to_owned(),
        ty,
        required: false,
    }
}

/// The half [`sub_caps`] cannot express: a key that is not a sub-cap at all.
///
/// `rule:types/shape-type`'s width subtyping accepts a field the target does
/// not name, which is right for a shape and wrong for this option — a
/// `{memmory: "64M"}` that compiles is a ceiling the program asked for and did
/// not get. So the written shape's own field names are read back and the extras
/// refused, which is [`code::E_UNKNOWN_OPTION`]'s own reading of a bag: a
/// misspelled key that is silently ignored is what that code exists for.
fn reject_unknown_sub_cap(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Shape(fields) = env.interner.get(ty) else {
        return;
    };
    let unknown: Vec<String> = fields
        .iter()
        .filter(|field| !is_sub_cap(&field.name))
        .map(|field| field.name.clone())
        .collect();
    for name in unknown {
        env.diags.report(
            Diagnostic::error(
                code::E_UNKNOWN_OPTION,
                format!("`{name}` is not a limit a spawn site can narrow"),
            )
            .with_primary(span, "no sub-cap of this name")
            .with_help(format!("the sub-caps are: {}", sub_cap_list())),
        );
    }
}

/// Whether `name` is one of the two lists above.
fn is_sub_cap(name: &str) -> bool {
    SUB_CAP_SETTINGS.contains(&name) || SUB_CAP_COUNTS.contains(&name)
}

/// Every sub-cap, for the help text — the list a program reads instead of the
/// directive table, since what is narrowable here is a subset of that table.
fn sub_cap_list() -> String {
    SUB_CAP_SETTINGS
        .iter()
        .chain(SUB_CAP_COUNTS.iter())
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `on:`'s rule: the placement is one of two words, **written here**.
///
/// A written word rather than a `string` value, because the placement decides
/// which scheduler starts the child and is decided once, where the spawn is
/// (`rule:concurrency/on-worker-runs-the-child-on-another-core`). A computed
/// one would have to be read at run time, where the only answer to a spelling
/// nobody declared is a spawn that fails for a reason the program could have
/// been told about here; two spawns under an `if` say the same thing with the
/// question asked at compile time.
///
/// The refusal is this code and not a mismatch against a union of the two
/// literals, for [`entry_operand`]'s reason in a smaller key: naming an
/// expected type claims the position accepts values of it, and this one accepts
/// two words.
fn check_placement(value: &Expr, env: &mut Env<'_>) {
    let diag = if let ExprKind::Str(span) = value.kind {
        let written = crate::string_lit::cook_string_literal(env.src, span);
        if PLACEMENTS.contains(&written.as_str()) {
            return;
        }
        Diagnostic::error(
            code::E_SPAWN_PLACEMENT_UNKNOWN,
            format!("`{written}` is not a placement"),
        )
        .with_primary(value.span, "no core is named this")
    } else {
        Diagnostic::error(
            code::E_SPAWN_PLACEMENT_UNKNOWN,
            "a placement is written at the spawn site",
        )
        .with_primary(
            value.span,
            "this is a value rather than one of the two words",
        )
    };
    env.diags.report(diag.with_help(
        "`on: \"worker\"` starts the child on another core and `on: \"here\"` starts it on \
         this one, which is what a spawn without the option does. A placement a program \
         computes is two spawns under an `if`",
    ));
}

/// `await <operand>` — the prefix half of the same surface, checked in the
/// same shape.
///
/// Its operand is checked first, and **against [`script_handle`]**: `await
/// $handel` is an undefined variable whether or not the construct compiles,
/// and `await 5` is a value that no `spawn script` produced. That second one is
/// reported by [`check_expr`] itself, naming the expected class — this module
/// adds no rule of its own for it, because there is nothing about the mismatch
/// that a declared parameter of the same type would not already say.
///
/// The answer is [`script_result`] whether or not the operand checked out: a
/// program that reads `$result->ok` hears about a `string` binding on the next
/// line rather than only about `await` itself.
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

/// `rule:security/isolate-shares-nothing`'s `ScriptResult`, interned: the shape this module's doc decides on,
/// with one field per member of the native `Completion` that fills it.
pub(crate) fn script_result(env: &mut Env<'_>) -> TypeId {
    let bool_ty = env.interner.bool_ty();
    let mixed = env.interner.mixed();
    let string = env.interner.string();
    let failure = env.interner.shape(vec![
        ShapeField::required("class".to_owned(), string),
        ShapeField::required("message".to_owned(), string),
    ]);
    let null = env.interner.null();
    let maybe_failure = env.interner.make_union([failure, null]);
    env.interner.shape(vec![
        ShapeField::required("ok".to_owned(), bool_ty),
        ShapeField::required("value".to_owned(), mixed),
        ShapeField::required("output".to_owned(), string),
        ShapeField::required("error".to_owned(), maybe_failure),
    ])
}
