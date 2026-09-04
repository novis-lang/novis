//! [ADR 0073](../../../docs/adr/0073-scheduled-work-is-config.md) § 5's ticker:
//! the `[[schedule]]` entries a boot accepted, fired beside the accept loop.
//!
//! **A second task, not a second scheduler.** [`serve_on_this_core`](crate::serve::serve_on_this_core)
//! is the shape this follows in every respect that matters: it runs *as* a task
//! on the core the caller already has, it spawns one child per unit of work, and
//! it takes the part only the binary can supply as a parameter. There the
//! parameter is the handler that says which isolate a request is; here it is
//! [`Fires`], which says which isolate an entry's `script` is — turning a path
//! into runnable code is the compiler's, and this crate has none. A ticker with
//! its own scheduler would be a second place a core is driven from, which ADR
//! 0138 and this crate's own docs § *There is no second scheduler* both refuse.
//!
//! # The clock is asked, never counted from
//!
//! Every fire is computed by [`nvs_config::schedule::Cron::next_after`] from
//! **now**, never from the entry's last fire. § 6 is what makes that the whole
//! implementation of two rules at once: a missed interval — the host was down,
//! the machine was suspended, a fire ran long — is *skipped* rather than
//! replayed, and the two DST answers are decided in the one place a civil minute
//! becomes an instant, which is that function and not this module. So there is
//! no accumulated schedule here to drift, and a wall-clock step forwards costs
//! the intervals it stepped over rather than firing them in a burst.
//!
//! The parse is the boot's own parse, for the reason `nvs_config::schedule`'s
//! module doc gives: a scheduler that re-read the expression would be a second
//! dialect, and the two disagreeing produces exactly the failure that module
//! exists to prevent — an entry that booted and fires at the wrong minute, which
//! nothing observes.
//!
//! # What is not armed, and what is not decided yet
//!
//! **A `fleet` entry is not armed** ([`arm`] skips it and says so). § 3 makes a
//! fleet-scoped interval exactly one run *across* the deployment, held by a lease
//! in the shared store, and no lease can be taken anywhere in this tree yet —
//! `Core\Cache`'s wire is `put` and `get` (ADR 0059 § 2) and neither is a
//! compare-and-set. Firing such an entry on each host's own clock is the precise
//! failure § 3's key exists to prevent, so the safe half is to run none of them
//! and name each one at boot.
//!
//! Not here yet, in the order § 5 and § 6 state them: the per-entry `limits` and
//! `grants` sub-caps, which narrow a run's budget and its capabilities and which
//! nothing in this tree can narrow *per isolate* yet; and § 6's `overlap`, whose
//! default is `skip` — a fire is spawned unconditionally today, so an entry whose
//! run outlives its interval overlaps itself. Both are the ticker's questions
//! rather than the caller's, and both belong to this module when they land.

use std::cell::Cell;
use std::io;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::time::Duration;

use jiff::Zoned;
use jiff::tz::TimeZone;
use nvs_config::schedule::{Cron, zone_of};
use nvs_config::tree::Schedule;
use nvs_host::{Completion, Isolate, Waiting, Wake, spawn_child, suspend_current};
use nvs_runtime::host::Woken;
use nvs_runtime::{Ctx, OutputSink, TaskRoot};

/// One entry of the roster, with the expression the boot accepted and the instant it next fires.
///
/// Cloned once per fire, which is what lets the fire's own task outlive the tick that started it:
/// two short strings, a [`Cron`] that is `Copy`, and a [`TimeZone`] whose clone is a handle.
#[derive(Clone, Debug)]
pub struct Armed {
    /// § 1's `name` — the label a log line and a metric carry, and what the script is handed.
    name: String,
    /// § 1's `script`, as the operator wrote it. The boot resolved it against the `script.spawn`
    /// roots and refused one outside them, so what is left here is a path the compiler may take.
    script: String,
    /// § 2's expression, parsed at boot. The one dialect, read once.
    cron: Cron,
    /// § 6's zone: `timezone`, or UTC. Every civil question about this entry is asked in it.
    zone: TimeZone,
    /// The next instant this entry fires, or [`None`] when it has none left inside
    /// [`Cron::next_after`]'s horizon — `30 2 30 2 *` is the honest example. A retired entry stays
    /// in the roster and never fires again, which is the same answer as removing it and keeps the
    /// indices a tick walked stable.
    next: Option<Zoned>,
}

