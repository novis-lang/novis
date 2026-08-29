//! `Core\Task` — [ADR 0072](../../../../docs/adr/0072-core-task-structured-concurrency.md)'s
//! structured concurrency, registered here as a signature ahead of the body
//! that will answer it.
//!
//! What is on disk is the **compile-time half** of § 1: `Task::all` takes a
//! shape literal of zero-argument `fn` literals and answers a shape with the
//! same field names, each field carrying *that field's* declared return type.
//! [`crate::registry::CoreTy::CallableShapeTo`] is the mechanism and owns why
//! no ordinary type at that position could say it;
//! `nvs_types::expr::args` is the one place a field is read, and the one place
//! `E0773`/`E0774` are reported.
//!
//! # The body is a placeholder, and this is the third such row
//!
//! [`crate::attributes`] and [`crate::program`] register signatures whose
//! members are folded in `nvs check` and therefore genuinely never run. This
//! one is not that: a program that writes `Core\Task::all({...})` compiles,
//! `nvs-ir` lowers the call, and [`unimplemented_scheduler_half`] is what it
//! reaches. Running the closures as children of the calling task — with § 3's
//! `{limit, deadline}` enforced and § 4's "control does not leave the call with
//! work still running" held — is the *next* slice of Stage 4, and it needs the
//! `nvs-host` scheduler reachable from a helper, which no `Core` member is yet.
//! The body says exactly that and stops rather than answering plausibly.
//!
//! Nothing under `tests/conformance/` calls it: the three cases that name
//! `Core\Task::all` are all `--EXPECTF-ERROR--` cases over § 1's typing rules,
//! which is what the whole class currently is.
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
    methods: &[CoreMethod {
        name: "all",
        params: &[CoreTy::CallableShapeTo("S"), CoreTy::Options(OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Var("S"),
        symbol: "nvs_core_task_all",
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_task_all" => (unimplemented_scheduler_half as *const ()).cast(),
        _ => return None,
    })
}

/// The body the row names until the scheduler half of § 1 lands — see this
/// module's own docs, which own why this is a placeholder rather than the
/// unreachable body [`crate::attributes`] has.
extern "C" fn unimplemented_scheduler_half() {
    // Written through the handle rather than with `eprintln!`, which this
    // crate's lints refuse: a `Core` member's own output goes through
    // `nvs_runtime::Ctx`'s sinks, and this is not output — it is the last
    // thing a process does before stopping.
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(
        b"nvs: `Core\\Task::all` type-checks but has no body yet - ADR 0072 \xc2\xa7 1's \
          children run on the `nvs-host` scheduler, which no `Core` member can reach \
          from a helper at this commit\n",
    );
    std::process::abort();
}
