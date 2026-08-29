//! `Core\Task` — [ADR 0072](../../../../docs/adr/0072-core-task-structured-concurrency.md)'s
//! structured concurrency, registered here as a signature ahead of the body
//! that will answer it.
//!
//! What is on disk is the **compile-time half** of §§ 1 and 2. `Task::all`
//! takes a shape literal of zero-argument `fn` literals and answers a shape
//! with the same field names, each field carrying *that field's* declared
//! return type. [`crate::registry::CoreTy::CallableShapeTo`] is the mechanism
//! and owns why no ordinary type at that position could say it;
//! `nvs_types::expr::args` is the one place a field is read, and the one place
//! `E0773`/`E0774` are reported.
//!
//! `Task::map` needs none of that, and the contrast is § 2's whole argument for
//! two members rather than one. Its subject is an `array<T>` and its callback
//! is written once for every element, so one ordinary
//! [`crate::registry::CoreTy::CallableTo`] binds `U` from that one callback's
//! declared return type and the answer is `array<U>` — the same three-line
//! shape `Core\Arr::map` already has, plus § 3's options bag. A shape literal
//! cannot express "one per element of a runtime array" and an array cannot
//! carry a per-element type; each member is the cheap spelling of exactly the
//! job the other cannot state.
//!
//! # Both bodies are one placeholder
//!
//! [`crate::attributes`] and [`crate::program`] register signatures whose
//! members are folded in `nvs check` and therefore genuinely never run. These
//! are not that: a program that writes `Core\Task::all({...})` or
//! `Core\Task::map([...], fn ...)` compiles, `nvs-ir` lowers the call, and
//! [`unimplemented_scheduler_half`] is what it reaches. Running the closures as
//! children of the calling task — with § 3's `{limit, deadline}` enforced and
//! § 4's "control does not leave the call with work still running" held — is
//! the *next* slice of Stage 4.
//!
//! **The route is now decided and on disk**: [`nvs_runtime::host`] is the seam
//! a `Core` member reaches its host through, and that module's own docs are the
//! one home for why it is a thread-local declared in `nvs-runtime` rather than
//! a dependency on `nvs-host`, and why what crosses it is a whole group rather
//! than a `spawn`/`wait`/`cancel` for this member to sequence. What is missing
//! is at both ends of it: nothing implements [`nvs_runtime::host::Host`] yet,
//! and neither row here turns its argument into
//! [`Job`](nvs_runtime::host::Job)s. The body says exactly that and stops
//! rather than answering plausibly.
//!
//! Nothing under `tests/conformance/` runs either one: the six cases that name
//! them are all `--EXPECTF-ERROR--` cases over §§ 1 to 3's typing rules, which
//! is what the whole class currently is. That is a real discharge of the
//! coverage floor and not a loophole — the gate greps `--FILE--` and runs
//! nothing, and a case pinning what the *checker* believes is the only kind
//! that can exist before a body does.
//!
//! # `{limit, deadline}` is the only optioned spelling
//!
//! § 3: one trailing options shape (ADR 0063 R2), the same two fields on `all`
//! and on the `map` that follows it. Both default to [`Const::Null`] and both
//! mean "unbounded" there — `limit` because a shape literal is already bounded
//! by its field count, and `deadline` because a call that names none is bounded
//! by the request tree's own `wall_time`, which ADR 0005 makes finite. There is
//! no `timeout` member and no `race`: a timeout on a group *is* the `deadline`
//! option, and § 3 defers `race` under the future spelling `Task::first`.

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};
use crate::time::DURATION_NAME;

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Task";

/// § 3's `{limit?: uint, deadline?: Duration}`, in the order the ABI passes
/// them.
///
/// Both are [`Const::Null`] rather than a number, for that variant's own
/// stated reason: neither type has an "unbounded" value in it, and a `limit`
/// of `0` would be a real bound meaning "run nothing". The module doc above is
/// the one home for what each default means.
const OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "limit",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "deadline",
        ty: CoreTy::Instance(DURATION_NAME),
        default: Const::Null,
    },
];

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "all",
            params: &[CoreTy::CallableShapeTo("S"), CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Var("S"),
            symbol: "nvs_core_task_all",
        },
        CoreMethod {
            name: "map",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableTo("U"),
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("U")),
            symbol: "nvs_core_task_map",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_task_all" | "nvs_core_task_map" => {
            (unimplemented_scheduler_half as *const ()).cast()
        }
        _ => return None,
    })
}

/// The body **both** rows name until the scheduler half of §§ 1 and 2 lands —
/// see this module's own docs, which own why this is a placeholder rather than
/// the unreachable body [`crate::attributes`] has.
///
/// One function rather than one per member, because what stops the call is the
/// same missing thing in both: the seam they would run their children through
/// has no implementor at this commit, so a second copy would differ only in the
/// member it names and would have to be deleted with the first.
extern "C" fn unimplemented_scheduler_half() {
    // Written through the handle rather than with `eprintln!`, which this
    // crate's lints refuse: a `Core` member's own output goes through
    // `nvs_runtime::Ctx`'s sinks, and this is not output — it is the last
    // thing a process does before stopping.
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(
        b"nvs: `Core\\Task` type-checks but has no body yet - ADR 0072 \xc2\xa7\xc2\xa7 1 and 2's \
          children run through the `nvs_runtime::host` seam, which nothing implements \
          or installs at this commit\n",
    );
    std::process::abort();
}