impl Armed {
    /// § 1's `name`, which is also what the run is logged under.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// § 1's `script` — the path [`Fires::isolate`] compiles.
    #[must_use]
    pub fn script(&self) -> &str {
        &self.script
    }

    /// The instant this entry fires next, or [`None`] when it never will again.
    #[must_use]
    pub fn next(&self) -> Option<&Zoned> {
        self.next.as_ref()
    }

    /// Whether `now` has reached this entry's next fire.
    fn due(&self, now: &Zoned) -> bool {
        self.next
            .as_ref()
            .is_some_and(|next| next.timestamp() <= now.timestamp())
    }

    /// The fire has happened: ask for the next one **from the clock**, never from the one that just
    /// ran (§ 6). Answers `false` when there is no next fire, which is the entry retiring.
    fn rearm(&mut self, now: &Zoned) -> bool {
        self.next = self.cron.next_after(&now.with_time_zone(self.zone.clone()));
        self.next.is_some()
    }
}

/// The caller's half of a fire: which isolate an entry's script is, and what its result means.
///
/// Three questions rather than one closure, because they are asked at three different points — one
/// on the fire's own task before it runs, one after it ends, and one from the tick itself — and a
/// single `FnMut` would have to be shared across a task boundary to answer all three.
///
/// **Only `nvs serve` implements this** (§ 5): `nvs run`, `nvs check` and a bundled ADR 0048
/// executable run no schedules, because a schedule is a property of a running deployment rather than
/// of executing a file.
pub trait Fires {
    /// The isolate this entry's `script` is, or [`None`] when it cannot be built.
    ///
    /// Called on the fire's own task, with that task's context — which is what
    /// [`nvs_runtime::script::resolve`] needs and is the reason this takes a `ctx` where the
    /// server's handler does not. A `None` is a compile failure the implementation has already
    /// reported; the ticker adds nothing to it, because the front end's diagnostic is the message
    /// and a second line saying so is noise on every interval.
    ///
    /// § 5's shape, and the implementation owes all of it: `Isolate::new` with the entry's `name` as
    /// the argument the script reads back through `Core\Script::args()`, and [`nvs_host::Output`]
    /// `Capture`, because a scheduled run's output is logged rather than delivered.
    fn isolate(&self, entry: &Armed, ctx: &mut Ctx) -> Option<Isolate>;

    /// § 5's log line: what the run answered, delivered to nobody.
    ///
    /// A top-level `return` is [`Completion::value`], an uncaught throw is its `error`, and what the
    /// script echoed is its captured output (ADR 0088 § 3's table).
    ///
    /// **Read only.** That `value` carries one reference the collector must give up, and the
    /// collector is the ticker: [`Completion::discard_value`] is called on the way out of this,
    /// once, for every fire. An implementation that wants the value in its line renders it here.
    fn ran(&self, entry: &Armed, done: &Completion);

    /// The ticker's own notes — an entry that retired, and nothing else at present.
    fn note(&self, note: &str);
}

