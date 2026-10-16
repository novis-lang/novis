//! The async bridge between a guest call and a core's coroutines.
//!
//! `rule:packaging/a-guest-call-yields-on-its-core` runs a guest call as a
//! wasmtime async call that the request's coroutine polls. These checks hold
//! the four things that model rests on: a pending host import parks the
//! coroutine, a second coroutine can enter wasm while the first is parked
//! inside it, an epoch tick yields a long call to the core's other tasks, and
//! a CPU deadline traps a call that keeps yielding. After each of them the
//! thread still runs a fresh call.

#![cfg(feature = "wasm-probe")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use nvs_abi_probe::wasm::{AsyncProbe, CoreLoop, EpochTicker, Finished, GuestState};
use wasmtime::Trap;

/// A CPU deadline no check here reaches.
const NO_DEADLINE: Duration = Duration::from_secs(3600);

/// The host's epoch tick.
const TICK: Duration = Duration::from_millis(1);

/// The order things happened in, across the tasks of one loop.
#[derive(Clone, Default)]
struct Events(Rc<RefCell<Vec<&'static str>>>);

impl Events {
    fn push(&self, event: &'static str) {
        self.0.borrow_mut().push(event);
    }

    fn take(&self) -> Vec<&'static str> {
        self.0.take()
    }
}

/// What a task returns: its result, and how many epoch ticks yielded it.
type Out = anyhow::Result<(u64, u64)>;

fn probe() -> Rc<AsyncProbe> {
    Rc::new(AsyncProbe::pooled(4).expect("async probe"))
}

/// Calls `export` on a fresh store and instance from inside a task.
fn guest_call(
    probe: &AsyncProbe,
    park: &nvs_abi_probe::wasm::Park<'_>,
    state: GuestState,
    export: &str,
    arg: Option<u64>,
) -> Out {
    let mut store = probe.store(state);
    let instance = park.block_on(probe.instantiate(&mut store))?;
    let out = match arg {
        Some(x) => {
            let f = instance.get_typed_func::<(u64,), (u64,)>(&mut store, export)?;
            park.block_on(f.call_async(&mut store, (x,)))?.0
        }
        None => {
            let f = instance.get_typed_func::<(), (u64,)>(&mut store, export)?;
            park.block_on(f.call_async(&mut store, ()))?.0
        }
    };
    Ok((out, store.data().yields))
}

/// One task, alone on a loop, calls `echo` with its gate already open.
fn fresh_call(probe: &Rc<AsyncProbe>) -> u64 {
    let mut core = CoreLoop::new();
    let p = Rc::clone(probe);
    core.spawn(move |park| {
        let state = GuestState::new(NO_DEADLINE);
        state.gate.open();
        guest_call(&p, park, state, "echo", Some(4))
    })
    .expect("spawn");
    let done = core.run().expect("a fresh call finishes");
    done[0].out.as_ref().expect("a fresh call succeeds").0
}

/// A task calls `echo` with its gate closed; a sibling opens the gate.
fn park_on_an_import(probe: &Rc<AsyncProbe>) -> (Vec<Finished<Out>>, Vec<&'static str>) {
    let events = Events::default();
    let state = GuestState::new(NO_DEADLINE);
    let gate = Arc::clone(&state.gate);
    let mut core = CoreLoop::new();

    let (p, ev) = (Rc::clone(probe), events.clone());
    core.spawn(move |park| {
        ev.push("guest call starts");
        let out = guest_call(&p, park, state, "echo", Some(21));
        ev.push("guest call returns");
        out
    })
    .expect("spawn");

    let ev = events.clone();
    core.spawn(move |_| {
        ev.push("sibling opens the gate");
        gate.open();
        Ok((0, 0))
    })
    .expect("spawn");

    (core.run().expect("both tasks finish"), events.take())
}

/// A task spins in wasm until a flag only its sibling sets.
fn yield_at_a_tick(probe: &Rc<AsyncProbe>) -> (Vec<Finished<Out>>, Vec<&'static str>) {
    let events = Events::default();
    let state = GuestState::new(Duration::from_secs(30));
    let flag = Arc::clone(&state.flag);
    let mut core = CoreLoop::new();

    let (p, ev) = (Rc::clone(probe), events.clone());
    core.spawn(move |park| {
        ev.push("long call starts");
        let out = guest_call(&p, park, state, "spin-until-flag", None);
        ev.push("long call returns");
        out
    })
    .expect("spawn");

    let ev = events.clone();
    core.spawn(move |_| {
        ev.push("sibling sets the flag");
        flag.store(true, Ordering::Release);
        Ok((0, 0))
    })
    .expect("spawn");

    (core.run().expect("both tasks finish"), events.take())
}

