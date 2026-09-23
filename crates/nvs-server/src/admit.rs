//! `rule:http-server/the-server-block-is-boot-class`
//! 's in-flight ceiling, as
//! `rule:http-server/admission-is-arithmetic-not-a-number`
//! states it: the arithmetic that turns a written `max_in_flight` into an
//! effective one, and the one relaxed counter that refuses a request over it.
//!
//! # Why an arithmetic and not a number
//!
//! A ceiling on *concurrency* and a cap on *per-request memory* that have no
//! stated relationship do not bound anything together — their product is what
//! the machine is actually asked to hold, and where that product exceeds what
//! it has, the operating system's out-of-memory killer is the real admission
//! controller. It terminates the process, which is `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` tier A's failure
//! arriving through the one door every cap above it was supposed to have
//! closed. So [`Ceiling::of`] takes the **smaller** of what the file asked for
//! and what the budget affords, and [`Ceiling::clamp_note`] is the sentence an
//! operator reads when those two disagree. Clamping rather than refusing to
//! boot is § 13's own call: a server that will not start because two directives
//! disagree is the worse outage, and the operator learns the same fact either
//! way.
//!
//! The inputs are `nvs_config::server::Capacity`'s, and reading them is
//! that module's — including the part that asks the operating system what this
//! machine has, and prefers a container's limit to the host's. The division is
//! here because the clamp is an admission decision and the counter that
//! enforces it has to live beside the thing it refuses.
//!
//! # Refusing before allocating, and not after
//!
//! [`Admission::admit`] is taken at the top of a request, **before** a mount is
//! selected, before an isolate exists and before any Novis code runs. § 5 is
//! explicit that the order is the whole point: a cap that allocates in order to
//! refuse does not protect what it exists to protect. The consequence to accept
//! is that this path has no custom error page — the answer is a fixed `503`
//! with `Retry-After: 1` and no body, and there is nowhere for an application
//! to be asked about it, because being asked is the cost being avoided.
//!
//! **One relaxed atomic, counted process-wide rather than per core.** A
//! per-core share would let one hot core refuse while its neighbours idle,
//! which is a ceiling that binds the wrong thing. Relaxed is enough because
//! nothing is published through this counter: it orders no memory and guards no
//! data, it only answers how many requests are in flight, and it is not on the
//! value path `rule:programs/memory-priority`'s
//! non-atomic refcount decision protects.
//!
//! **What it spends**, per that ADR: two `usize`s for the whole process — the
//! count and the ceiling a reload moves — one
//! [`InFlight`] guard per request being served — a borrow and nothing else, so
//! O(in-flight) with a zero-sized tail — and the response above, which allocates
//! no body at all.

use std::sync::atomic::{AtomicUsize, Ordering};

use hyper::{Response, StatusCode};
use nvs_config::Capacity;

use crate::serve::Answer;

/// What the engine holds whatever the traffic is, subtracted from the budget
/// before it is divided: the compiled-unit cache, the JIT's own arenas, one
/// accept loop and one coroutine stack per core.
///
/// **A stated reserve rather than a measurement**, and deliberately generous:
/// under-reserving is the direction that hands admission back to the
/// out-of-memory killer, while over-reserving costs an operator a few
/// concurrent requests they can ask for explicitly. M7's benchmark is what
/// replaces it with a number, and the plan's *Verify* paragraph is where that
/// is asked for.
pub const ENGINE_RESERVE: u64 = 128 * 1024 * 1024;

/// § 13's arithmetic, done: the two numbers and which of them won.
///
/// Kept as all three rather than reduced to [`effective`](Self::effective)
/// because the clamp has to be *reportable* — an operator whose observed
/// capacity changed should be able to find out why from a log line rather than
/// from a benchmark, which is the whole reason § 13 rejected clamping silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ceiling {
    /// `[server] max_in_flight`, or § 5's own default where it is unwritten.
    pub configured: u64,
    /// What the budget affords: the budget less [`ENGINE_RESERVE`], divided by
    /// the per-request cap. `None` where the tree states no cap or the host
    /// answered no budget — there is then no second number, and the configured
    /// one stands.
    pub afforded: Option<u64>,
    /// The smaller of the two, and the number [`Admission`] enforces.
    pub effective: usize,
}

