//! `rule:http-server/the-server-block-is-boot-class`'s admission, as
//! `rule:http-server/admission-is-arithmetic-not-a-number` states it: a request
//! is admitted while the memory the process holds now leaves room for it to
//! reach its own ceiling, and while fewer than `max_in_flight` requests are in
//! flight.
//!
//! # Memory held, not memory assumed
//!
//! `[limits] memory` is the most one request may hold, and most requests hold
//! a small part of it. So [`Admission::admit`] admits when `used + cap` fits
//! under `budget − ENGINE_RESERVE`, where `used` is what every core holds as
//! `nvs_runtime::budget::held_by_cores` reads it and `cap` is the per-request
//! cap. What a connection holds between requests is not in `used`
//! (`nvs_runtime::Ctx::between_requests`), so idle connections do not refuse
//! a request. A newly admitted request always has room to reach its own ceiling. A
//! tree with no per-request cap (`memory = false`) is admitted while `used`
//! is under the room, and a host that answers no budget has no memory
//! condition. `max_in_flight` stays a count, because it is the operator's own
//! number and the one bound that holds whatever the requests weigh.
//!
//! **With no request in flight, a request is always admitted.** The process
//! then holds only the engine's own bytes, so a server that refused there
//! would refuse everything by arithmetic, which is the deployment `E0622`
//! refuses when it is written down.
//!
//! **What is admitted can still grow past the room together.** Each request
//! was given room for its own ceiling, but two admitted against the same
//! headroom can both grow into it. [`Ceiling::arm`] gives the runtime the same
//! limit, and a core whose published balance takes the sum past it stops the
//! request that grew, with the memory error its own ceiling would raise
//! (`nvs_runtime::budget::set_process_limit`). The limit is the out-of-memory
//! killer's job done one request at a time, with the process still running.
//!
//! **A burst is counted before it allocates.** A request admitted and not yet
//! run holds nothing, so a burst on one core was admitted whole, and the limit
//! above then stopped those past the room. Each admitted request counts one
//! publish step (`nvs_runtime::budget::hold_share`) until it grows one or ends,
//! and the request a connection starts runs before the next ready connection
//! is admitted (`nvs_host::spawn_child_next`), so a request that allocates has
//! published it by the next admission on its core. A burst of requests that
//! allocate is refused once their bytes reach the room, and a burst of light
//! ones costs a mebibyte each.
//!
//! The inputs are `nvs_config::server::Capacity`'s, and reading them is that
//! module's, including the part that asks the operating system what this
//! machine has.
//!
//! # Refusing before allocating, and not after
//!
//! [`Admission::admit`] is taken at the top of a request, **before** a mount is
//! selected, before an isolate exists and before any Novis code runs.
//! `rule:http-server/max-in-flight-refuses-before-allocating` is explicit that
//! the order is the whole point: a cap that allocates in order to
//! refuse does not protect what it exists to protect. The consequence to accept
//! is that this path has no custom error page — the answer is a fixed `503`
//! with `Retry-After: 1` and no body, and there is nowhere for an application
//! to be asked about it, because being asked is the cost being avoided.
//!
//! **Relaxed atomics, counted process-wide rather than per core.** A per-core
//! share would let one hot core refuse while its neighbours idle, which is a
//! ceiling that binds the wrong thing. Relaxed is enough because nothing is
//! published through these numbers: they order no memory and guard no data.
//! The sum of the gauges is stale by at most one publish step per core, which
//! is what the runtime's stop at the limit covers.
//!
//! # A wedged core is shed
//!
//! `rule:http-server/a-wedged-core-is-shed-never-killed`: admission reads the
//! watchdog's report through [`Admission::subscribe`] and has no detector of
//! its own. A core the watchdog reports takes no new work, because
//! [`Admission::admit`] refuses on a thread [`nvs_host::this_core_is_wedged`]
//! names. `max_in_flight` drops by the wedged cores' share, so the cores
//! still turning are not asked to carry a ceiling that counts the wedged one.
//! When a sweep finds the core turning again, its share returns. Nothing is
//! killed. A valve with no subscription, which is every test and `nvs run`,
//! sheds nothing.
//!
//! **What it spends**, per `rule:programs/memory-priority`: five words for the
//! whole process — the count, the three numbers a reload moves and the
//! watchdog's counts — one [`InFlight`] guard per request being served, which
//! is a borrow and nothing else, and the response above, which allocates no
//! body at all. Each admission reads one gauge per core, publishes the
//! admitting core's own, publishes it again if it admits, and reads the
//! watchdog's counts with three relaxed loads.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use hyper::{Response, StatusCode};
use nvs_config::Capacity;