/// The roster this process arms, from the entries the boot accepted.
///
/// **`fleet` entries are skipped**, one note each: the module doc § *What is not armed* is why, and
/// it is a refusal to run rather than an omission. An entry that cannot be read at all — no `cron`,
/// an expression outside § 2's dialect, a `timezone` no IANA database knows — is skipped with a note
/// as well, and is unreachable: [`nvs_config::schedule::validate`] refused the boot over every one of
/// those before a socket existed. The note is what makes a hole in that argument visible rather than
/// silent, which is the failure mode a schedule has by construction.
pub fn arm(entries: &[Schedule], now: &Zoned, mut note: impl FnMut(&str)) -> Vec<Armed> {
    let mut armed = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let name = match entry.name.as_deref().map(str::trim) {
            Some(name) if !name.is_empty() => name.to_owned(),
            _ => format!("entry {}", index + 1),
        };
        if entry.scope.as_deref().map(str::trim) == Some("fleet") {
            note(&format!(
                "`{name}` is `scope = \"fleet\"` and is not armed on this host: § 3's lease needs a \
                 shared store that can compare-and-set, and firing it on each host's own clock is \
                 what that key exists to prevent",
            ));
            continue;
        }
        let (Some(expression), Some(script)) = (entry.cron.as_deref(), entry.script.as_deref())
        else {
            note(&format!(
                "`{name}` names no `cron` or no `script` and is not armed"
            ));
            continue;
        };
        let Ok(cron) = Cron::parse(expression) else {
            note(&format!("`{name}` is not a schedule and is not armed"));
            continue;
        };
        let Ok(zone) = zone_of(entry.timezone.as_deref()) else {
            note(&format!(
                "`{name}` names a timezone this build has no rules for"
            ));
            continue;
        };
        let Some(next) = cron.next_after(&now.with_time_zone(zone.clone())) else {
            note(&format!(
                "`{name}` names no instant inside the horizon and is not armed"
            ));
            continue;
        };
        armed.push(Armed {
            name,
            script: script.to_owned(),
            cron,
            zone,
            next: Some(next),
        });
    }
    armed
}

/// § 5's ticker, on the core this is called from: sleep until the soonest fire, fire everything the
/// clock has reached, and ask again.
///
/// `now` is the clock rather than [`Zoned::now`] called inline, for the reason
/// [`serve_on_this_core`](crate::serve::serve_on_this_core) takes `keep_serving`: a loop that reads a
/// global directly can only be tested by waiting for it. `nvs serve` passes `Zoned::now`.
///
/// Returns as soon as the roster has no fire left — an empty roster, or one where every entry has
/// retired — because a task that can never do anything again is one the process should not be kept
/// alive by.
///
/// # Errors
///
/// [`io::ErrorKind::Other`] when this is called off a task, because there is then no parent to hang
/// a fire off and running one on this stack would silently serialize the whole roster behind it.
/// That is the same refusal, for the same reason, that the accept loop makes.
pub fn tick_on_this_core<F>(
    entries: &mut [Armed],
    fires: &Rc<F>,
    now: impl Fn() -> Zoned,
    mut keep_ticking: impl FnMut() -> ControlFlow<()>,
) -> io::Result<()>
where
    F: Fires + 'static,
{
    // Taken once, and it is also the check that this is a task at all: a wake exists exactly when
    // `spawn_child` has a parent to hang a fire off.
    let Some(parent) = Wake::current().map(Rc::new) else {
        return Err(io::Error::other("the ticker must run as a task on a core"));
    };
    let outstanding = Rc::new(Cell::new(0_usize));
    loop {
        let Some(wait) = soonest(entries, &now()) else {
            return Ok(());
        };
        // The core is handed back for the wait, which is the whole reason this is a task: the accept
        // loop beside it keeps serving while a schedule waits out its interval. A cancelled wait is
        // this task being torn down.
        if !wait.is_zero() && matches!(nvs_host::sleep(wait), Woken::Cancelled) {
            break;
        }
        // Read once for the whole pass, so that two entries due in the same minute are answered
        // against one instant rather than against a clock that moved between them.
        let clock = now();
        let due: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.due(&clock))
            .map(|(index, _)| index)
            .collect();
        for index in due {
            // Counted in here rather than inside the fire's own body, so that a run handed over is
            // already outstanding by the time the tail below can look; `Running`'s `Drop` is what
            // counts it back out, and it is a drop rather than a line at the end of the body
            // because a cancelled coroutine is torn down where it parked and never reaches one.
            outstanding.set(outstanding.get() + 1);
            let running = Ran {
                outstanding: Rc::clone(&outstanding),
                parent: Rc::clone(&parent),
            };
            fire(&entries[index], fires, running)?;
            if !entries[index].rearm(&clock) {
                fires.note(&format!(
                    "`{}` has no further fire inside the horizon and will not run again",
                    entries[index].name()
                ));
            }
        }
        if keep_ticking().is_break() {
            break;
        }
    }

    // ADR 0072 § 4, and it is the same tail the accept loop has for the same reason: the fires are
    // this task's children, so a ticker that simply returned would take every run still going down
    // with it — including, on a shutdown, the nightly job that was three minutes into an hour of
    // work. It parks instead, and each fire's guard wakes it on the way out.
    while outstanding.get() > 0 {
        let resumed = suspend_current(Waiting::Parked);
        // Cancelled: the caller is being torn down and the fires go with it, which is § 4's own
        // answer and not something to wait out. Not suspended: there is no core to hand back, so
        // there is no turn in which a fire could ever finish and this would spin.
        if resumed.cancelled() || !resumed.suspended() {
            break;
        }
    }
    Ok(())
}