impl Ceiling {
    /// The arithmetic over the numbers the configuration resolved.
    ///
    /// **A machine too small for even one request still admits one.** The floor
    /// is not a rounding convenience: a ceiling of zero is a process that
    /// accepts connections and answers `503` to everything, which is the
    /// deployment `E0622` refuses when it is written down, and reaching it by
    /// division instead would be the same server arrived at silently.
    #[must_use]
    pub fn of(capacity: &Capacity) -> Self {
        let afforded = match (capacity.budget, capacity.per_request) {
            (Some(budget), Some(cap)) => {
                let afforded = budget.saturating_sub(ENGINE_RESERVE) / cap;
                Some(afforded.max(1))
            }
            _ => None,
        };
        let effective = afforded.map_or(capacity.configured, |afforded| {
            afforded.min(capacity.configured)
        });
        Self {
            configured: capacity.configured,
            afforded,
            // Only reachable on a 32-bit host asked to hold four billion
            // requests, where the memory budget has already been the binding
            // number for a long time.
            effective: usize::try_from(effective).unwrap_or(usize::MAX),
        }
    }

    /// The one line § 13 asks to be logged at boot when the two numbers
    /// disagree, naming both directives and both numbers; `None` when the
    /// configured ceiling is what stands.
    ///
    /// A `String` handed back rather than a log call made here, because this
    /// crate is given a socket and not a logger — the boot that owns the
    /// process is what writes it, once.
    #[must_use]
    pub fn clamp_note(&self) -> Option<String> {
        let afforded = self.afforded?;
        (afforded < self.configured).then(|| {
            format!(
                "`server.max_in_flight` is {configured}, but the memory budget affords \
                 {afforded} requests at `limits.memory` each; {afforded} is the ceiling in force",
                configured = self.configured,
            )
        })
    }
}

/// The process-wide count of requests in flight, and the ceiling it refuses
/// past.
///
/// Shared across every core rather than divided among them, which the module
/// doc owns. Nothing here is generic over a clock or a policy: § 5's valve is
/// one number and one comparison, and a valve with a strategy in it is a
/// scheduler.
#[derive(Debug)]
pub struct Admission {
    /// [`Ceiling::effective`] for the snapshot in force, which a reload sets
    /// through [`Admission::resize`] because `limits.memory` is a `Reload`
    /// directive and it is one of § 13's inputs. A request already admitted
    /// keeps its place: the count is not reset, so a lowered ceiling refuses
    /// new requests until enough of the admitted ones end.
    ceiling: AtomicUsize,
    /// How many requests are between [`Admission::admit`] and the end of their
    /// answer.
    in_flight: AtomicUsize,
}

impl Admission {
    /// The valve § 13's arithmetic settled on.
    #[must_use]
    pub fn new(ceiling: &Ceiling) -> Self {
        Self {
            ceiling: AtomicUsize::new(ceiling.effective),
            in_flight: AtomicUsize::new(0),
        }
    }

    /// Refuses past `ceiling` from the next [`admit`](Self::admit) on.
    ///
    /// Relaxed for the module doc's reason: the number orders no memory, and
    /// a request that races the store is checked against one of the two
    /// ceilings, either of which was in force at that moment.
    pub fn resize(&self, ceiling: &Ceiling) {
        self.ceiling.store(ceiling.effective, Ordering::Relaxed);
    }

    /// A place in the count, or `None` where the ceiling is already met.
    ///
    /// `fetch_update` rather than a `fetch_add` and a rollback: the rollback
    /// shape lets the count read above the ceiling for as long as the losing
    /// racers take to give their increments back, and something has to be true
    /// about a number named "in flight" for [`in_flight`](Self::in_flight) to
    /// be worth exporting under
    /// `rule:observability/the-runtime-exports-what-it-already-measures`.
    #[must_use]
    pub fn admit(&self) -> Option<InFlight<'_>> {
        let ceiling = self.ceiling();
        self.in_flight
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                (current < ceiling).then_some(current + 1)
            })
            .ok()
            .map(|_| InFlight { admission: self })
    }

    /// How many requests are in flight right now.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// The number this valve refuses past — [`Ceiling::effective`] as the last
    /// boot or reload set it.
    #[must_use]
    pub fn ceiling(&self) -> usize {
        self.ceiling.load(Ordering::Relaxed)
    }
}

/// One admitted request's place in the count, given back however that request
/// ended.
///
/// A guard rather than a decrement after the answer, for `serve::Served`'s
/// reason: `rule:concurrency/cancellation-runs-no-user-code`
/// 's cancellation tears a coroutine down where it parked, so the line after
/// the answer is exactly the one a cancelled request never reaches — and a
/// ceiling that leaks a place per cancelled client is a server that refuses
/// everything after enough of them.
#[derive(Debug)]
pub struct InFlight<'a> {
    /// The count this place belongs to.
    admission: &'a Admission,
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.admission.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

