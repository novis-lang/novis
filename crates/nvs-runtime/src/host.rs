//! The seam a `Core` member reaches its host through — one thread-local, one
//! trait, and one operation shaped like the promise it has to keep.
//!
//! [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md) § 1's
//! `Core\Task::all` runs its fields as children of the calling task, and that
//! task lives on `nvs-host`'s scheduler. A `Core` member is a `nvs-stdlib`
//! helper. Nothing joined those two before this module, and the three decisions
//! that join them are recorded here because this is the file neither side can
//! be read without.
//!
//! # 1. The edge is inverted through this crate, not added between the two
//!
//! `nvs-stdlib` does **not** depend on `nvs-host`, and it may not start: the
//! signature registry lives in `nvs-stdlib`, so `nvs-types` and `nvs-codegen`
//! both depend on it, and an edge from there to the host would link `mio`,
//! `corosensei` and `core_affinity` into `nvs check` — a type checker carrying
//! a reactor and a coroutine library to answer a question about a signature.
//!
//! So the trait is declared *here*, in the crate both sides already depend on.
//! `nvs-host` implements it; `nvs-stdlib` calls it; neither names the other.
//! That is the same inversion `nvs-runtime` already runs on the other axis —
//! this crate's `ctx` module holds `Core\Cli\Text`'s name as a constant rather
//! than asking `nvs-stdlib` about it, "the dependency running `nvs-stdlib` →
//! `nvs-runtime` and not back".
//!
//! # 2. A thread-local, not a second opaque pointer in `Ctx`
//!
//! `nvs-host`'s `reactor` module made this same call for the reactor and its
//! module doc owns the general argument; what is repeated here is only the part
//! that decides it for a *scheduler*, because a `Core` member — unlike a
//! `std::io::Read` — does hold a [`Ctx`] and could have read a pointer out of
//! it.
//!
//! **A scheduler is per core; a `Ctx` is per request.** Putting it in the
//! context is one copy of the core's identity per in-flight request, written on
//! every spawn, to say something that is true of the whole thread — and `Ctx`
//! is `#[repr(C)]` with offsets compiled code loads inline, so a field there is
//! an ABI change rather than a struct change. The yielder is in `Ctx` because
//! it genuinely is per *task*: it points into the stack of the one coroutine
//! that is running, and there is no thread-wide answer to what it is. A host is
//! the opposite shape, so it takes the opposite route.
//!
//! [`install`] publishes one for as long as its guard lives — a worker does it
//! once, around everything it runs — and [`with_current`] is how a helper
//! borrows it. It nests and restores, so a scheduler driven from inside another
//! one's task does not clobber the outer one.
//!
//! # 3. What crosses is a group, not a task API
//!
//! [`Host`] has one method and it is [`Host::run_group`]. It is deliberately
//! **not** `spawn` / `wait` / `cancel` for `nvs-stdlib` to sequence, and that is
//! the decision worth the most here.
//!
//! ADR 0072 § 4's guarantee — "control does not leave the call with work still
//! running" — is a property of the *sequence*, not of any one call in it. A
//! seam handing out task ids makes keeping it the caller's diligence again,
//! which is the exact failure `nvs_host::spawn_child` already refuses on the
//! parent link: it reads the running task rather than taking a parent, so § 1's
//! "each is a child of the calling task" is a property of the call. The same
//! argument applies one level up. A group crosses whole, the host owns the
//! waiting and the cancelling, and there is no order of seam calls a future
//! member could get wrong.
//!
//! It is also what lets both members share it: `Task::all` and `Task::map`
//! differ in how their jobs are *built* — a shape literal's fields against one
//! callback over an array — and in nothing about how they run.
//!
//! [`Outcome`] is § 4's table, minus its last row. "The calling task is
//! cancelled" is not a variant because it is not a return: `nvs-host` tears a
//! cancelled task down by a forced unwind of its stack, which travels *through*
//! this call rather than out of it, and what that unwind runs is native `Drop`
//! and no script code (§ 5).
//!
//! # What it spends
//!
//! One machine word pair per thread — a null-checked wide pointer in a
//! thread-local, `const`-initialized and holding no `Drop` type, which is what
//! this crate's own allocator module requires of every one in it. It is
//! O(cores) and does not grow with requests served, per
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md). A helper that
//! asks pays one thread-local load and one null test.