use crate::serve::Answer;

/// What the engine holds whatever the traffic is, subtracted from the budget
/// before requests are admitted against it: the compiled-unit cache, the JIT's
/// own arenas, one accept loop and one coroutine stack per core.
///
/// **A stated reserve rather than a measurement**, and deliberately generous:
/// under-reserving is the direction that hands admission back to the
/// out-of-memory killer, while over-reserving costs an operator a little
/// headroom. No benchmark measures the engine's own footprint, so the figure is
/// stated here and nowhere else.
pub const ENGINE_RESERVE: u64 = 128 * 1024 * 1024;

/// The numbers admission compares against, resolved from one configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ceiling {
    /// `[server] max_in_flight`, or its shipped `10000` where it is unwritten.
    pub max_in_flight: usize,
    /// The most one request may hold, or `None` where the tree states no cap.
    pub per_request: Option<u64>,
    /// The budget less [`ENGINE_RESERVE`], or `None` where the host answered
    /// no budget. A budget smaller than the reserve leaves a room of `0`, so
    /// only the first request in flight is admitted.
    pub room: Option<u64>,
}

impl Ceiling {
    /// The numbers the configuration resolved, with the reserve taken off the
    /// budget.
    #[must_use]
    pub fn of(capacity: &Capacity) -> Self {
        Self {
            // Only short of the written number on a 32-bit host asked to hold
            // four billion requests, where memory binds long before the count.
            max_in_flight: usize::try_from(capacity.configured).unwrap_or(usize::MAX),
            per_request: capacity.per_request,
            room: capacity
                .budget
                .map(|budget| budget.saturating_sub(ENGINE_RESERVE)),
        }
    }

    /// Gives the runtime [`Self::room`] as the process's memory limit, so a
    /// request that grows past it after admission is stopped.
    ///
    /// The boot that owns the process calls this, and so does each reload: the
    /// limit is the process's, and a server test that armed it would stop
    /// requests of every other test in the same process.
    pub fn arm(&self) {
        nvs_runtime::budget::set_process_limit(self.room);
    }
}

/// The stored [`Ceiling::room`] where the host answered no budget.
const NO_BUDGET: u64 = u64::MAX;

/// The stored [`Ceiling::per_request`] where the tree states no cap. A cap of
/// zero is resolved to no cap by the configuration, so `0` is free to mean it.
const NO_CAP: u64 = 0;

/// The process-wide count of requests in flight, and the numbers it admits
/// against.
///
/// Shared across every core rather than divided among them, which the module
/// doc owns. Nothing here is generic over a clock or a policy: the valve is
/// two comparisons, and a valve with a strategy in it is a scheduler.
#[derive(Debug)]
pub struct Admission {
    /// [`Ceiling::max_in_flight`] for the snapshot in force.
    max_in_flight: AtomicUsize,
    /// [`Ceiling::per_request`], or [`NO_CAP`].
    per_request: AtomicU64,
    /// [`Ceiling::room`], or [`NO_BUDGET`].
    room: AtomicU64,
    /// How many requests are between [`Admission::admit`] and the end of their
    /// answer.
    in_flight: AtomicUsize,
    /// What every core holds now, in bytes: `nvs_runtime::budget::held_by_cores`
    /// outside this crate's own tests.
    held: fn() -> isize,
    /// The watchdog's counts, once [`Admission::subscribe`] is called.
    shed: OnceLock<nvs_host::Shed>,
}