/// A task loops in wasm forever under a short CPU deadline, beside a sibling.
fn trap_at_the_deadline(
    probe: &Rc<AsyncProbe>,
    budget: Duration,
) -> (
    Vec<Finished<anyhow::Result<u64>>>,
    Vec<&'static str>,
    Duration,
) {
    let events = Events::default();
    let mut core = CoreLoop::new();
    let started = Instant::now();

    let (p, ev) = (Rc::clone(probe), events.clone());
    core.spawn(move |park| {
        let mut store = p.store(GuestState::new(budget));
        let instance = park.block_on(p.instantiate(&mut store))?;
        let forever = instance.get_typed_func::<(), ()>(&mut store, "forever")?;
        let out = park.block_on(forever.call_async(&mut store, ()));
        ev.push("runaway call ends");
        out.map(|()| 0).map_err(|err| {
            assert!(
                store.data().yields > 0,
                "the call yielded before it trapped"
            );
            anyhow::Error::from(err)
        })
    })
    .expect("spawn");

    let ev = events.clone();
    core.spawn(move |_| {
        ev.push("sibling runs");
        Ok(0)
    })
    .expect("spawn");

    let done = core.run().expect("both tasks finish");
    (done, events.take(), started.elapsed())
}

#[test]
fn a_guest_call_parks_its_coroutine_on_a_pending_host_import_and_resumes() {
    let probe = probe();
    let (done, events) = park_on_an_import(&probe);

    let (out, _) = done[0].out.as_ref().expect("the parked call succeeds");
    assert_eq!(
        *out, 42,
        "the import's result reached the guest and came back"
    );
    assert!(
        done[0].parks >= 1,
        "the pending import parked the coroutine"
    );
    assert_eq!(
        events,
        [
            "guest call starts",
            "sibling opens the gate",
            "guest call returns"
        ],
        "the sibling ran while the call was parked inside wasm"
    );
}

#[test]
fn a_second_coroutine_enters_wasm_while_the_first_is_parked_and_both_finish() {
    let probe = probe();
    let events = Events::default();
    let first = GuestState::new(NO_DEADLINE);
    let first_gate = Arc::clone(&first.gate);
    let mut core = CoreLoop::new();

    let (p, ev) = (Rc::clone(&probe), events.clone());
    core.spawn(move |park| {
        ev.push("first call starts");
        let out = guest_call(&p, park, first, "echo", Some(21));
        ev.push("first call returns");
        out
    })
    .expect("spawn");

    let (p, ev) = (Rc::clone(&probe), events.clone());
    core.spawn(move |park| {
        let second = GuestState::new(NO_DEADLINE);
        second.gate.open();
        ev.push("second call starts");
        let out = guest_call(&p, park, second, "echo", Some(5));
        ev.push("second call returns");
        first_gate.open();
        out
    })
    .expect("spawn");

    let done = core.run().expect("both tasks finish");
    assert_eq!(done[0].out.as_ref().expect("first call").0, 42);
    assert_eq!(done[1].out.as_ref().expect("second call").0, 10);
    assert_eq!(
        events.take(),
        [
            "first call starts",
            "second call starts",
            "second call returns",
            "first call returns"
        ],
        "a whole guest call ran on the thread while the first was suspended in wasm"
    );
}

#[test]
fn an_epoch_tick_yields_a_long_guest_call_to_another_coroutine() {
    let probe = probe();
    let _ticker = EpochTicker::start(probe.engine(), TICK);
    let (done, events) = yield_at_a_tick(&probe);

    let (rounds, yields) = done[0].out.as_ref().expect("the long call succeeds");
    assert!(*rounds >= 1);
    assert!(*yields >= 1, "an epoch tick yielded the call");
    assert!(done[0].parks >= 1, "the yield suspended the coroutine");
    assert_eq!(
        events,
        [
            "long call starts",
            "sibling sets the flag",
            "long call returns"
        ],
        "the sibling ran in the middle of the long call"
    );
}

#[test]
fn a_cpu_deadline_traps_a_yielding_guest() {
    let probe = probe();
    let _ticker = EpochTicker::start(probe.engine(), TICK);
    let budget = Duration::from_millis(50);
    let (done, events, elapsed) = trap_at_the_deadline(&probe, budget);

    let err = done[0].out.as_ref().expect_err("the runaway call traps");
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::Interrupt),
        "the deadline trapped the call: {err:?}"
    );
    assert!(done[0].parks >= 1, "the call yielded before the deadline");
    assert!(elapsed >= budget, "the call ran until its deadline");
    assert_eq!(
        events,
        ["sibling runs", "runaway call ends"],
        "the sibling ran while the runaway call was yielding"
    );
}

#[test]
fn a_fresh_call_on_the_thread_succeeds_after_a_park_a_yield_and_a_trap() {
    let probe = probe();
    let _ticker = EpochTicker::start(probe.engine(), TICK);

    let (done, _) = park_on_an_import(&probe);
    assert!(done[0].parks >= 1);
    assert_eq!(fresh_call(&probe), 8, "a fresh call after a park");

    let (done, _) = yield_at_a_tick(&probe);
    assert!(done[0].out.as_ref().is_ok_and(|(_, yields)| *yields >= 1));
    assert_eq!(fresh_call(&probe), 8, "a fresh call after a yield");

    let (done, _, _) = trap_at_the_deadline(&probe, Duration::from_millis(20));
    assert!(done[0].out.is_err());
    assert_eq!(fresh_call(&probe), 8, "a fresh call after a trap");
}
