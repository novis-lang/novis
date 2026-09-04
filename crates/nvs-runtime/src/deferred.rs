//! [ADR 0072](/docs/adr/0072-core-task-structured-concurrency.md) § 6's
//! after-response work: what `Core\Task::afterResponse` registered, and the one
//! place it runs.
//!
//! # When "after the response" is, on a host that has no response
//!
//! § 6 is written for a request: the connection ends, the request tree does
//! not. `nvs run` has no response at all — it is a CLI invocation — so the
//! section's trigger has to be stated as something both hosts share. **It is
//! the request task's own frame returning.** Under a server that is the moment
//! the response is fully written, because the response *is* what the request
//! frame produced; under `nvs run` it is the end of the script. One rule, and
//! nothing about the member's meaning changes with which host is running it.
//!
//! **A request that did not return ordinarily runs none of it.** An uncaught
//! throw, a [`crate::EXITED`] and an
//! [ADR 0020](/docs/adr/0020-error-escalation-ladder.md) `FATAL` all
//! leave the request's status on the context, and that status is what the host
//! is about to report; script running over it would either lose the status or
//! lose its own. § 6's subject is a request that produced a response, and the
//! other three cases belong to the ladder rather than to this queue. The
//! registrations are released with the context, unrun.
//!
//! # Only the request's own task may register
//!
//! The queue is a field of one [`Ctx`], and a [`Ctx::child`] — a `Core\Task`
//! child, an isolate, and the deferred closures below, which run as children
//! themselves — is born **sealed**. A registration made on one would be drained
//! by nobody and released when that child ended, so a refusal is the only
//! honest answer: `Core\Task::afterResponse` throws the `RuntimeError` § 6's
//! last bullet names, at the call site, while there is still a request to
//! decide what to do about it. What this costs is a `Core\Task::all` child that
//! wants to defer having to hand the work back to the request that started it;
//! what it buys is that no registration is ever silently dropped.
//!
//! **An isolate is not sealed**, and that is the rule above read for a program
//! rather than a second rule: a `spawn script` child, a scheduled entry and a
//! served request each run a whole one, so the frame that produced the answer
//! returning is what "after the response" is for all three.
//! `nvs_host::isolate`'s completion path is the drain, and it runs there
//! *after* the answer is filed and the joiner has been told it may take it —
//! so a served request's after-response work runs on a tree the connection no
//! longer holds. The connection letting go is not enough on its own: a task's
//! death cancels whatever it left running, so that path detaches the tree from
//! the connection (`nvs_host::detach_current`) before it runs any of § 6's
//! work.
//!
//! # Each registration is one task, and the queue is a leaf
//!
//! A deferred closure runs as a group of one through [`crate::host::Host::run_group`],
//! which is what makes it "an ordinary task" in § 6's sense: it may `Core\Task::all`
//! inside itself with no special case, it is cancelled with the tree, and § 7's
//! `deadline` is [`Bounds::deadline`] with nothing else to implement. They run
//! in registration order rather than concurrently — § 6 asks for no ordering
//! between two `afterResponse` calls, and running them one at a time keeps a
//! tree's peak where the request already put it.
//!
//! **Deferred work may not defer more**, which is the section above applied to
//! this module's own children and not a second rule: a deferred closure runs on
//! a sealed context like every other child. The drain also seals the request's
//! own queue as it takes it, so nothing extends a drain already under way. A
//! queue that could extend itself is a tree that never leaves flight, and § 6's
//! whole affordability argument is that the tree stays in flight only *a
//! little* longer than the connection.
//!
//! # § 7's `max_concurrent` counts trees, and the refusal is at the call site
//!
//! The cap bounds "request trees per core kept alive for deferred work", so the
//! count is **per core and per tree**: a request takes one slot at its *first*
//! registration, however many it goes on to make, and gives it back when the
//! drain ends or when the context goes down with the queue unrun. A count of
//! registrations would bound the wrong thing — what an operator is sizing for
//! is [`Deferred`]'s own sentence about cost, which is one held arena per tree.
//!
//! [`DeferError::AtCapacity`] is the whole of the refusal, and it happens
//! **before** anything is queued: `Core\Task::afterResponse` turns it into the
//! `RuntimeError` § 7 names, at the call site, while the request is still
//! running and can still decide what to do about it. It does not queue and it
//! does not wait, for the reason that section gives — an unbounded queue in
//! front of a non-durable executor hides the overload and then loses the work
//! anyway.
//!
//! The count is a thread-local because a core is a thread here and the cap is
//! `System`-class: nothing inside a request can move it, so there is no reload
//! to race and no lock to take on a path a request reaches.
//!
//! # What it spends
//!
//! One `Vec` header per request — three words, and no allocation for the
//! requests that register nothing — plus two words and one closure reference
//! per registration. It is
//! O(in-flight trees) rather than O(requests served), per
//! [ADR 0004](/docs/adr/0004-memory-for-simplicity.md), and nothing on
//! the request path reads any of it: the queue is touched by the member and by
//! the drain, both of which are already off the hot path.