impl Admission {
    /// The valve for `ceiling`, reading what the process holds from every
    /// core's gauge.
    #[must_use]
    pub fn new(ceiling: &Ceiling) -> Self {
        let admission = Self {
            max_in_flight: AtomicUsize::new(0),
            per_request: AtomicU64::new(NO_CAP),
            room: AtomicU64::new(NO_BUDGET),
            in_flight: AtomicUsize::new(0),
            held: nvs_runtime::budget::held_by_cores,
            shed: OnceLock::new(),
        };
        admission.resize(ceiling);
        admission
    }

    /// The same valve reading what the process holds from `held`, for a test
    /// that states the reading.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn reading(ceiling: &Ceiling, held: fn() -> isize) -> Self {
        Self {
            held,
            ..Self::new(ceiling)
        }
    }

    /// Admits against `ceiling` from the next [`admit`](Self::admit) on.
    ///
    /// A reload calls it, because `limits.memory` and `max_in_flight` are both
    /// inputs. A request already admitted keeps its place: the count is not
    /// reset, so a lowered ceiling refuses new requests until enough of the
    /// admitted ones end. Relaxed for the module doc's reason: a request that
    /// races the stores is checked against numbers each of which was in force
    /// at that moment.
    pub fn resize(&self, ceiling: &Ceiling) {
        self.max_in_flight
            .store(ceiling.max_in_flight, Ordering::Relaxed);
        self.per_request
            .store(ceiling.per_request.unwrap_or(NO_CAP), Ordering::Relaxed);
        self.room
            .store(ceiling.room.unwrap_or(NO_BUDGET), Ordering::Relaxed);
    }

    /// A place in the count, or `None` where the count is at `max_in_flight`
    /// or the memory the process holds leaves no room for one more request.
    ///
    /// The admitting core publishes its own balance first, so the reading is
    /// fresh for the core that asks. `fetch_update` rather than a `fetch_add`
    /// and a rollback: the rollback shape lets the count read above the
    /// ceiling for as long as the losing racers take to give their increments
    /// back, and something has to be true about a number named "in flight"
    /// for [`in_flight`](Self::in_flight) to be worth exporting under
    /// `rule:observability/the-runtime-exports-what-it-already-measures`.
    ///
    /// A core the watchdog reports as wedged is given no place, and the count
    /// refuses at `max_in_flight` less the wedged cores' share. The module
    /// doc's *A wedged core is shed* owns both.
    #[must_use]
    pub fn admit(&self) -> Option<InFlight<'_>> {
        if nvs_host::this_core_is_wedged() {
            return None;
        }
        nvs_runtime::budget::publish_held();
        let max_in_flight = self.available();
        let fits = self.memory_fits();
        let place = self
            .in_flight
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                (current < max_in_flight && (current == 0 || fits)).then_some(current + 1)
            })
            .ok()
            .map(|_| InFlight { admission: self });
        // Counted before it allocates, so the next admission on any core reads
        // it: the module doc's *A burst is counted before it allocates*.
        if place.is_some() {
            nvs_runtime::budget::hold_share();
        }
        place
    }

    /// Sheds onto the watchdog's report from the next [`admit`](Self::admit)
    /// on. The process's boot calls it once, with the one watchdog it builds,
    /// and a second call changes nothing.
    pub fn subscribe(&self, shed: nvs_host::Shed) {
        let _ = self.shed.set(shed);
    }

    /// `max_in_flight` less the share of the cores the watchdog reports as
    /// wedged, rounded so that a wedged core takes no more than its share.
    /// Every core wedged leaves `0`.
    fn available(&self) -> usize {
        let max_in_flight = self.max_in_flight();
        let Some(shed) = self.shed.get() else {
            return max_in_flight;
        };
        let cores = shed.cores();
        // Two loads, so a core leaving between them can read one more wedged
        // core than there are cores.
        let wedged = shed.wedged().min(cores);
        if cores == 0 || wedged == 0 {
            return max_in_flight;
        }
        let share = (max_in_flight as u128 * wedged as u128) / cores as u128;
        max_in_flight - usize::try_from(share).unwrap_or(max_in_flight)
    }

    /// Whether the memory every core holds now leaves room for one more
    /// request to reach its cap.
    fn memory_fits(&self) -> bool {
        let room = self.room.load(Ordering::Relaxed);
        if room == NO_BUDGET {
            return true;
        }
        let used = u64::try_from((self.held)()).unwrap_or(0);
        match self.per_request.load(Ordering::Relaxed) {
            NO_CAP => used < room,
            cap => used.saturating_add(cap) <= room,
        }
    }

    /// How many requests are in flight right now.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// The count this valve refuses at while no core is wedged, as the last
    /// boot or reload set it.
    #[must_use]
    pub fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::Relaxed)
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
        // What the request freed, and its share if it never grew, published as
        // it ends, so the next admission on any core reads it.
        nvs_runtime::budget::give_share_back();
    }
}