/// One fire's place in the ticker's tally, given back however that run's task ended.
///
/// A guard rather than a decrement at the end of the body, for the reason `serve`'s `Served` is
/// one: ADR 0072 § 5's cancellation tears a coroutine down where it parked, so the end of the body
/// is exactly the line a cancelled run never reaches.
struct Ran {
    /// The ticker's count of fires spawned and not yet finished.
    outstanding: Rc<Cell<usize>>,
    /// The ticking task, which may be parked on that count reaching zero.
    parent: Rc<Wake>,
}

impl Drop for Ran {
    fn drop(&mut self) {
        self.outstanding.set(self.outstanding.get() - 1);
        // Waking a task that is not parked does nothing, which is the ordinary case: the ticker is
        // usually asleep on the next interval.
        self.parent.wake();
    }
}

/// One fire: a **root** isolate on its own task, and not a child of anything that is serving.
///
/// § 5's rule is that a scheduled run is a second root — ADR 0006's other existing shape, the one an
/// inbound request already is — so what is spawned here is a task with its own fresh [`Ctx`] and an
/// [`Isolate`] with no `inbound`. `OutputSink::Sink` on that context for the reason the accept loop
/// holds it: the run's own output is captured by its isolate and comes back as data (ADR 0088 § 3),
/// so the task around it writes nothing.
///
/// The isolate is run to completion **on that task and not on the tick's**, which is what keeps a
/// run that takes an hour from being the reason the next minute's entry is late.
fn fire<F>(entry: &Armed, fires: &Rc<F>, running: Ran) -> io::Result<()>
where
    F: Fires + 'static,
{
    let fires = Rc::clone(fires);
    let entry = entry.clone();
    let spawned = spawn_child(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |ctx| {
        let _running = running;
        let Some(isolate) = fires.isolate(&entry, ctx) else {
            return;
        };
        match isolate.run(ctx) {
            Ok(mut done) => {
                fires.ran(&entry, &done);
                // Nothing is waiting for a scheduled run (§ 5), so this frame is where the answer
                // stops — and `Completion::value` is a reference copied into *this* ownership
                // rather than a borrow of the arena that has already gone. Dropping it without
                // releasing it leaks the returned graph once per fire, which grows with runs served
                // rather than with runs in flight: AGENTS.md's priority 5 calls that a leak and not
                // a footprint. The queue worker discharges the identical obligation for the
                // identical reason.
                done.discard_value();
            }
            // The argument had no meaning across the boundary, which is a fault in the caller's
            // `isolate` rather than anything the script did. Nothing ran, so there is no completion
            // to log and this is the only place it can be said.
            Err(refused) => fires.note(&format!(
                "`{}` was not run: its argument could not cross: {refused}",
                entry.name()
            )),
        }
    });
    if spawned.is_none() {
        return Err(io::Error::other("the ticker must run as a task on a core"));
    }
    Ok(())
}