use std::cell::Cell;
use std::time::Duration;

use crate::ctx::Ctx;
use crate::throwable::Thrown;
use crate::value::Value;

/// One child of the group: what it runs, and the answer it hands back.
///
/// A boxed Rust closure rather than a Novis `callable` [`Value`], because the
/// two members build their jobs differently and the host has no business
/// knowing which: `Task::all` closes over one field's `fn` literal, `Task::map`
/// over the shared callback and one element. What the host is told is that this
/// runs on a child's stack with a child's context and produces a value.
///
/// **A job may be dropped without ever being called**, and the builder owes the
/// release in that case. A `limit` that never lets the last job start, a
/// sibling that threw, and an expired deadline all end the group with jobs
/// still queued, and the host drops them rather than running work whose result
/// § 4 has already decided it will not return. Anything a job captured that
/// owns a reference — the element `Task::map` closed over, the closure both
/// members call — must therefore be released by that capture's own [`Drop`] and
/// not only by the body, or a cancelled group leaks one reference per job it
/// never reached.
pub type Job = Box<dyn FnOnce(&mut Ctx) -> Value>;

/// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md) § 3's
/// `{limit?: uint, deadline?: Duration}`, decoded.
///
/// `None` is that section's "unbounded" in both fields, which is why neither is
/// a number with a sentinel: a `limit` of `0` is a real bound meaning "run
/// nothing", and no `Duration` means "never".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bounds {
    /// How many children may be running at once, or `None` for all of them.
    pub limit: Option<u32>,
    /// When cancellation is *requested*, or `None` for no deadline of this
    /// group's own. § 4 is the one home for why that is not when the call
    /// returns.
    pub deadline: Option<Duration>,
}

/// How a group ended — [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
/// § 4's table, and the module docs own why its last row is not here.
///
/// Every variant is reached with **nothing still running**. That is the whole
/// of what the member promises and it is discharged on this side of the seam.
#[derive(Debug)]
pub enum Outcome {
    /// Every child returned. One answer per [`Job`], in the order given.
    Completed(Vec<Value>),
    /// A child threw, every sibling was cancelled, and the call waited for
    /// those cancellations. This is the **first** throw by completion order; a
    /// second is written by the host rather than handed back, because only one
    /// can propagate and neither may be swallowed.
    ///
    /// A [`Thrown`] rather than a [`Value`], because that is what a failure
    /// already is on both sides of this seam: it is what [`Ctx::take_thrown`]
    /// hands the host out of the child's context and what [`Ctx::raise`] takes
    /// from the member on the way out, so neither end has to unwrap an object
    /// pointer out of a tagged value and put it back. It may be null — the
    /// state a bare-message fault with no exception class installed leaves —
    /// and `raise` handles that as it already does everywhere else.
    Threw(Thrown),
    /// The deadline expired, every child was cancelled, and the call waited.
    /// The member turns this into `TimeoutError`; the host does not know that
    /// class.
    TimedOut,
}

/// Whatever is running tasks on this thread, as much of it as a `Core` member
/// is allowed to want.
///
/// Exactly one implementor is ever expected — `nvs-host`'s scheduler — and
/// there is none at this commit: the seam is the route, and the body that
/// travels it is the next slice. The module docs own why the trait is declared
/// here rather than there, and why it has one method rather than a task API.
///
/// `Debug` is a supertrait so that [`Installed`] can derive it; an implementor
/// is expected to be a unit struct, since every scrap of a scheduler's state is
/// already reachable from the thread it belongs to.
pub trait Host: std::fmt::Debug {
    /// Runs `jobs` as children of the calling task under `bounds`, and returns
    /// only once none of them is still running.
    ///
    /// The calling task is suspended while they run, so this borrows its
    /// context rather than holding anything across the switch. What each child
    /// gets for a context, how the `limit` is applied, and what a cancellation
    /// waits for are all the implementor's, and `nvs-host`'s scheduler module
    /// is their home.
    fn run_group(&self, ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome;
}

thread_local! {
    /// The host running tasks on this thread, or `None` when nothing is.
    ///
    /// A `Cell` of a `Copy` wide pointer so it is `const`-initialized and
    /// carries no destructor — see the module docs' *What it spends*, and
    /// the `alloc` module for why that is a rule in this crate rather than a
    /// preference.
    static CURRENT: Cell<Option<&'static dyn Host>> = const { Cell::new(None) };
}

/// Restores whatever host was installed before, when dropped.
///
/// Held on the installer's own stack, which is why the `Drop` here does not
/// break the thread-local's no-destructor rule: the slot holds a pointer, this
/// guard holds the previous one.
#[derive(Debug)]
#[must_use = "the host is uninstalled the moment this guard is dropped"]
pub struct Installed {
    previous: Option<&'static dyn Host>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.set(self.previous.take()));
    }
}

