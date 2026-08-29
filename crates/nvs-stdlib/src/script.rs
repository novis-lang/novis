//! `Core\Script\Handle` — what `spawn script` hands back, and the one `Core`
//! class registered so that a program can name a value it may not touch.
//!
//! [ADR 0006](../../../../docs/adr/0006-isolated-script-execution.md)'s spawn
//! is an expression, so its answer needs a type, and `await` is the only thing
//! a program may do with that answer. `crates/nvs-types/src/expr/isolate.rs`'s
//! module doc is the one home of *why* this is a `Core` class rather than a
//! shape — a `Core` instance has no property a program can reach, which is
//! exactly what a handle is — and this module does not restate the argument.
//!
//! # Registered with no members, on purpose
//!
//! [`crate::attributes`] registers members whose bodies never run; this
//! registers no members at all, which is a different thing and a stronger one.
//! There is no `$job->cancel()`, no `$job->isDone()` and no `$job->id()`:
//! cancellation is the parent's own teardown reaching its children
//! ([ADR 0072](../../../../docs/adr/0072-core-task-structured-concurrency.md)
//! § 5), and every "is it finished yet" answer is stale before the caller reads
//! it — the same reasoning [`crate::channel`] records for `count`/`isFull`.
//! `await` is the whole surface, and it is a keyword rather than a member
//! because it suspends the calling task.
//!
//! The class is absent from [`crate::registry::CONSTRUCTORS`] for the same
//! reason, so `new Core\Script\Handle()` is refused where it is written: the
//! only thing that may build one is the lowering of a `spawn script`, which
//! knows what isolate it names.
//!
//! # No slots yet, and that is not an oversight
//!
//! [`CoreClass::slots`](crate::registry::CoreClass::slots) is the layout a
//! helper body reads back by index, so it is declared by whoever writes it.
//! Nothing writes one yet — the lowering that builds a handle out of a live
//! `nvs_host::Isolate` is item 22's, and it is what decides whether the
//! isolate's identity is one word in a slot or an entry in a table this crate
//! keeps. Declaring a name here first would be a layout only one of the two
//! files agreed on. **What it spends** today: nothing at run time and one
//! `CoreClass` of statics in the binary.
//!
//! `Core\Script` itself — item 22's `args()` and the `valueOrThrow($result)`
//! that a shape cannot carry as a method — lands in this module beside its
//! handle, the way `Core\Task` and `Core\Task\Channel<T>` already sit together
//! under [ADR 0011](../../../../docs/adr/0011-functions-and-constants-are-class-members.md).

use crate::registry::CoreClass;

/// The handle class's fully-qualified name, as
/// [`CoreTy::Instance`](crate::registry::CoreTy::Instance) spells it.
///
/// `pub` because `nvs-types` names this class when it types a `spawn script`
/// expression, and a name written out in two crates is a name that can drift.
pub const HANDLE_NAME: &str = r"Core\Script\Handle";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const HANDLE: CoreClass = CoreClass {
    name: HANDLE_NAME,
    methods: &[],
    instance: &[],
    slots: &[],
    constants: &[],
};