use std::cell::Cell;
use std::time::Duration;

use nvs_render::Level;

use crate::ctx::Ctx;
use crate::host::{Bounds, Job, Outcome};
use crate::value::Value;

/// Why a registration was refused — [`Ctx::defer`]'s two answers, which
/// `Core\Task::afterResponse` renders as two different `RuntimeError` messages
/// because they oblige a caller to do two different things.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferError {
    /// The queue is sealed: this context is a child — a `Core\Task` child, an
    /// isolate, or deferred work itself — and § 6's last bullet refuses it. The
    /// caller hands the work back to the request that started it; retrying
    /// cannot help.
    Sealed,
    /// This core is already holding `cap` trees alive for after-response work
    /// (§ 7). The caller may do the work inline, respond anyway, or tell its own
    /// caller to retry — the load will pass.
    AtCapacity {
        /// `[deferred] max_concurrent` in force, so the message can name the
        /// number an operator would have to change.
        cap: u64,
    },
}

thread_local! {
    /// How many request trees this core is holding open for after-response
    /// work, against § 7's cap.
    ///
    /// Maintained by [`Ctx::defer`] and [`Ctx::release_deferred_slot`] and by
    /// nothing else, which is why it can be a bare [`Cell`]: every write is on
    /// the thread that owns the tree, at a point that already has `&mut Ctx`.
    static TREES: Cell<u64> = const { Cell::new(0) };
}

/// Takes a slot for one tree if the core has one under `cap`.
pub(crate) fn take_tree_slot(cap: u64) -> bool {
    TREES.with(|trees| {
        if trees.get() >= cap {
            return false;
        }
        trees.set(trees.get() + 1);
        true
    })
}

/// Gives one back, for a tree that has finished its after-response work or
/// ended without running any.
pub(crate) fn give_back_tree_slot() {
    TREES.with(|trees| trees.set(trees.get().saturating_sub(1)));
}

/// How many trees this core is holding open for after-response work right now.
///
/// The observable half of § 7's cap: the number an operator sizes
/// `max_concurrent` against, and the one thing a test can read to tell a tree
/// that is still in flight from one that has gone.
#[must_use]
pub fn trees_in_flight() -> u64 {
    TREES.with(Cell::get)
}

/// One `Core\Task::afterResponse` registration — a closure this context owns a
/// reference to, and the deadline the call resolved to.
#[derive(Debug)]
pub struct Deferred {
    /// The closure, owned: a helper's arguments are borrowed from the caller's
    /// frame and this one outlives the call by the rest of the request.
    pub(crate) closure: Value,
    /// § 7's `deadline` in nanoseconds — the option the call named, or the
    /// `[deferred] deadline` default it inherited — and `0` for neither, which
    /// is a deferred task under no deadline of its own.
    pub(crate) deadline_nanos: u64,
}

/// Runs everything `Core\Task::afterResponse` registered on this request, in
/// registration order, and seals the queue.
///
/// Called by whoever ran the request's own frame, immediately after it returned
/// ordinarily — `nvs-cli`'s `run` for a CLI invocation, and
/// `nvs_host::isolate`'s completion path for everything that runs as an
/// isolate, a served request included. The module doc owns why "ordinarily" is
/// a condition and what a failing request does instead.
pub fn run_deferred(ctx: &mut Ctx) {
    for work in ctx.take_deferred() {
        run_one(ctx, work);
    }
    // At the end and not at the take above, because § 7's cap counts a tree for
    // as long as it is *kept alive* for this work — and the arena an operator
    // is sizing for is still held all the way through the loop.
    ctx.release_deferred_slot();
}

/// One registration, as a group of one.
#[expect(
    unsafe_code,
    reason = "the queue handed over the one reference it held to this closure, \
              and this is the frame that releases it"
)]
fn run_one(ctx: &mut Ctx, work: Deferred) {
    let closure = work.closure;
    let bounds = Bounds {
        limit: None,
        deadline: (work.deadline_nanos != 0).then(|| Duration::from_nanos(work.deadline_nanos)),
    };
    let outcome = crate::host::with_current(|host| {
        let job: Job = Box::new(move |child: &mut Ctx| call_deferred(child, closure));
        host.run_group(ctx, vec![job], bounds)
    });
    match outcome {
        // § 6: an uncaught throw goes through ADR 0020's ladder, and tier 2's
        // `onUncaughtThrow` does **not** fire, because it was the finished
        // request's. So this lands at tier 4, the floor, as the same record
        // `Core\Log::write` writes — `crate::floor` owns why one shape and not
        // two. The *request's* status is not touched either way, since there is
        // no response left for this to affect.
        Some(Outcome::Threw(thrown)) => {
            let mut record = crate::floor::uncaught(&thrown);
            record
                .envelope
                .fields
                .push(("origin".to_owned(), crate::floor::text(ORIGIN)));
            crate::floor::report(ctx, &record);
        }
        Some(Outcome::TimedOut) => {
            let mut record =
                crate::floor::note(Level::Error, "deferred work stopped: its deadline expired");
            record
                .envelope
                .fields
                .push(("origin".to_owned(), crate::floor::text(ORIGIN)));
            crate::floor::report(ctx, &record);
        }
        // The tree itself was cancelled while this ran. Nothing to report and
        // nothing left to run: the remaining registrations are drained into
        // this same loop and each finds the cancellation again.
        Some(Outcome::Cancelled | Outcome::Completed(_)) => {}
        // No host on this thread — an embedder that installed none. The work
        // was registered and is owed, so it runs here rather than being
        // dropped, and the only thing it loses is the deadline.
        None => {
            let answer = call_deferred(ctx, closure);
            // SAFETY: `call_deferred` answers with a value it owns.
            unsafe { answer.release() };
        }
    }
    // SAFETY: the queue held exactly one reference to this closure and handed
    // it over with the `Deferred` above; nothing else points at it.
    unsafe { closure.release() };
}