/// How long until the soonest fire in the roster, or [`None`] when there is none.
///
/// Saturating at zero rather than answering a negative: an entry the clock has already passed is due
/// now, which is the same answer a wait of zero produces and one fewer state for the loop to hold.
fn soonest(entries: &[Armed], now: &Zoned) -> Option<Duration> {
    entries
        .iter()
        .filter_map(|entry| entry.next.as_ref())
        .map(|next| {
            // `duration_since` and not the `-` operator: the difference of two timestamps is a
            // `Span`, whose calendar units need a reference date to become a wait, and this
            // question has none — it is an absolute gap between two instants.
            let gap = next.timestamp().duration_since(now.timestamp());
            if gap.is_negative() {
                Duration::ZERO
            } else {
                gap.unsigned_abs()
            }
        })
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use nvs_host::{Output, Program, TaskId};
    use nvs_runtime::Value;
    use std::cell::{Cell, RefCell};

    /// One `[[schedule]]` block, as a boot that accepted it hands it over.
    fn entry(name: &str, cron: &str, scope: &str) -> Schedule {
        Schedule {
            name: Some(name.to_owned()),
            cron: Some(cron.to_owned()),
            script: Some("jobs/report.nvs".to_owned()),
            scope: Some(scope.to_owned()),
            ..Schedule::default()
        }
    }

    /// An instant written as RFC 3339, in UTC — the spelling `nvs-config`'s own schedule cases use.
    fn instant(written: &str) -> Zoned {
        written
            .parse::<Timestamp>()
            .expect("the case names a real instant")
            .to_zoned(TimeZone::UTC)
    }

    /// What the ticker did, from the caller's side of [`Fires`].
    struct Watcher {
        /// The task [`Fires::isolate`] was asked on — the fire's own, never the tick's.
        asked_on: Cell<Option<TaskId>>,
        /// Whether the isolate's **own** context carried a request, which is what
        /// `Core\Request` reads and what a root has none of (§ 5, ADR 0012).
        answering: Rc<Cell<Option<bool>>>,
        /// The task the program itself ran on.
        ran_on: Rc<Cell<Option<TaskId>>>,
        /// § 5's log line, one per completed run.
        logged: RefCell<Vec<String>>,
    }

    impl Watcher {
        fn new() -> Self {
            Self {
                asked_on: Cell::new(None),
                answering: Rc::new(Cell::new(None)),
                ran_on: Rc::new(Cell::new(None)),
                logged: RefCell::new(Vec::new()),
            }
        }
    }

    impl Fires for Watcher {
        fn isolate(&self, entry: &Armed, _ctx: &mut Ctx) -> Option<Isolate> {
            self.asked_on.set(nvs_host::current_task());
            let answering = Rc::clone(&self.answering);
            let ran_on = Rc::clone(&self.ran_on);
            let name = entry.name().to_owned();
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                answering.set(Some(child.inbound().is_some()));
                ran_on.set(nvs_host::current_task());
                child
                    .write_output(format!("ran {name}").as_bytes())
                    .expect("a buffer");
                Value::null()
            });
            Some(Isolate::new(program, Value::null(), Output::Capture))
        }

        fn ran(&self, entry: &Armed, done: &Completion) {
            self.logged.borrow_mut().push(format!(
                "{} ok={} {}",
                entry.name(),
                done.ok,
                String::from_utf8_lossy(&done.output)
            ));
        }

        fn note(&self, _note: &str) {}
    }

    /// ADR 0073 § 5: a fire is a **root** isolate on a task of its own — not a child of a
    /// connection, and not the tick's own stack.
    ///
    /// Four claims in one run, because they are four readings of the same fire and a ticker that
    /// answered three of them would still be wrong: the entry runs when the clock reaches its
    /// minute; the isolate is asked for on a task that is not the ticker's, so a run that takes an
    /// hour cannot be why the next minute's entry is late; the isolate's own context carries **no
    /// request**, which is what makes `Core\Request` throw inside it (ADR 0012) and is the whole
    /// difference between this root and the one an inbound request is; and what it echoed comes
    /// back through [`Fires::ran`] as § 5's logged result rather than being delivered anywhere.
    ///
    /// The clock is the parameter and not the wall, which is what keeps this test at a tenth of a
    /// second rather than at the minute a `* * * * *` entry otherwise costs: it stands still at
    /// `00:00:59.9` for the tick's first read and a minute later for every read after, so the wait
    /// the ticker computes is real and short and the pass that follows it is due.
    #[test]
    fn a_schedule_entry_fires_as_a_root_isolate() {
        let base = instant("2026-01-01T00:00:59.9Z");
        let later = instant("2026-01-01T00:01:59.9Z");
        let roster = Rc::new(RefCell::new(arm(
            &[entry("nightly", "* * * * *", "host")],
            &base,
            |note| panic!("nothing to report at boot, and it said: {note}"),
        )));
        assert_eq!(roster.borrow().len(), 1, "the entry is armed");
        assert_eq!(
            roster.borrow()[0]
                .next()
                .map(|next| next.timestamp().to_string()),
            Some("2026-01-01T00:01:00Z".to_owned()),
            "the first fire is the next minute the expression names"
        );

        let watcher = Rc::new(Watcher::new());
        let reads = Rc::new(Cell::new(0_usize));
        let ticking: Rc<Cell<Option<TaskId>>> = Rc::new(Cell::new(None));
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let roster = Rc::clone(&roster);
            let watcher = Rc::clone(&watcher);
            let ticking = Rc::clone(&ticking);
            move |_ctx| {
                ticking.set(nvs_host::current_task());
                tick_on_this_core(
                    &mut roster.borrow_mut(),
                    &watcher,
                    move || {
                        let read = reads.get();
                        reads.set(read + 1);
                        if read == 0 {
                            base.clone()
                        } else {
                            later.clone()
                        }
                    },
                    || ControlFlow::Break(()),
                )
                .expect("the ticker ran as a task");
            }
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        assert_eq!(
            watcher.logged.borrow().as_slice(),
            ["nightly ok=true ran nightly"],
            "the entry fired once and its result was logged rather than delivered"
        );
        assert_eq!(
            watcher.answering.get(),
            Some(false),
            "a scheduled run is a root: no request rides on the isolate"
        );
        let asked_on = watcher.asked_on.get().expect("the fire ran on a task");
        assert_ne!(
            Some(asked_on),
            ticking.get(),
            "the fire is a task of its own and not the tick's stack"
        );
        assert_ne!(
            watcher.ran_on.get(),
            Some(asked_on),
            "the program runs in the isolate's own child task"
        );
        assert_eq!(
            roster.borrow()[0]
                .next()
                .map(|next| next.timestamp().to_string()),
            Some("2026-01-01T00:02:00Z".to_owned()),
            "§ 6: the next fire is asked from the clock, so the minute that passed \
             while the host was busy is skipped rather than replayed"
        );
    }

    /// § 3: a `fleet` entry is not armed on this host, and the boot says so out loud.
    ///
    /// The refusal and not the omission is the point — no lease can be taken anywhere in this tree
    /// yet, and firing a fleet-scoped interval on each host's own clock is the exact failure that
    /// key exists to prevent. A ticker that armed it would look correct on one host.
    #[test]
    fn a_fleet_scoped_entry_is_not_armed_on_this_host() {
        let mut notes = Vec::new();
        let armed = arm(
            &[
                entry("nightly", "@daily", "host"),
                entry("invoices", "@daily", "fleet"),
            ],
            &instant("2026-01-01T00:00:00Z"),
            |note| notes.push(note.to_owned()),
        );

        assert_eq!(
            armed.iter().map(Armed::name).collect::<Vec<_>>(),
            ["nightly"],
            "the host-scoped entry is armed and the fleet-scoped one is not"
        );
        assert_eq!(notes.len(), 1, "and the one that is not is named");
        assert!(
            notes[0].contains("invoices") && notes[0].contains("fleet"),
            "the note names the entry and why: {}",
            notes[0]
        );
    }

    /// § 6, on the entry itself: a fire that was missed is skipped, never replayed.
    ///
    /// Asserted at the boundary the loop actually uses — `rearm` from an instant an hour past the
    /// fire that should have happened — because a ticker that counted forward from the last fire
    /// answers the first row of any sweep identically and only diverges after an outage. Which is
    /// exactly when nobody is reading.
    #[test]
    fn a_missed_interval_is_skipped_rather_than_replayed() {
        let mut armed = arm(
            &[entry("hourly", "0 * * * *", "host")],
            &instant("2026-01-01T00:30:00Z"),
            |note| panic!("nothing to report at boot, and it said: {note}"),
        );
        assert_eq!(
            armed[0].next().map(|next| next.timestamp().to_string()),
            Some("2026-01-01T01:00:00Z".to_owned())
        );

        // The host was down, suspended, or the fire before this one ran long: five fires' worth of
        // clock passed between the arm and the answer.
        assert!(armed[0].rearm(&instant("2026-01-01T05:30:00Z")));

        assert_eq!(
            armed[0].next().map(|next| next.timestamp().to_string()),
            Some("2026-01-01T06:00:00Z".to_owned()),
            "the next fire is the next one after *now*, not the four that were missed"
        );
    }
}
