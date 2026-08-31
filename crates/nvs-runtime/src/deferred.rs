//! [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md) § 6's
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
//! [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) `FATAL` all
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
//! **Known gap: an isolate is sealed for want of a drain, not by design.** A
//! `spawn script` child runs a whole program and its own frame returning is a
//! perfectly good trigger; nothing calls [`run_deferred`] on its context, and
//! the place that would is `nvs_host::isolate`'s completion path.
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
//! # Known gap: § 7's `max_concurrent` is not counted here
//!
//! The cap bounds "request trees per core kept alive for deferred work", and a
//! host that can hold two such trees at once is the server. `nvs run` has one
//! request tree per process and no spelling reaches the directive from inside a
//! request — it is `System` — so a count kept here would bound one against 256
//! and refuse nothing that a program could ever make it refuse. The directive is
//! registered (`nvs_config::DIRECTIVES`) and the tree parses it; **the check
//! belongs with the mount that holds the trees**, at the same call site this
//! module's seal is checked, and it is written when there is one to hold them.
//!
//! # What it spends
//!
//! One `Vec` header per request — three words, and no allocation for the
//! requests that register nothing — plus two words and one closure reference
//! per registration. It is
//! O(in-flight trees) rather than O(requests served), per
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md), and nothing on
//! the request path reads any of it: the queue is touched by the member and by
//! the drain, both of which are already off the hot path.

use std::time::Duration;

use nvs_render::Level;

use crate::ctx::Ctx;
use crate::host::{Bounds, Job, Outcome};
use crate::value::Value;

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
/// ordinarily — `nvs-cli`'s `run` today, a mount once there is a server. The
/// module doc owns why "ordinarily" is a condition and what a failing request
/// does instead.
pub fn run_deferred(ctx: &mut Ctx) {
    for work in ctx.take_deferred() {
        run_one(ctx, work);
    }
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