/// `rule:http-server/max-in-flight-refuses-before-allocating`'s fixed answer at
/// the ceiling: `503`, `Retry-After: 1`, no body.
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
    use std::sync::atomic::AtomicIsize;
    use std::time::{Duration, Instant};

    use nvs_runtime::{Ctx, OutputSink, TaskRoot};

    use super::*;

    /// One mebibyte, so the numbers below read in the units an operator
    /// writes.
    const MIB: u64 = 1024 * 1024;

    fn capacity(configured: u64, per_request: Option<u64>, budget: Option<u64>) -> Capacity {
        Capacity {
            configured,
            per_request,
            budget,
        }
    }

    /// The reserve comes off the budget, and an absent input stays absent.
    #[test]
    fn the_ceiling_takes_the_reserve_off_the_budget_and_divides_nothing() {
        let ceiling = Ceiling::of(&capacity(10_000, Some(64 * MIB), Some(1024 * MIB)));
        assert_eq!(ceiling.max_in_flight, 10_000, "the written count was moved");
        assert_eq!(ceiling.per_request, Some(64 * MIB));
        assert_eq!(ceiling.room, Some(1024 * MIB - ENGINE_RESERVE));

        let no_budget = Ceiling::of(&capacity(10_000, Some(64 * MIB), None));
        assert_eq!(no_budget.room, None, "an absent budget became a number");
        let tiny = Ceiling::of(&capacity(10_000, Some(64 * MIB), Some(64 * MIB)));
        assert_eq!(
            tiny.room,
            Some(0),
            "a budget under the reserve did not floor at zero"
        );
    }

    static WAITING: AtomicIsize = AtomicIsize::new(0);

    fn waiting() -> isize {
        WAITING.load(Ordering::Relaxed)
    }

    /// Many requests that each hold little are all admitted under a cap that
    /// the old count would have divided the budget by.
    #[test]
    fn many_light_requests_are_admitted_under_a_large_cap() {
        // A 15 GiB host at 128 MiB a request: 119 places by division.
        let ceiling = Ceiling::of(&capacity(10_000, Some(128 * MIB), Some(15 * 1024 * MIB)));
        let admission = Admission::reading(&ceiling, waiting);
        let mut places = Vec::new();
        for _ in 0..1_000 {
            let place = admission
                .admit()
                .expect("a light request was refused with room to spare");
            places.push(place);
            // Each waiting request holds about 80 KiB.
            WAITING.fetch_add(80 * 1024, Ordering::Relaxed);
        }
        assert_eq!(admission.in_flight(), 1_000);
        drop(places);
        assert_eq!(admission.in_flight(), 0, "the count did not return to zero");
    }

    static HEAVY: AtomicIsize = AtomicIsize::new(0);

    fn heavy() -> isize {
        HEAVY.load(Ordering::Relaxed)
    }

    /// A request is refused when what the process holds plus its cap passes
    /// the room, and admitted again once the room is back. The first request
    /// in flight is admitted whatever the reading says.
    #[test]
    fn a_request_is_refused_when_held_plus_its_cap_passes_the_room() {
        let room = 1024 * MIB;
        let ceiling = Ceiling::of(&capacity(
            10_000,
            Some(64 * MIB),
            Some(room + ENGINE_RESERVE),
        ));
        let admission = Admission::reading(&ceiling, heavy);

        HEAVY.store(isize::try_from(2 * room).expect("fits"), Ordering::Relaxed);
        let first = admission
            .admit()
            .expect("the first request in flight was refused by arithmetic");

        HEAVY.store(
            isize::try_from(room - 64 * MIB).expect("fits"),
            Ordering::Relaxed,
        );
        let second = admission
            .admit()
            .expect("a request whose cap fits exactly was refused");

        HEAVY.store(
            isize::try_from(room - 64 * MIB + 1).expect("fits"),
            Ordering::Relaxed,
        );
        assert!(
            admission.admit().is_none(),
            "a request was admitted with no room to reach its cap"
        );
        drop((first, second));
    }

    static UNCAPPED: AtomicIsize = AtomicIsize::new(0);

    fn uncapped() -> isize {
        UNCAPPED.load(Ordering::Relaxed)
    }

    /// With no per-request cap, a request is admitted while what the process
    /// holds is under the room. With no budget, memory is not a condition.
    #[test]
    fn no_cap_admits_under_the_room_and_no_budget_admits_on_count() {
        let room = 512 * MIB;
        let admission = Admission::reading(
            &Ceiling::of(&capacity(10_000, None, Some(room + ENGINE_RESERVE))),
            uncapped,
        );
        let first = admission.admit().expect("the first request was refused");
        UNCAPPED.store(isize::try_from(room - 1).expect("fits"), Ordering::Relaxed);
        let second = admission
            .admit()
            .expect("a request under the room was refused");
        UNCAPPED.store(isize::try_from(room).expect("fits"), Ordering::Relaxed);
        assert!(
            admission.admit().is_none(),
            "a request was admitted with the room used up"
        );

        admission.resize(&Ceiling::of(&capacity(10_000, Some(64 * MIB), None)));
        let third = admission
            .admit()
            .expect("a host with no budget refused on memory");
        drop((first, second, third));
    }

    /// What this test's thread last published as a core, which leaves out the
    /// cores every other test runs.
    fn this_core() -> isize {
        nvs_runtime::budget::held_by_this_core().unwrap_or(0)
    }

    /// A core whose connections hold more than the room between requests still
    /// admits a request, because admission reads what requests hold. Once a
    /// request starts on that connection, its bytes count again.
    #[test]
    fn connection_buffers_between_requests_do_not_refuse_a_request() {
        std::thread::spawn(|| {
            let _core = nvs_runtime::budget::CoreGauge::claim();
            let admission = Admission::reading(
                &Ceiling::of(&capacity(
                    10_000,
                    Some(16 * MIB),
                    Some(ENGINE_RESERVE + 32 * MIB),
                )),
                this_core,
            );
            // Another request is in flight, so the memory condition is asked.
            let first = admission.admit().expect("the first request was refused");

            // Handed to a task and armed on its stack, as a spawn does.
            let mut connection = Ctx::new(OutputSink::Sink);
            connection.leave_this_stack();
            connection.arm_on_this_stack();
            connection.between_requests();
            // The buffers an idle connection keeps, and a park, which is where
            // its tree's bytes are set aside.
            let buffers = vec![1_u8; 64 * 1024 * 1024];
            nvs_runtime::budget::ArmedStack::take().resume();
            assert!(
                admission.admit().is_some(),
                "a connection's buffers between requests refused a request ({} bytes published)",
                this_core()
            );

            // A request on that connection counts what the connection holds.
            connection.reroot();
            nvs_runtime::budget::publish_held();
            assert!(
                admission.admit().is_none(),
                "a request in flight was admitted past the room ({} bytes published)",
                this_core()
            );
            drop((buffers, first));
        })
        .join()
        .expect("the core thread panicked");
    }

    /// A burst on one core is refused once its shares reach the room, before
    /// any request of it allocates, and every share goes back as the burst
    /// ends.
    #[test]
    fn a_burst_on_one_core_is_refused_before_it_allocates() {
        std::thread::spawn(|| {
            let _core = nvs_runtime::budget::CoreGauge::claim();
            nvs_runtime::budget::publish_held();
            let before = this_core();
            let admission = Admission::reading(
                &Ceiling::of(&capacity(
                    10_000,
                    Some(16 * MIB),
                    Some(ENGINE_RESERVE + 24 * MIB),
                )),
                this_core,
            );
            let mut places = Vec::with_capacity(64);
            let mut refused = false;
            for _ in 0..64 {
                match admission.admit() {
                    Some(place) => places.push(place),
                    None => {
                        refused = true;
                        break;
                    }
                }
            }
            assert!(
                refused,
                "a burst of {} requests that allocated nothing was admitted past the room",
                places.len()
            );
            assert!(
                places.len() <= 9,
                "{} requests were admitted with room for 8 shares",
                places.len()
            );

            drop(places);
            assert!(
                this_core() < before + nvs_runtime::budget::GAUGE_STEP,
                "the burst's shares were not given back ({} bytes published, {before} before)",
                this_core()
            );
            assert!(admission.admit().is_some(), "the room did not come back");
        })
        .join()
        .expect("the core thread panicked");
    }

    /// The count: places are handed out up to `max_in_flight`, refused past
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

    static RELOADED: AtomicIsize = AtomicIsize::new(0);

    fn reloaded() -> isize {
        RELOADED.load(Ordering::Relaxed)
    }

    /// A reload moves both conditions under requests already admitted: they
    /// keep their places, a lowered ceiling refuses until enough of them end,
    /// and a raised one admits at once.
    #[test]
    fn a_resized_valve_keeps_the_requests_it_already_admitted() {
        let admission = Admission::reading(&Ceiling::of(&capacity(2, None, None)), reloaded);
        let first = admission.admit().expect("the first request was refused");
        let second = admission.admit().expect("the second request was refused");

        admission.resize(&Ceiling::of(&capacity(1, None, None)));
        assert_eq!(admission.max_in_flight(), 1);
        assert_eq!(admission.in_flight(), 2, "a resize dropped a place");
        drop(first);
        assert!(
            admission.admit().is_none(),
            "a request was admitted with the count still at the lowered ceiling"
        );

        admission.resize(&Ceiling::of(&capacity(3, None, None)));
        let third = admission.admit().expect("a raised ceiling refused");

        // A reload that leaves less room than the cap refuses on memory from
        // the next request, and keeps the ones in flight.
        RELOADED.store(isize::try_from(100 * MIB).expect("fits"), Ordering::Relaxed);
        admission.resize(&Ceiling::of(&capacity(
            3,
            Some(64 * MIB),
            Some(ENGINE_RESERVE + 128 * MIB),
        )));
        assert!(
            admission.admit().is_none(),
            "a request was admitted past a room a reload lowered"
        );
        assert_eq!(admission.in_flight(), 2, "a resize dropped a place");
        drop((second, third));
        assert_eq!(admission.in_flight(), 0, "the count did not return to zero");
    }

    /// A watchdog that reports a core a millisecond past its deadline, and
    /// sweeps every millisecond.
    fn a_quick_watchdog() -> nvs_host::Watchdog {
        nvs_host::Watchdog::with(Duration::from_millis(1), Duration::from_millis(1), |_| {})
    }

    /// A task id to file a deadline under. The task never runs: a deadline
    /// table only keys on the id.
    fn a_task() -> nvs_host::TaskId {
        let mut sched = nvs_host::Scheduler::new();
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, |_| {})
    }

    /// A deadline table with one deadline filed a millisecond from now, which
    /// no core will take: the table of a core that stopped turning.
    fn a_wedged_table(task: nvs_host::TaskId) -> nvs_host::Timers {
        let mut timers = nvs_host::Timers::default();
        timers.arm(task, Instant::now() + Duration::from_millis(1));
        timers
    }

    fn a_cpu() -> nvs_host::CpuId {
        *nvs_host::cpus()
            .first()
            .expect("the platform reported no CPUs")
    }

    /// Waits for the watchdog's thread to sweep until `done` holds.
    fn until(what: &str, done: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < deadline, "{what}");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// The core the watchdog reports is refused a place even with the count
    /// at zero, and a core beside it is still admitted.
    #[test]
    fn a_wedged_core_takes_no_new_work() {
        let dog = a_quick_watchdog();
        let admission = Admission::new(&Ceiling::of(&capacity(4, None, None)));
        admission.subscribe(dog.shed());
        let turning = nvs_host::Timers::default();
        let _turning = dog.register(a_cpu(), turning.view());
        let wedged = a_wedged_table(a_task());
        // Registered last, so this thread is the wedged core.
        let _wedged = dog.register(a_cpu(), wedged.view());

        until("the watchdog never reported the wedged core", || {
            dog.shed().wedged() == 1
        });
        assert!(nvs_host::this_core_is_wedged());
        assert!(
            admission.admit().is_none(),
            "a wedged core was given a place"
        );
        assert_eq!(admission.in_flight(), 0);

        let beside = std::thread::scope(|scope| {
            scope
                .spawn(|| admission.admit().is_some())
                .join()
                .expect("the other core panicked")
        });
        assert!(beside, "a core that is not wedged was refused");
    }

    /// One wedged core of two takes half of `max_in_flight` with it.
    #[test]
    fn max_in_flight_drops_by_a_wedged_cores_share() {
        let dog = a_quick_watchdog();
        let admission = Admission::new(&Ceiling::of(&capacity(4, None, None)));
        admission.subscribe(dog.shed());
        let wedged = a_wedged_table(a_task());
        let _wedged = dog.register(a_cpu(), wedged.view());
        // Registered last, so this thread is the core that still turns.
        let turning = nvs_host::Timers::default();
        let _turning = dog.register(a_cpu(), turning.view());

        until("the watchdog never reported the wedged core", || {
            dog.shed().wedged() == 1
        });
        assert_eq!(dog.shed().cores(), 2);
        let first = admission.admit().expect("the first request was refused");
        let second = admission.admit().expect("the second request was refused");
        assert!(
            admission.admit().is_none(),
            "a third request was admitted past half of a ceiling of four"
        );
        assert_eq!(admission.max_in_flight(), 4, "the configured number moved");
        drop((first, second));
    }

    /// A wedged core whose deadline clears gets its share back on the next
    /// sweep, and takes work again.
    #[test]
    fn a_core_that_turns_again_gets_its_share_back() {
        let dog = a_quick_watchdog();
        let admission = Admission::new(&Ceiling::of(&capacity(2, None, None)));
        admission.subscribe(dog.shed());
        let task = a_task();
        let mut wedged = a_wedged_table(task);
        let _wedged = dog.register(a_cpu(), wedged.view());

        until("the watchdog never reported the wedged core", || {
            dog.shed().wedged() == 1
        });
        assert!(
            admission.admit().is_none(),
            "a wedged core was given a place"
        );

        assert!(wedged.disarm(task), "the deadline was not filed");
        until("the core that turned again was still reported", || {
            dog.shed().wedged() == 0
        });
        assert!(!nvs_host::this_core_is_wedged());
        let first = admission
            .admit()
            .expect("a core that turned again was refused");
        let second = admission
            .admit()
            .expect("the share did not return to the ceiling");
        drop((first, second));
    }
}