/// Calls one deferred closure and leaves any failure where the host reads it.
///
/// The same translation `nvs_stdlib::task`'s `call_child` performs and for the
/// same reason: a [`Job`] has no error channel, because the host takes a
/// child's failure off the child's own context.
fn call_deferred(child: &mut Ctx, closure: Value) -> Value {
    match crate::call_closure(child, closure, &[]) {
        Ok(answer) => answer,
        // The callee already recorded what failed; that is the whole of what
        // this variant means.
        Err(crate::Fault::Pending(_)) => Value::null(),
        Err(crate::Fault::Thrown(class, message)) => {
            child.set_pending_as(class, message);
            Value::null()
        }
        // `Fault` is `#[non_exhaustive]`, and a deferred call reaches the other
        // variants only through a helper ABI that already raised what it means
        // — the tier that cannot be silently wrong is the fatal one.
        Err(other) => {
            child.set_pending(format!("a deferred task failed: {other:?}"));
            Value::null()
        }
    }
}

/// The `origin` field both failures above carry — ADR 0020 § 6's "whatever
/// structured context that call site has", which here is the one fact a reader
/// cannot recover from the record otherwise: the throw happened after the
/// response, in work the request registered rather than in the request.
const ORIGIN: &str = "deferred work";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputSink;

    /// [ADR 0072](/docs/adr/0072-core-task-structured-concurrency.md)
    /// § 7: a core holds at most `max_concurrent` request trees open for
    /// after-response work, and the tree past the cap is refused at the call
    /// site rather than queued.
    ///
    /// Three claims in one, because they are one rule and a test that split
    /// them would let a plausible implementation pass two of them. **The cap
    /// counts trees**, so a second registration by a tree that is already
    /// counted takes nothing; **the refusal names the directive**, since that
    /// is what a program's `catch` and an operator's grep both need; and **a
    /// slot comes back when its tree ends**, which is what makes the cap a
    /// ceiling on what is in flight rather than on what has ever run.
    ///
    /// The registrations are `null` on purpose: nothing here drains, so the
    /// closures are never called, and what is being pinned is the arithmetic
    /// in front of the queue. `nvs-host`'s `tests/deferred.rs` is where a real
    /// registration runs.
    #[test]
    fn the_deferred_queue_is_bounded_by_max_concurrent() {
        let cap = Ctx::new(OutputSink::Sink).deferred_max_concurrent();
        assert!(
            cap > 0,
            "§ 7's default cap is not a number to hold trees at"
        );
        assert_eq!(
            trees_in_flight(),
            0,
            "this core was holding trees before the test made one"
        );

        let mut held: Vec<Ctx> = Vec::new();
        for tree in 0..cap {
            let mut ctx = Ctx::new(OutputSink::Sink);
            assert_eq!(
                ctx.defer(Value::null(), 0),
                Ok(()),
                "tree {tree} was refused under a cap of {cap}"
            );
            held.push(ctx);
        }
        assert_eq!(trees_in_flight(), cap, "the core is not counting trees");

        // A tree already counted goes on registering: the second closure is
        // more work for one tree, not a second tree.
        assert_eq!(
            held[0].defer(Value::null(), 0),
            Ok(()),
            "a tree that already holds a slot was refused its second closure"
        );
        assert_eq!(
            trees_in_flight(),
            cap,
            "a second registration by one tree took a second slot"
        );

        let mut over = Ctx::new(OutputSink::Sink);
        assert_eq!(
            over.defer(Value::null(), 0),
            Err(DeferError::AtCapacity { cap }),
            "the tree past the cap was not refused, or was not told the number"
        );

        // The end of a tree is the other half of the count.
        held.pop();
        assert_eq!(
            trees_in_flight(),
            cap - 1,
            "a tree that ended did not give its slot back"
        );
        assert_eq!(
            over.defer(Value::null(), 0),
            Ok(()),
            "the freed slot was not available to the next tree"
        );
    }
}