/// § 5's fixed answer at the ceiling: `503`, `Retry-After: 1`, no body.
///
/// Fixed in every sense — no application was asked, because not asking is the
/// point, and `Retry-After` is what gives a proxy in front of this something to
/// fail over on rather than a bare failure to blame on the wrong component.
#[must_use]
pub fn over_capacity() -> Response<Answer> {
    let mut response = Response::new(Answer::empty());
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response.headers_mut().insert(
        hyper::header::RETRY_AFTER,
        hyper::header::HeaderValue::from_static("1"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One mebibyte, so the arithmetic below reads in the units an operator
    /// writes.
    const MIB: u64 = 1024 * 1024;

    fn capacity(configured: u64, per_request: Option<u64>, budget: Option<u64>) -> Capacity {
        Capacity {
            configured,
            per_request,
            budget,
        }
    }

    /// § 13's whole rule, from both sides: the budget binds where it is the
    /// smaller number, the written ceiling binds where it is, and an absent
    /// half of the arithmetic leaves the written one standing rather than
    /// producing a zero.
    #[test]
    fn max_in_flight_is_derived_from_the_memory_budget() {
        // 1 GiB, less the 128 MiB reserve, at 64 MiB a request: 14 places, not
        // the 10,000 the file asked for.
        let clamped = Ceiling::of(&capacity(10_000, Some(64 * MIB), Some(1024 * MIB)));
        assert_eq!(clamped.afforded, Some(14), "the budget was not divided");
        assert_eq!(clamped.effective, 14, "the larger number won");
        let note = clamped.clamp_note().expect("a clamp went unreported");
        assert!(
            note.contains("10000") && note.contains("14"),
            "the clamp did not name both numbers: {note}"
        );

        // The same machine asked for less than it affords: nothing is clamped
        // and nothing is logged.
        let asked = Ceiling::of(&capacity(8, Some(64 * MIB), Some(1024 * MIB)));
        assert_eq!(asked.afforded, Some(14), "the budget stopped being read");
        assert_eq!(asked.effective, 8, "the configured ceiling did not stand");
        assert!(
            asked.clamp_note().is_none(),
            "a ceiling under the budget was reported as a clamp"
        );

        // Neither half of the arithmetic on its own is a bound: a tree that
        // states no per-request cap has nothing for a concurrency ceiling to
        // have a relationship with, and a host that answers no budget has not
        // been asked a question this crate can put to it.
        for absent in [
            capacity(10_000, None, Some(1024 * MIB)),
            capacity(10_000, Some(64 * MIB), None),
            capacity(10_000, None, None),
        ] {
            let ceiling = Ceiling::of(&absent);
            assert_eq!(ceiling.afforded, None, "an absent input became a number");
            assert_eq!(ceiling.effective, 10_000, "the written ceiling was lost");
            assert!(ceiling.clamp_note().is_none(), "an absence was reported");
        }

        // A machine too small for one request still admits one: a ceiling of
        // zero is the `503`-only server `E0622` refuses when it is written
        // down, and division must not arrive at it silently.
        let tiny = Ceiling::of(&capacity(10_000, Some(512 * MIB), Some(64 * MIB)));
        assert_eq!(tiny.effective, 1, "the floor of one was not held");
    }

    /// The valve itself: places are handed out up to the ceiling, refused past
    /// it, and given back by the guard's drop rather than by a line after the
    /// answer.
    #[test]
    fn a_place_in_the_count_is_given_back_however_the_request_ended() {
        let admission = Admission::new(&Ceiling::of(&capacity(2, None, None)));
        let first = admission.admit().expect("the first request was refused");
        let second = admission.admit().expect("the second request was refused");
        assert_eq!(admission.in_flight(), 2);
        assert!(
            admission.admit().is_none(),
            "a third request was admitted past a ceiling of two"
        );

        drop(first);
        assert_eq!(
            admission.in_flight(),
            1,
            "the guard did not give its place back"
        );
        let third = admission.admit().expect("a freed place was not reusable");
        drop((second, third));
        assert_eq!(admission.in_flight(), 0, "the count did not return to zero");
    }

    /// A reload moves the ceiling under requests already admitted: they keep
    /// their places, a lowered ceiling refuses until enough of them end, and a
    /// raised one admits at once.
    #[test]
    fn a_resized_valve_keeps_the_requests_it_already_admitted() {
        let admission = Admission::new(&Ceiling::of(&capacity(2, None, None)));
        let first = admission.admit().expect("the first request was refused");
        let second = admission.admit().expect("the second request was refused");

        admission.resize(&Ceiling::of(&capacity(1, None, None)));
        assert_eq!(admission.ceiling(), 1);
        assert_eq!(admission.in_flight(), 2, "a resize dropped a place");
        drop(first);
        assert!(
            admission.admit().is_none(),
            "a request was admitted with the count still at the lowered ceiling"
        );

        admission.resize(&Ceiling::of(&capacity(3, None, None)));
        let third = admission.admit().expect("a raised ceiling refused");
        drop((second, third));
        assert_eq!(admission.in_flight(), 0, "the count did not return to zero");
    }
}