/// Publishes `host` as this thread's host for as long as the guard lives.
///
/// Nesting is restoration, not replacement: a scheduler driven from inside
/// another one's task installs over it and puts the outer one back on the way
/// out.
pub fn install(host: &'static dyn Host) -> Installed {
    let previous = CURRENT.with(|slot| slot.replace(Some(host)));
    Installed { previous }
}

/// Borrows this thread's host, or answers `None` when there is none.
///
/// `None` is not a fault. A `nvs run` of a CLI program has no scheduler under
/// it, and a member that needs one says so through its own diagnostic rather
/// than assuming one is there.
pub fn with_current<R>(f: impl FnOnce(&dyn Host) -> R) -> Option<R> {
    CURRENT.with(Cell::get).map(f)
}

/// Whether this thread has a host, without borrowing it.
#[must_use]
pub fn is_installed() -> bool {
    CURRENT.with(|slot| slot.get().is_some())
}

#[cfg(test)]
mod tests {
    use super::{Bounds, Host, Installed, Job, Outcome, install, is_installed, with_current};
    use crate::ctx::{Ctx, OutputSink};

    /// A host that answers every group with an empty completion and counts the
    /// calls, which is all the route itself has to be proved against.
    #[derive(Debug)]
    struct Recording(std::cell::Cell<u32>);

    // The trait is `&'static dyn Host`, and a `static` is the only way to hand
    // one over. `Cell` is not `Sync`, so this test's own implementor is a
    // thread-local rather than a plain `static` — which is also the shape a
    // real host has, its state being per core.
    thread_local! {
        static RECORDING: &'static Recording =
            Box::leak(Box::new(Recording(std::cell::Cell::new(0))));
    }

    impl Host for Recording {
        fn run_group(&self, _ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome {
            self.0.set(self.0.get() + 1);
            assert!(bounds.limit != Some(0), "a limit of zero is a real bound");
            drop(jobs);
            Outcome::Completed(Vec::new())
        }
    }

    fn recording() -> &'static Recording {
        RECORDING.with(|host| *host)
    }

    fn install_recording() -> Installed {
        install(recording())
    }

    #[test]
    fn a_thread_with_no_host_answers_none_rather_than_panicking() {
        assert!(!is_installed());
        assert!(with_current(|_| ()).is_none());
    }

    #[test]
    fn an_installed_host_is_reachable_from_a_free_function() {
        let before = recording().0.get();
        let _guard = install_recording();
        assert!(is_installed());

        // Exactly the shape a `Core` member has: no argument carries the host
        // in, and the call site names no scheduler type.
        let mut ctx = Ctx::new(OutputSink::Sink);
        let outcome = with_current(|host| host.run_group(&mut ctx, Vec::new(), Bounds::default()));

        assert!(matches!(outcome, Some(Outcome::Completed(answers)) if answers.is_empty()));
        assert_eq!(recording().0.get(), before + 1);
    }

    #[test]
    fn installing_nests_and_restores_rather_than_clobbering() {
        let outer = install_recording();
        assert!(is_installed());
        {
            let _inner = install_recording();
            assert!(is_installed());
        }
        assert!(is_installed(), "the outer host is back, not gone");
        drop(outer);
        assert!(!is_installed());
    }
}
