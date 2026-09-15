//! `rule:config/a-scheduled-run-is-a-root-isolate`'s ticker:
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
//! # Where a `fleet` entry's lease comes from
//!
//! § 3 makes a fleet-scoped interval exactly one run *across* the deployment,
//! held by a lease in the shared store. The store is `Core\Cache`'s shared tier,
//! and **this crate cannot reach it**: `nvs-server` names `hyper`, `nvs-host`,
//! `nvs-config`, `nvs-runtime` and two codecs, and `nvs-stdlib` is not among
//! them. Naming it to get at a key would put the ticker one layer above the rule
//! it implements — an HTTP server resting on the standard library so that a
//! schedule can take a lock — so the lease arrives the way everything else only
//! the binary can supply arrives here: **as a parameter**. [`Leases`] is that
//! parameter and it is [`Fires`]'s sibling in every respect, down to `nvs serve`
//! being its only implementor, because `nvs-cli` is the one crate in this tree
//! that names both this one and `nvs-stdlib`.
//!
//! The ticker therefore holds no store, no client and no backend. It asks two
//! questions about one key — take it for this long, and hold it that long
//! again — and the first one's answer decides whether this host runs the fire.
//! § 3's key is the entry's `name` plus the fire's *scheduled* instant rather
//! than the instant it was noticed, so two hosts whose clocks differ by a second
//! still ask for the same key. The second question is asked on the fire's own
//! task for as long as that run is in flight, which is what keeps the key this
//! host's through a run that outlives the interval it was scheduled for.
//!
//! # The fallback, when there is no lease to take
//!
//! [`arm`] takes `Option<&dyn Leases>`, and a [`None`] leaves every `fleet` entry
//! **unarmed** with one note naming it. What makes it a [`None`] is the binary's
//! to know and not this module's, because the ticker has no type for a store: a
//! tree naming no shared store, and a store that would not answer the boot, both
//! arrive here as the same absence. Firing the entry on each host's own clock
//! instead is the precise failure § 3's key exists to prevent, so the safe half
//! is to run none of them and name each one while an operator is still reading
//! the boot.
//!
//! # An entry never overlaps itself
//!
//! § 6's three modes are all here, and what they share is the count they are
//! asked of: the entry's **own** fires still running, and not the process-wide
//! tally this loop keeps for its teardown — two entries sharing one would let a
//! nightly report suppress an hourly one. A fire that is dropped or held rearms
//! like any other, so a job that runs long falls behind by intervals rather than
//! by copies. [`Overlap`] states each mode; the two that are not `skip` cost the
//! loop one thing each. `queue` makes the wait a [`nvs_host::timer::wait_until`]
//! rather than a `sleep`, because a held fire is waiting on a run *ending* and a
//! sleep re-arms past exactly that wake; `kill` keeps the id of the task each
//! fire was spawned as, which is the handle it cancels and then waits out.
//!
//! # Known gaps
//!
//! 1. **A fire's context carries no configuration, so its `limits` sub-cap has
//!    no ceiling to narrow and its `script` is refused at the door.** [`fire`]
//!    builds the run's root on a bare [`Ctx`], and every question either half
//!    asks of a context with none answers the closed way:
//!    `nvs_runtime::Ctx::narrow_under` takes no sub-cap where there is no
//!    configuration to set it in, and `nvs_runtime::capability::granted` denies
//!    `script.spawn` to a context holding none, which is the refusal
//!    `nvs_cli::serve`'s [`Fires::isolate`] reports. The grant half of a
//!    narrowing lands regardless — it subtracts from a list rather than setting
//!    a directive — so what is missing here is the deployment's own snapshot on
//!    that context and not the narrowing over it.
//!    — owner: m7-server-surface

use std::cell::Cell;
use std::io;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::time::{Duration, Instant};

use jiff::Zoned;
use jiff::tz::TimeZone;
use nvs_config::capability::Cap;
use nvs_config::schedule::{Cron, zone_of};
use nvs_config::tree::{Capabilities, LimitSet, Schedule};
use nvs_host::{Completion, Isolate, TaskId, Waiting, Wake, spawn_child, suspend_current};
use nvs_runtime::host::{Narrowing, Woken};
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
    /// This entry's own fires, spawned and not yet finished — § 6's question, asked per entry
    /// rather than of the process-wide tally the tick keeps for its teardown.
    ///
    /// An [`Rc`] because the [`Ran`] guard that gives a run back outlives the pass that took it,
    /// and a [`Cell`] because a roster and its fires are all on one core. The clone a fire carries
    /// shares this count, which costs nothing and keeps the two from disagreeing.
    running: Rc<Cell<usize>>,
    /// § 6's `overlap`, read once at boot: what this entry does when it is due and still running.
    overlap: Overlap,
    /// § 5's `limits` and `grants`, read at boot into the one shape a spawn site hands a child.
    ///
    /// The ticker's question rather than the implementor's, for `overlap`'s reason: what a run may
    /// spend and which capabilities it may ask for is the entry's configuration, and a [`Fires`]
    /// that built this would be a second place a `[[schedule]]` block is read. Carried on the entry
    /// and cloned onto each fire, because the isolate it is applied to exists only on the fire's
    /// own task.
    ///
    /// Applying it can only ever take room away — `nvs_runtime::Ctx::narrow` leaves the inherited
    /// ceiling standing where an entry asked for a wider one, and the grant list it installs is
    /// asked beside the deployment's `[capabilities]` rather than instead of it — which is why
    /// `rule:config/a-schedule-entry-narrows-only`'s *narrowing only* needs no check here.
    narrowing: Narrowing,
    /// § 3's `scope`, as the one question the ticker asks of it: whether each fire of this entry
    /// has to take a lease before it runs.
    ///
    /// A `bool` and not the word, because `host` and `fleet` are the only two the boot accepts
    /// (`nvs_config::schedule` refuses a third) and everything downstream of that refusal is this
    /// single branch. An entry is only ever armed with this set when [`arm`] was given a
    /// [`Leases`], so a `true` here means a lease can actually be taken.
    fleet: bool,
    /// Whether a fire is held for this entry — `queue`'s single pending run, and never more than
    /// one of them (§ 6).
    ///
    /// The tick is the only reader and the only writer, so this is a plain [`Cell`] rather than a
    /// shared one; the copy a fire's clone carries is dead weight and is never asked.
    held: Cell<bool>,
    /// The task the last fire of this entry was spawned as, which is what `kill` cancels.
    ///
    /// [`None`] until the first fire, and stale rather than cleared once one ends: cancelling a
    /// task that has finished marks nothing ([`nvs_host::cancel_task`] answers `0`), and the mode
    /// only reaches for it when the entry is still running.
    fired_as: Cell<Option<TaskId>>,
}

/// § 6's `overlap`: what a fire does when the previous run of the same entry is still going.
///
/// The mode is the entry's, read at boot and never re-read, because it is configuration rather
/// than state — [`arm`] is the one place a written word becomes one of these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Overlap {
    /// The default: the fire is dropped and named. A job that is already behind gets no closer to
    /// finishing by being started twice.
    #[default]
    Skip,
    /// At most one fire is held, and it starts the moment the run before it ends. A second overlap
    /// while one is already held is dropped and named — the bound is the whole difference between
    /// this and the unbounded pending queue `rule:concurrency/deferred-is-bounded-by-two-directives` refuses to build.
    Queue,
    /// The running isolate is cancelled, its teardown is waited for, and only then does the new run
    /// start. Cancellation runs no user code (`rule:concurrency/cancellation-runs-no-user-code`).
    Kill,
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

    /// Whether a fire of this entry is still going — § 6's question, asked of the entry the tick is
    /// about to fire and of nothing else.
    fn busy(&self) -> bool {
        self.running.get() > 0
    }

    /// Whether `queue` is holding a fire for this entry.
    fn held(&self) -> bool {
        self.held.get()
    }

    /// Whether `now` has reached this entry's next fire.
    fn due(&self, now: &Zoned) -> bool {
        self.next
            .as_ref()
            .is_some_and(|next| next.timestamp() <= now.timestamp())
    }

    /// § 3's key for one fire: the entry's `name`, plus the instant that fire was *scheduled* for.
    ///
    /// The scheduled instant and not the one the tick noticed it at, which is the whole of what
    /// makes this the same key on two hosts: a ticker wakes a few milliseconds after its minute and
    /// two machines never agree on how many, so a key carrying the observed time would be two keys
    /// and both hosts would win. As a UTC timestamp for the same reason — the same fire written in
    /// two zones is one instant, and § 6 already decided the zone question where the civil minute
    /// becomes one.
    fn lease_key(&self, scheduled: &Zoned) -> String {
        format!("nvs.schedule.{}@{}", self.name, scheduled.timestamp())
    }

    /// How long that key is held: to this entry's **next** fire.
    ///
    /// The interval rather than a number, so that § 3's "once per interval" is the lease's own
    /// arithmetic and there is no constant for a deployment to be surprised by. A key that outlived
    /// its interval would leave the following fire unrunnable by anyone; one that expired well
    /// inside it would let a second host take the same fire while the first is still in it, which
    /// is what [`Renewal`] holds the key against for as long as the run is in flight.
    ///
    /// [`Duration::ZERO`] when this entry has no next fire — it is retiring, so there is no
    /// interval to hold and nothing after this to protect.
    fn lease_ttl(&self, scheduled: &Zoned) -> Duration {
        // `duration_since` for the reason `soonest` gives: two instants differ by an absolute gap,
        // and the `-` operator's `Span` would need a reference date to become one.
        self.cron
            .next_after(&scheduled.with_time_zone(self.zone.clone()))
            .map(|next| next.timestamp().duration_since(scheduled.timestamp()))
            .filter(|gap| !gap.is_negative())
            .map_or(Duration::ZERO, |gap| gap.unsigned_abs())
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
/// **Only `nvs serve` implements this** (§ 5): `nvs run`, `nvs check` and a bundled `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
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
    /// script echoed is its captured output (`rule:tooling/echo-always-has-a-sink`'s table).
    ///
    /// **Read only.** That `value` carries one reference the collector must give up, and the
    /// collector is the ticker: [`Completion::discard_value`] is called on the way out of this,
    /// once, for every fire. An implementation that wants the value in its line renders it here.
    fn ran(&self, entry: &Armed, done: &Completion);

    /// The ticker's own notes — an entry that retired, and a fire § 6's `skip` dropped because the
    /// entry's previous run had not finished.
    fn note(&self, note: &str);
}

/// The caller's other half: § 3's lease, as the one question the ticker asks about a shared store.
///
/// One method rather than a store, a client or a connection, because this crate has none of those
/// and must not grow one — the module doc § *Where a `fleet` entry's lease comes from* is that
/// argument, and it is the same one that makes [`Fires`] a trait instead of a compiler in here.
///
/// **Only `nvs serve` implements this**, over `Core\Cache`'s shared tier's set-if-absent, on a
/// connection of its own and under a token naming the process that took the key. A tree with no
/// such store, or one whose store will not answer the boot, is where the binary passes [`None`]
/// and § 3's fallback holds — the module doc's section on it states the rest.
pub trait Leases {
    /// Take the lease named `key`, to be held for `ttl`, and report whether **this** host got it.
    ///
    /// A set-if-absent with an expiry, and nothing else: `true` when the key was absent (or had
    /// expired) and is now this host's, `false` when another host holds it. The implementation
    /// never blocks and never retries — a host that lost this interval has lost it, and § 3 has the
    /// next interval ask again rather than queueing behind the winner.
    ///
    /// **A store that cannot answer this atomically must not implement it.** The whole value of the
    /// method is that two hosts asking at once get two different answers; one that read-then-wrote
    /// would hand both of them `true` under exactly the load that makes it matter, which is the
    /// failure § 3's `scope` exists to prevent and is worse than the [`None`] fallback because it
    /// is silent.
    ///
    /// A failure to reach the store at all is a `false`: not running this interval is the safe
    /// answer, and § 3 already says a partition can leave one unrun.
    fn take(&self, key: &str, ttl: Duration) -> bool;

    /// Hold the lease named `key` for another `ttl`, and report whether it is **still** this
    /// host's.
    ///
    /// Asked on a fire's own task while that run is in flight, so a run outliving the interval it
    /// was scheduled for keeps its key rather than having the store expire it underneath. The
    /// implementation extends the key **only while this host is what holds it** — extending one
    /// that has already passed to another host would hand one interval to two of them, which is
    /// the failure § 3's `scope` exists to prevent.
    ///
    /// A `false` is not an error: § 3 bounds `"fleet"` at at-most-once per interval, and a lease
    /// lost under a live run is the example it gives. The ticker stops asking and says so, and the
    /// run is left alone — ending work that is halfway through is not something a lease answers. A
    /// failure to reach the store is a `false` for [`Leases::take`]'s reason.
    fn renew(&self, key: &str, ttl: Duration) -> bool;
}

/// The roster this process arms, from the entries the boot accepted.
///
/// **`fleet` entries are armed only when `leases` is [`Some`]**, and skipped with one note each
/// otherwise: the module doc § *The fallback, when there is no lease to take* is why, and it is a
/// refusal to run rather than an omission. An entry that cannot be read at all — no `cron`,
/// an expression outside § 2's dialect, a `timezone` no IANA database knows — is skipped with a note
/// as well, and is unreachable: [`nvs_config::schedule::validate`] refused the boot over every one of
/// those before a socket existed. The note is what makes a hole in that argument visible rather than
/// silent, which is the failure mode a schedule has by construction.
///
/// An entry naming an `overlap` § 6 does not is armed under `skip` and noted once, for the same
/// reason and with the same unreachability: the boot refuses an unknown word before this is
/// reached, so the note exists to make a hole in *that* argument visible rather than silent.
pub fn arm(
    entries: &[Schedule],
    now: &Zoned,
    leases: Option<&dyn Leases>,
    mut note: impl FnMut(&str),
) -> Vec<Armed> {
    let mut armed = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let name = match entry.name.as_deref().map(str::trim) {
            Some(name) if !name.is_empty() => name.to_owned(),
            _ => format!("entry {}", index + 1),
        };
        let fleet = entry.scope.as_deref().map(str::trim) == Some("fleet");
        if fleet && leases.is_none() {
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
        // § 6's mode, read once here rather than at each fire: it is configuration, and a tick that
        // re-read a string every minute would be deciding the same thing over and over from a
        // field nothing can change without a reload.
        let overlap = match entry.overlap.as_deref().map(str::trim) {
            None | Some("skip") => Overlap::Skip,
            Some("queue") => Overlap::Queue,
            Some("kill") => Overlap::Kill,
            // Not reachable through a boot that validated the tree, and here for the same reason
            // the refusals above are: a word § 6 does not name falls back to the mode that starts
            // the fewest runs, and says so rather than choosing quietly.
            Some(word) => {
                note(&format!(
                    "`{name}` names `overlap = \"{word}\"`, which § 6 does not, and runs as `skip`"
                ));
                Overlap::Skip
            }
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
            running: Rc::new(Cell::new(0)),
            overlap,
            narrowing: Narrowing {
                limits: entry.limits.as_ref().map(sub_cap).unwrap_or_default(),
                grants: entry.grants.as_ref().map(grant_names),
            },
            fleet,
            held: Cell::new(false),
            fired_as: Cell::new(None),
        });
    }
    armed
}

/// § 5's `limits` table as the pairs a [`Narrowing`] carries: the bare directive name, and the
/// value **as the operator wrote it**.
///
/// The text and not a parsed quantity, because `nvs_config::value::as_written` is then the one
/// reader for both sides of the comparison the child makes — the `512M` a ceiling refuses and the
/// `512M` an entry narrows with are the same string — and a parse here would be a second idea of
/// what `M` means.
///
/// The table is destructured rather than read field by field, so a key added to
/// [`LimitSet`] is a compile error here instead of a sub-cap that silently does not cross.
fn sub_cap(limits: &LimitSet) -> Vec<(String, String)> {
    let LimitSet {
        memory,
        cpu_time,
        wall_time,
        max_tasks,
        max_output,
    } = limits;
    [
        ("memory", memory),
        ("cpu_time", cpu_time),
        ("wall_time", wall_time),
        ("max_tasks", max_tasks),
        ("max_output", max_output),
    ]
    .into_iter()
    .filter_map(|(key, written)| {
        Some((
            key.to_owned(),
            nvs_config::value::as_written(written.as_ref()?),
        ))
    })
    .collect()
}

/// § 5's `grants` table as the capability names a [`Narrowing`] carries.
///
/// Presence is the whole question — [`Cap::grant`] answers [`None`] for a row the entry did not
/// write — because this list only ever subtracts: a name here buys the right to *ask*, and the
/// deployment's own `[capabilities]` is still asked underneath it
/// (`nvs_runtime::capability::granted`). So a scope written beside the name narrows nothing, and
/// `rule:config/a-schedule-entry-narrows-only` is where that reading lives.
fn grant_names(grants: &Capabilities) -> Vec<String> {
    Cap::ALL
        .iter()
        .copied()
        .filter(|cap| cap.grant(grants).is_some())
        .map(|cap| cap.name().to_owned())
        .collect()
}

/// § 5's ticker, on the core this is called from: sleep until the soonest fire, fire everything the
/// clock has reached, and ask again.
///
/// `now` is the clock rather than [`Zoned::now`] called inline, for the reason
/// [`serve_on_this_core`](crate::serve::serve_on_this_core) takes `keep_serving`: a loop that reads a
/// global directly can only be tested by waiting for it. `nvs serve` passes `Zoned::now`.
///
/// `leases` is the same value [`arm`] was given and is asked once per fire of a `fleet` entry — the
/// key § 3 names, held for the interval, and a refusal means another host has this one. It is asked
/// *after* § 6's overlap question and never before it: an entry still running its own previous fire
/// has already lost this interval on this host, and taking the lease only to drop the fire under
/// `skip` would leave the interval unrun by the whole deployment rather than by one machine. An
/// [`Rc`] rather than a borrow because the renewal the key is then held under outlives this call's
/// frame — it runs on the fire's own task.
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
    leases: Option<&Rc<dyn Leases>>,
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
        //
        // Which wait it is depends on what there is to wait for. A held fire is waiting on a run
        // *ending* rather than on a minute arriving, and a run ending is a wake: `wait_until`
        // returns on either, where `sleep` deliberately re-arms past a wake because its caller
        // asked for an instant. So the interval stays the bound and the ending is the answer.
        let woken = if entries.iter().any(Armed::held) {
            nvs_host::timer::wait_until(Instant::now() + wait)
        } else if wait.is_zero() {
            Woken::Elapsed
        } else {
            nvs_host::sleep(wait)
        };
        if matches!(woken, Woken::Cancelled) {
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
            // § 6, and the whole of it: an entry that is due while its own previous run is still
            // going is answered by its mode, and the rearm below happens whichever answer it gets —
            // an entry left due would make the next wait zero and turn one long run into a spin.
            let entry = &entries[index];
            if entry.busy() {
                match entry.overlap {
                    Overlap::Skip => fires.note(&format!(
                        "`{}` is still running its previous fire, so this one is dropped (§ 6's \
                         `overlap = \"skip\"`)",
                        entry.name()
                    )),
                    Overlap::Queue if entry.held() => fires.note(&format!(
                        "`{}` is still running and already holds a fire, so this one is dropped \
                         (§ 6's `overlap = \"queue\"` holds one)",
                        entry.name()
                    )),
                    // Held rather than started, and started by the pass at the top of this loop as
                    // soon as the run ends. One, never a queue: the second overlap above is dropped
                    // instead, because an unbounded backlog in front of a non-durable executor is
                    // what `rule:concurrency/deferred-is-bounded-by-two-directives` refuses to build.
                    Overlap::Queue => entry.held.set(true),
                    Overlap::Kill => {
                        // The run is cancelled at its next safepoint and its teardown *waited for*
                        // before the new one starts, which is the ordering § 6 states: the two must
                        // not be alive together, or `kill` would be `skip` with an extra run. The
                        // wait is a park over the entry's own count, given back by the run's guard
                        // however it ended — `rule:concurrency/cancellation-runs-no-user-code`'s teardown runs no user code, so that
                        // `Drop` is the whole of what there is to wait for.
                        if let Some(task) = entry.fired_as.get() {
                            nvs_host::cancel_task(task);
                        }
                        while entry.busy() {
                            let resumed = suspend_current(Waiting::Parked);
                            // Cancelled: this ticker is being torn down and the fire goes with it.
                            // Not suspended: there is no core, so nothing could ever finish and
                            // this would spin. Either way the entry stays busy and the fire below
                            // is dropped rather than started beside a run that is still there.
                            if resumed.cancelled() || !resumed.suspended() {
                                break;
                            }
                        }
                        if entry.busy() {
                            fires.note(&format!(
                                "`{}` did not tear down, so this fire is dropped rather than \
                                 started beside it (§ 6's `overlap = \"kill\"`)",
                                entry.name()
                            ));
                        } else {
                            start(entry, fires, &outstanding, &parent, None)?;
                        }
                    }
                }
            } else if entry.fleet {
                // § 3, and the key travels with the fire: what this host took is what that run
                // holds for as long as it is in flight. A lease another host has is noted rather
                // than silent, because "the nightly did not run here" is a thing an operator
                // reading one host's log has to be able to tell from a failure — and on a fleet of
                // twenty this is the ordinary line that nineteen of them write.
                if let Some(held) = took_the_lease(entry, leases) {
                    start(entry, fires, &outstanding, &parent, Some(held))?;
                } else {
                    fires.note(&format!(
                        "`{}` is held by another host for this interval and is not run here (§ 3's \
                         lease)",
                        entry.name()
                    ));
                }
            } else {
                start(entry, fires, &outstanding, &parent, None)?;
            }
            if !entries[index].rearm(&clock) {
                fires.note(&format!(
                    "`{}` has no further fire inside the horizon and will not run again",
                    entries[index].name()
                ));
            }
        }
        // § 6's `queue`: a held fire starts the moment the run before it has ended, and this is
        // that moment. The wait above is the only line in this loop that hands the core back, so a
        // run can only have finished while this task was parked in it — which makes this pass the
        // first look after every ending rather than a poll. After the due walk rather than before
        // it, so that a pass finding the entry both free and due answers the clock: the held fire
        // is not lost by that, it starts when *that* run ends, and either order keeps § 6's bound
        // of one held fire per entry. It carries no lease: the pass that held it answered § 6
        // before § 3 was asked, so this host never took a key for the interval it was held from.
        for entry in entries.iter() {
            if entry.held() && !entry.busy() {
                entry.held.set(false);
                start(entry, fires, &outstanding, &parent, None)?;
            }
        }
        if keep_ticking().is_break() {
            break;
        }
    }

    // `rule:concurrency/nothing-is-still-running-when-a-call-returns`, and it is the same tail the accept loop has for the same reason: the fires are
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

/// Hand one fire over: both tallies up, the guard built, the task spawned, and its id kept for a
/// `kill` that may have to cancel it.
///
/// Counted here rather than inside the fire's own body, so that a run handed over is already
/// outstanding by the time the tick's tail can look; [`Ran`]'s `Drop` is what counts it back out,
/// and it is a drop rather than a line at the end of the body because a cancelled coroutine is torn
/// down where it parked and never reaches one.
fn start<F>(
    entry: &Armed,
    fires: &Rc<F>,
    outstanding: &Rc<Cell<usize>>,
    parent: &Rc<Wake>,
    renewal: Option<Renewal>,
) -> io::Result<()>
where
    F: Fires + 'static,
{
    outstanding.set(outstanding.get() + 1);
    entry.running.set(entry.running.get() + 1);
    let running = Ran {
        outstanding: Rc::clone(outstanding),
        mine: Rc::clone(&entry.running),
        parent: Rc::clone(parent),
    };
    entry
        .fired_as
        .set(Some(fire(entry, fires, running, renewal)?));
    Ok(())
}

/// One fire's place in the ticker's tally, given back however that run's task ended.
///
/// A guard rather than a decrement at the end of the body, for the reason `serve`'s `Served` is
/// one: `rule:concurrency/cancellation-runs-no-user-code`'s cancellation tears a coroutine down where it parked, so the end of the body
/// is exactly the line a cancelled run never reaches.
struct Ran {
    /// The ticker's count of fires spawned and not yet finished.
    outstanding: Rc<Cell<usize>>,
    /// This fire's own entry's count, which is § 6's question and is the same count
    /// [`Armed::busy`] reads on the next pass.
    mine: Rc<Cell<usize>>,
    /// The ticking task, which may be parked on that count reaching zero.
    parent: Rc<Wake>,
}

impl Drop for Ran {
    fn drop(&mut self) {
        self.outstanding.set(self.outstanding.get() - 1);
        self.mine.set(self.mine.get() - 1);
        // Waking a task that is not parked does nothing, which is the ordinary case: the ticker is
        // usually asleep on the next interval.
        self.parent.wake();
    }
}

/// One fire: a **root** isolate on its own task, and not a child of anything that is serving.
///
/// § 5's rule is that a scheduled run is a second root — `rule:security/isolate-shares-nothing`'s other existing shape, the one an
/// inbound request already is — so what is spawned here is a task with its own fresh [`Ctx`] and an
/// [`Isolate`] with no `inbound`. `OutputSink::Sink` on that context for the reason the accept loop
/// holds it: the run's own output is captured by its isolate and comes back as data (`rule:tooling/echo-always-has-a-sink`),
/// so the task around it writes nothing.
///
/// The isolate is run to completion **on that task and not on the tick's**, which is what keeps a
/// run that takes an hour from being the reason the next minute's entry is late.
///
/// `renewal` is the key a `fleet` entry's fire took, and is [`None`] for every other fire. It is
/// started here rather than by the caller because this is the task § 3's renewal is held for: the
/// timer is a child of it, so the run ending is what stops the key being extended.
fn fire<F>(
    entry: &Armed,
    fires: &Rc<F>,
    running: Ran,
    renewal: Option<Renewal>,
) -> io::Result<TaskId>
where
    F: Fires + 'static,
{
    let fires = Rc::clone(fires);
    let entry = entry.clone();
    let spawned = spawn_child(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |ctx| {
        let _running = running;
        if let Some(renewal) = renewal {
            renewal.keep(&fires, entry.name());
        }
        let Some(isolate) = fires.isolate(&entry, ctx) else {
            return;
        };
        // § 5's sub-caps, on the isolate and not on the context the implementor was handed: what a
        // narrowing means is *tighter than what remains of the parent*, and the parent is this
        // task's own root. `Isolate::start` applies it to the child's context before its first
        // statement, which is the only point at which that context exists.
        // `rule:observability/four-kinds-become-a-span`'s root for a run that
        // answers no request, read while the isolate is still in hand for the
        // reason `nvs_host::Isolate::recorded_trace` gives. The draw is the
        // implementor's, because the head sample is a key off the tree and this
        // loop holds none.
        let recording = isolate.recorded_trace();
        match isolate.narrowed_by(entry.narrowing.clone()).run(ctx) {
            Ok(mut done) => {
                fires.ran(&entry, &done);
                // The entry's name is what the root span is *of*, which is a
                // scheduled run's whole request line: it has no method and no
                // path, and the name is what an operator wrote and what every
                // other record of this fire already says.
                if let Some(trace) = &recording {
                    crate::trace::record(entry.name(), trace, &done.trace);
                }
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
    // The id is `kill`'s handle on this run and nothing else's: § 6's other two modes never reach
    // for it, and it is answered here because this is where the task exists.
    spawned.ok_or_else(|| io::Error::other("the ticker must run as a task on a core"))
}

/// § 3's lease for the fire this entry is due for: the key this host took, or [`None`] when the
/// interval is another host's.
///
/// The whole of the fleet decision, in one place the tick reaches and a test can reach without a
/// core — which is why it is a function rather than four lines inside the due walk. The two hosts a
/// fleet is are two rosters over one store, and that is exactly what asking this twice is.
///
/// Both [`None`]s above the question are unreachable and both refuse the fire, which is the safe
/// direction: a `fleet` entry is only armed when [`arm`] was given a [`Leases`], and an entry with
/// no next fire is never due. A refusal costs an unrun interval; a key handed back by accident
/// costs a run on every host, which is the failure the key exists to prevent.
fn took_the_lease(entry: &Armed, leases: Option<&Rc<dyn Leases>>) -> Option<Renewal> {
    let (Some(leases), Some(scheduled)) = (leases, entry.next.as_ref()) else {
        return None;
    };
    let held = Renewal {
        key: entry.lease_key(scheduled),
        ttl: entry.lease_ttl(scheduled),
        leases: Rc::clone(leases),
    };
    leases.take(&held.key, held.ttl).then_some(held)
}

/// § 3's renewal: the key one fire took, and what it takes to go on holding it.
///
/// Built by [`took_the_lease`] and carried onto the fire's own task, because that is the task whose
/// life the key is held for — the ticker's own would hold it long past the run, and a task of its
/// own would outlive a run that was cancelled. `nvs_host`'s task tree is what makes that exact:
/// [`Renewal::keep`] spawns a child of the fire, and a task that ends cancels its children, so the
/// run ending is the whole of what stops the timer.
struct Renewal {
    /// The key [`Armed::lease_key`] derived for this fire, and the one the renewals name.
    key: String,
    /// [`Armed::lease_ttl`]'s interval, asked for again at each renewal, so the key is always held
    /// one whole interval from now. A host that dies mid-run therefore releases it by expiry
    /// inside the bound § 3 states rather than blocking a later interval forever.
    ttl: Duration,
    /// The store the question goes to, as an [`Rc`] because this outlives the frame [`arm`] and
    /// the tick were called from.
    leases: Rc<dyn Leases>,
}

impl Renewal {
    /// How often the key is extended: half of what it is held for.
    ///
    /// A fraction rather than a constant, for [`Armed::lease_ttl`]'s reason — the interval is the
    /// only number § 3 has, and a second one here would be a thing a deployment could be surprised
    /// by. Half of it leaves a whole renewal's worth of slack for a store that answers slowly,
    /// which asking at the expiry itself would not.
    fn every(&self) -> Duration {
        self.ttl / 2
    }

    /// Hold the key for as long as this fire runs, on a task of the fire's own.
    ///
    /// Spawned as a child of the caller, which is the fire's task: the run ending cancels it, and
    /// so does the cancellation § 6's `kill` sends, so there is no state to give back and no guard
    /// to write. A key this host no longer holds stops the timer and is one line an operator sees;
    /// the run itself is left alone, because § 3 bounds `fleet` at at-most-once and ending work
    /// that is halfway through is not what a lease answers.
    fn keep<F>(self, fires: &Rc<F>, name: &str)
    where
        F: Fires + 'static,
    {
        let every = self.every();
        if every.is_zero() {
            // [`Armed::lease_ttl`] answers zero for an entry with no next fire: it is retiring, so
            // there is no interval to hold and nothing after this run to protect.
            return;
        }
        let fires = Rc::clone(fires);
        let name = name.to_owned();
        // `TaskRoot::Request` for the fire's own reason: this is one run's housekeeping, and a
        // fault in it belongs to that run rather than retiring the core serving beside it. The
        // [`None`] this answers off a task is unreachable — the caller is a task by construction.
        let _timer = spawn_child(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            loop {
                if matches!(nvs_host::sleep(every), Woken::Cancelled) {
                    return;
                }
                if !self.leases.renew(&self.key, self.ttl) {
                    fires.note(&format!(
                        "`{name}` is still running here, but its lease is no longer this host's \
                         and was not renewed (§ 3's at-most-once bound)"
                    ));
                    return;
                }
            }
        });
    }
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
    use nvs_config::capability::Scope;
    use nvs_host::{Output, Program, TaskId};
    use nvs_runtime::Value;
    use std::cell::{Cell, RefCell};
    use std::sync::Arc;

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

    /// One `[[schedule]]` block carrying § 5's two optional tables, written the way an operator
    /// writes them.
    ///
    /// Through TOML rather than through the struct, because the dotted key in
    /// `grants = {net.connect = [...]}` is § 5's own spelling and the deserializer is the one thing
    /// that turns it into the `[capabilities.net] connect` the rest of this tree reads. A case
    /// building `Capabilities` by hand would be asserting against its own idea of that mapping.
    fn sub_capped(name: &str, tables: &str) -> Schedule {
        format!(
            "name = \"{name}\"\ncron = \"* * * * *\"\nscript = \"jobs/report.nvs\"\n\
             scope = \"host\"\n{tables}"
        )
        .parse::<toml::Table>()
        .expect("the case writes valid TOML")
        .try_into()
        .expect("the case writes a `[[schedule]]` block this tree has")
    }

    /// The deployment those entries narrow: a ceiling on the heap, one host an outbound call may
    /// reach, and every endpoint a bind may name.
    ///
    /// `process.exec` is absent on purpose — it is what an entry below asks for and must not get.
    fn deployment() -> Arc<nvs_config::Snapshot> {
        let table: toml::Table = "[limits]\nmemory = \"64M\"\n\n\
             [capabilities.net]\nconnect = [\"reports.internal\"]\nlisten = true\n"
            .parse()
            .expect("the case writes valid TOML");
        Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes blocks this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
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
        /// How long each fire's program holds its task before returning. Zero for a run that ends
        /// inside the pass that started it; anything else outlives the next interval, which is the
        /// only way to ask § 6's question.
        holds: Duration,
        /// How many fires were asked for an isolate — a run § 6's `skip` dropped never gets here,
        /// which is what makes this different from counting the runs that *finished*.
        started: Cell<usize>,
        /// The task [`Fires::isolate`] was asked on — the fire's own, never the tick's.
        asked_on: Cell<Option<TaskId>>,
        /// Whether the isolate's **own** context carried a request, which is what
        /// `Core\Request` reads and what a root has none of (§ 5, `rule:statements/no-host-populated-variables`).
        answering: Rc<Cell<Option<bool>>>,
        /// The task the program itself ran on.
        ran_on: Rc<Cell<Option<TaskId>>>,
        /// § 5's log line, one per completed run.
        logged: RefCell<Vec<String>>,
        /// [`Fires::note`]'s lines — a retired entry, and a fire `skip` dropped.
        noted: RefCell<Vec<String>>,
        /// The deployment every fire runs under, put on the fire's own context before its isolate
        /// is built. [`None`] for a case whose subject is not § 5's sub-caps, which is every case
        /// but one: a context holding no configuration has no ceiling for a `limits` table to
        /// narrow and grants nothing at all, so a fake that always installed one would be answering
        /// a question those cases do not ask.
        configured: Option<Arc<nvs_config::Snapshot>>,
        /// The ceiling the fire's **own** context resolved out of that deployment — the number a
        /// sub-cap is tighter than, read where the parent is still reachable.
        ceiling: Cell<usize>,
        /// What each fire's own context said about § 5's two tables, in the order the fires ran.
        narrowed: Rc<RefCell<Vec<(String, Narrowed)>>>,
    }

    /// What a fire's context answers about § 5's sub-caps, read inside the run itself.
    ///
    /// Read from the child rather than from the [`Armed`] it came off, because the claim is about
    /// what the run *spends and may ask for* and not about what the ticker carried: a narrowing
    /// built correctly and applied to nothing looks identical from the entry's side.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Narrowed {
        /// The ceiling this run is held to, and `0` for a child that took none of its own and
        /// spends the tree's budget instead (`rule:security/isolate-budget-is-the-trees`) — which
        /// is what a fire writing no `limits` does.
        memory: usize,
        /// `net.connect` for a host the deployment names — the one capability an entry below asks
        /// to keep.
        connect: bool,
        /// `net.listen`, which the deployment grants and no entry names, so it is what a narrowing
        /// takes away.
        listen: bool,
        /// `process.exec`, which the deployment grants nobody and an entry below asks for anyway.
        exec: bool,
    }

    /// One capability by the name `nvs.toml` grants it under.
    fn cap(name: &str) -> Cap {
        Cap::parse(name).expect("`nvs.toml` grants a capability under this name")
    }

    impl Watcher {
        fn new() -> Self {
            Self {
                holds: Duration::ZERO,
                started: Cell::new(0),
                asked_on: Cell::new(None),
                answering: Rc::new(Cell::new(None)),
                ran_on: Rc::new(Cell::new(None)),
                logged: RefCell::new(Vec::new()),
                noted: RefCell::new(Vec::new()),
                configured: None,
                ceiling: Cell::new(0),
                narrowed: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }

    impl Fires for Watcher {
        fn isolate(&self, entry: &Armed, ctx: &mut Ctx) -> Option<Isolate> {
            self.asked_on.set(nvs_host::current_task());
            self.started.set(self.started.get() + 1);
            // On the fire's own context, which is the parent a sub-cap is measured against: the
            // isolate below inherits it, and `Ctx::narrow` then asks what remains of *this* budget.
            if let Some(snapshot) = &self.configured {
                ctx.set_config(Arc::clone(snapshot));
                self.ceiling.set(ctx.memory_limit());
            }
            let answering = Rc::clone(&self.answering);
            let ran_on = Rc::clone(&self.ran_on);
            let narrowed = Rc::clone(&self.narrowed);
            let name = entry.name().to_owned();
            let holds = self.holds;
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                answering.set(Some(child.inbound().is_some()));
                ran_on.set(nvs_host::current_task());
                narrowed.borrow_mut().push((
                    name.clone(),
                    Narrowed {
                        memory: child.memory_limit(),
                        connect: nvs_runtime::capability::granted(
                            child,
                            cap("net.connect"),
                            Scope::Host("reports.internal"),
                        ),
                        listen: nvs_runtime::capability::granted(
                            child,
                            cap("net.listen"),
                            Scope::Endpoint(
                                "127.0.0.1:8080"
                                    .parse()
                                    .expect("the case names an endpoint"),
                            ),
                        ),
                        exec: nvs_runtime::capability::granted(
                            child,
                            cap("process.exec"),
                            Scope::Unscoped,
                        ),
                    },
                ));
                if !holds.is_zero() {
                    // The core is handed back, so the tick beside this one keeps its own intervals
                    // while this run is outstanding — which is the state § 6 is about.
                    nvs_host::sleep(holds);
                }
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

        fn note(&self, note: &str) {
            self.noted.borrow_mut().push(note.to_owned());
        }
    }

    /// `rule:config/a-scheduled-run-is-a-root-isolate`: a fire is a **root** isolate on a task of its own — not a child of a
    /// connection, and not the tick's own stack.
    ///
    /// Four claims in one run, because they are four readings of the same fire and a ticker that
    /// answered three of them would still be wrong: the entry runs when the clock reaches its
    /// minute; the isolate is asked for on a task that is not the ticker's, so a run that takes an
    /// hour cannot be why the next minute's entry is late; the isolate's own context carries **no
    /// request**, which is what makes `Core\Request` throw inside it (`rule:statements/no-host-populated-variables`) and is the whole
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
            None,
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
                    None,
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

    /// `rule:config/a-schedule-entry-narrows-only`: an entry's `limits` and `grants` narrow the run
    /// they fire and can never widen it.
    ///
    /// Three entries under one deployment, in one pass, because *narrowing only* is a claim about
    /// the pair and each half alone passes on a plausible bug: an entry that asked for less, one
    /// that asked for more, and one that asked for nothing, which is the baseline the other two are
    /// read against — a ticker that applied nothing and one that applied everything both look right
    /// against a single entry.
    ///
    /// What the run is asked is what it may spend and what it may ask a capability for, read inside
    /// the fire's own child context through the same door a program reaches
    /// (`nvs_runtime::capability::granted`), and never the [`Narrowing`] the entry carries. The
    /// sub-cap is asserted against the unnarrowed fire rather than against a byte count, for the
    /// reason `crates/nvs-runtime/tests/tree_budget.rs`'s crossing case gives: what a ceiling
    /// resolves to is the configuration's arithmetic, and what this case is about is that the
    /// narrowing arrived at all.
    #[test]
    fn a_schedule_entrys_limits_and_grants_narrow_its_run_and_never_widen_it() {
        let base = instant("2026-01-01T00:00:59.9Z");
        let later = instant("2026-01-01T00:01:59.9Z");
        let roster = Rc::new(RefCell::new(arm(
            &[
                sub_capped("plain", ""),
                sub_capped(
                    "narrowed",
                    "limits = {memory = \"1M\"}\ngrants = {net.connect = [\"reports.internal\"]}\n",
                ),
                sub_capped(
                    "widened",
                    "limits = {memory = \"512M\"}\ngrants = {process.exec = true}\n",
                ),
            ],
            &base,
            None,
            |note| panic!("nothing to report at boot, and it said: {note}"),
        )));
        let watcher = Rc::new(Watcher {
            configured: Some(deployment()),
            ..Watcher::new()
        });
        let reads = Rc::new(Cell::new(0_usize));
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let roster = Rc::clone(&roster);
            let watcher = Rc::clone(&watcher);
            move |_ctx| {
                tick_on_this_core(
                    &mut roster.borrow_mut(),
                    &watcher,
                    None,
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

        let ran = watcher.narrowed.borrow();
        assert_eq!(ran.len(), 3, "every entry fired: {ran:?}");
        let fired = |name: &str| {
            ran.iter()
                .find(|(entry, _)| entry == name)
                .map(|(_, seen)| *seen)
                .expect("the entry fired")
        };
        let (plain, narrowed, widened) = (fired("plain"), fired("narrowed"), fired("widened"));

        assert_eq!(
            (plain.connect, plain.listen, plain.exec),
            (true, true, false),
            "an entry writing neither table runs under the deployment's own grants, whole"
        );
        assert_eq!(
            (narrowed.connect, narrowed.listen),
            (true, false),
            "`grants` holds the run to the capabilities it named and drops the rest"
        );
        let ceiling = watcher.ceiling.get();
        assert!(
            ceiling > 0,
            "the deployment's own `[limits] memory` is in force on the fire these narrow: {ceiling}"
        );
        assert!(
            plain.memory > 0 && plain.memory <= ceiling,
            "an entry writing neither table spends the tree's budget rather than one of its own, \
             so what holds it is what remained of the deployment's ceiling \
             (`rule:security/isolate-budget-is-the-trees`): {} against {ceiling}",
            plain.memory
        );
        assert!(
            narrowed.memory > 0 && narrowed.memory < ceiling,
            "`limits` holds the same run to a ceiling of its own, tighter than the deployment's: \
             {} against {ceiling}",
            narrowed.memory
        );
        assert_eq!(
            (widened.connect, widened.listen, widened.exec),
            (false, false, false),
            "and neither table widens anything: a capability the deployment withheld is not \
             granted by an entry naming it, and naming it dropped the two the deployment did grant"
        );
        assert!(
            widened.memory <= ceiling && widened.memory > narrowed.memory,
            "a `limits` above the deployment's ceiling leaves that ceiling standing — what remains \
             of it at the fire, and never the `512M` the entry asked for: {} against {ceiling}",
            widened.memory
        );
    }

    /// A shared store that *can* compare-and-set, standing in for the tier `Core\Cache::shared`
    /// will be once `rule:concurrency/a-cached-value-is-copied-across-the-boundary`'s wire has a set-if-absent.
    ///
    /// A key to the instant it expires at, plus a clock the case moves by hand — a lease's whole
    /// observable behaviour, and small enough that the cases below are about the ticker rather than
    /// about a fake. The clock is seconds and moved explicitly because a lease's bound is an
    /// interval: waiting one out would make these cases take a day.
    ///
    /// **One store, two rosters** is what makes these two hosts rather than one machine asked
    /// twice. That is the whole of the fleet in a unit test, and it is faithful for the reason § 3
    /// gives — coordination is the store's, and a host contributes nothing to it but the question.
    struct Store {
        /// Each key held, with the second on this store's clock that it expires at.
        held: RefCell<Vec<(String, u64)>>,
        /// This store's clock, in seconds from arming.
        now: Cell<u64>,
        /// Every `take`, in order: the key and how long it was asked to be held for. What proves
        /// two hosts asked for *one* key, which is the half a fake could otherwise pass by handing
        /// out `true` once per key.
        asked: RefCell<Vec<(String, Duration)>>,
        /// Every `renew`, in order, read the same way. A renewal names the key it extends, so this
        /// is what says a fire went on holding the key it took rather than some other one.
        renewed: RefCell<Vec<(String, Duration)>>,
    }

    impl Store {
        fn new() -> Self {
            Self {
                held: RefCell::new(Vec::new()),
                now: Cell::new(0),
                asked: RefCell::new(Vec::new()),
                renewed: RefCell::new(Vec::new()),
            }
        }
    }

    impl Leases for Store {
        fn take(&self, key: &str, ttl: Duration) -> bool {
            self.asked.borrow_mut().push((key.to_owned(), ttl));
            let now = self.now.get();
            let until = now + ttl.as_secs();
            let mut held = self.held.borrow_mut();
            match held.iter_mut().find(|(name, _)| name == key) {
                // Held by someone, and still inside its expiry: this host loses the interval.
                Some(entry) if entry.1 > now => false,
                // Expired, so it is takeable again — § 3's "a host that dies mid-run releases it by
                // expiry rather than blocking the next interval forever".
                Some(entry) => {
                    entry.1 = until;
                    true
                }
                None => {
                    held.push((key.to_owned(), until));
                    true
                }
            }
        }

        /// Extend a key this store still holds, and refuse one it does not.
        ///
        /// Presence rather than a token, because this fake has no second holder to be confused
        /// with: the token half of § 3's renewal is the store's own question and is asserted where
        /// the wire is, in `nvs-stdlib`'s lease cases. What this answers is the half the ticker
        /// asks — is the key still there to go on holding.
        fn renew(&self, key: &str, ttl: Duration) -> bool {
            self.renewed.borrow_mut().push((key.to_owned(), ttl));
            let until = self.now.get() + ttl.as_secs();
            let mut held = self.held.borrow_mut();
            match held.iter_mut().find(|(name, _)| name == key) {
                Some(entry) => {
                    entry.1 = until;
                    true
                }
                None => false,
            }
        }
    }

    /// One second short of a day, and a day: the two sides of the lease's bound.
    const DAY: u64 = 24 * 60 * 60;

    /// § 3: one fire across the deployment, decided by which host took the lease.
    ///
    /// Two rosters over one store is the fleet, and all three claims are read off that: both hosts
    /// arm the entry, exactly one of them takes the fire, and — the half a fake would otherwise
    /// hide — they asked for the **same key**, the entry's name plus the instant the fire was
    /// scheduled for. A ticker keying on the instant it noticed the fire at instead would pass the
    /// first two assertions on every run and fire on every host in production, because two machines
    /// never wake in the same millisecond.
    #[test]
    fn a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease() {
        let store = Rc::new(Store::new());
        let leases: Rc<dyn Leases> = Rc::clone(&store) as Rc<dyn Leases>;
        let entries = [entry("invoices", "@daily", "fleet")];
        let boot = instant("2026-01-01T12:00:00Z");
        let one = arm(&entries, &boot, Some(&*leases), |note| {
            panic!("nothing to report at boot, and it said: {note}")
        });
        let two = arm(&entries, &boot, Some(&*leases), |note| {
            panic!("nothing to report at boot, and it said: {note}")
        });

        assert_eq!(
            (one.len(), two.len()),
            (1, 1),
            "a fleet entry is armed on both hosts once a lease can be taken at all"
        );
        assert_eq!(
            [
                took_the_lease(&one[0], Some(&leases)).is_some(),
                took_the_lease(&two[0], Some(&leases)).is_some(),
            ],
            [true, false],
            "and exactly one of them runs the fire"
        );

        let asked = store.asked.borrow();
        assert_eq!(
            asked.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            [
                "nvs.schedule.invoices@2026-01-02T00:00:00Z",
                "nvs.schedule.invoices@2026-01-02T00:00:00Z"
            ],
            "both asked for one key: the name, and the instant the fire was scheduled for"
        );
        assert_eq!(
            asked[0].1,
            Duration::from_secs(DAY),
            "held for the interval, so `once per interval` is the lease's own arithmetic"
        );
    }

    /// § 3's expiry: a lease nobody released is taken by another host once its interval is out.
    ///
    /// The bound is asserted on both sides — a second short of the day it is still the first host's,
    /// and at the day it is not — because a store that expired a lease early would let a second host
    /// run a fire the first is still inside, and one that never expired it would leave every
    /// following interval unrun by the whole deployment the first time a host died mid-run. Only
    /// the pair of assertions separates those; either alone passes on a plausible-looking bug.
    #[test]
    fn a_fleet_scoped_entry_whose_lease_expired_is_taken_by_another_host() {
        let store = Rc::new(Store::new());
        let leases: Rc<dyn Leases> = Rc::clone(&store) as Rc<dyn Leases>;
        let entries = [entry("invoices", "@daily", "fleet")];
        let boot = instant("2026-01-01T12:00:00Z");
        let one = arm(&entries, &boot, Some(&*leases), |note| {
            panic!("nothing to report at boot, and it said: {note}")
        });
        let two = arm(&entries, &boot, Some(&*leases), |note| {
            panic!("nothing to report at boot, and it said: {note}")
        });

        assert!(
            took_the_lease(&one[0], Some(&leases)).is_some(),
            "the first host takes the lease"
        );
        assert!(
            took_the_lease(&two[0], Some(&leases)).is_none(),
            "and while it is held the second host does not run the fire"
        );

        // The winner dies mid-run: nothing hands the lease back, so the TTL is the only thing that
        // ends it.
        store.now.set(DAY - 1);
        assert!(
            took_the_lease(&two[0], Some(&leases)).is_none(),
            "a second short of the interval it is still the first host's"
        );
        store.now.set(DAY);
        assert!(
            took_the_lease(&two[0], Some(&leases)).is_some(),
            "and once it has expired the other host takes it"
        );
    }

    /// § 3's renewal: the key a fire took stays this host's for as long as that run is in flight,
    /// and stops being extended the moment it ends.
    ///
    /// Both halves, because either one alone passes on a bug that ships. A renewal that never
    /// fired would let the store expire the key under a run that outlives the interval it was
    /// scheduled for; one that outlived the run would hold the key against a deployment that no
    /// longer has anything running under it. The keys are read back off the store rather than
    /// asserted from the case, which is what proves the timer extends *the key this fire took* —
    /// a renewal naming anything else holds nothing and would still count.
    ///
    /// The interval is shortened after [`took_the_lease`] answered, so this costs a tenth of a
    /// second rather than the day a `@daily` entry's lease is really held for. Nothing else is a
    /// stand-in: it is [`fire`] itself on a real core, and what stops the timer is the run's task
    /// ending, exactly as in a server.
    #[test]
    fn a_fleet_lease_is_renewed_while_its_run_is_in_flight() {
        let store = Rc::new(Store::new());
        let leases: Rc<dyn Leases> = Rc::clone(&store) as Rc<dyn Leases>;
        let armed = arm(
            &[entry("invoices", "@daily", "fleet")],
            &instant("2026-01-01T12:00:00Z"),
            Some(&*leases),
            |note| panic!("nothing to report at boot, and it said: {note}"),
        );
        let mut renewal =
            took_the_lease(&armed[0], Some(&leases)).expect("this host took the lease");
        renewal.ttl = Duration::from_millis(40);
        let watcher = Rc::new(Watcher {
            holds: Duration::from_millis(130),
            ..Watcher::new()
        });
        let during = Rc::new(Cell::new(0_usize));

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let store = Rc::clone(&store);
            let watcher = Rc::clone(&watcher);
            let during = Rc::clone(&during);
            move |_ctx| {
                // What `start` counts before it hands a fire over, done here because this case
                // reaches past it: `Ran`'s `Drop` gives both of them back.
                let outstanding = Rc::new(Cell::new(1_usize));
                armed[0].running.set(1);
                let running = Ran {
                    outstanding: Rc::clone(&outstanding),
                    mine: Rc::clone(&armed[0].running),
                    parent: Rc::new(Wake::current().expect("the case runs as a task")),
                };
                fire(&armed[0], &watcher, running, Some(renewal)).expect("the fire is a task");
                // The fire is this task's child, so this one outlives it — the same park the
                // ticker's own tail makes, woken by that guard on the way out.
                while outstanding.get() > 0 {
                    let resumed = suspend_current(Waiting::Parked);
                    if resumed.cancelled() || !resumed.suspended() {
                        break;
                    }
                }
                during.set(store.renewed.borrow().len());
                // Three more of the shortened interval, with the run over: whatever the store sees
                // now is what a timer outliving its own fire would have written.
                nvs_host::sleep(Duration::from_millis(60));
            }
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let renewed = store.renewed.borrow();
        assert!(
            during.get() >= 2,
            "the key was held again while the run was in flight, and it was asked {} time(s)",
            during.get()
        );
        assert_eq!(
            renewed.len(),
            during.get(),
            "and the run ending is what stops it: nothing was renewed after the fire's task went"
        );
        assert!(
            renewed
                .iter()
                .all(|(key, _)| key == "nvs.schedule.invoices@2026-01-02T00:00:00Z"),
            "every renewal named the key this fire took: {renewed:?}"
        );
        assert_eq!(
            renewed[0].1,
            Duration::from_millis(40),
            "held for one whole interval again, so the key is never nearer expiry than that"
        );
        assert!(
            watcher.noted.borrow().is_empty(),
            "a lease that stayed this host's is not reported to anyone: {:?}",
            watcher.noted.borrow()
        );
    }

    /// § 3's fallback: with no lease to take, a `fleet` entry is not armed and the boot says so.
    ///
    /// The refusal and not the omission is the point. A tree naming no shared store and a store
    /// that will not answer the boot both arrive here as the same absence — and firing a
    /// fleet-scoped interval on each host's own clock instead is the exact failure the key exists
    /// to prevent. A ticker that armed it anyway would look correct on one host.
    #[test]
    fn a_shared_store_with_no_compare_and_set_leaves_the_entry_unarmed_and_says_so() {
        let mut notes = Vec::new();
        let armed = arm(
            &[
                entry("nightly", "@daily", "host"),
                entry("invoices", "@daily", "fleet"),
            ],
            &instant("2026-01-01T00:00:00Z"),
            None,
            |note| notes.push(note.to_owned()),
        );

        assert_eq!(
            armed.iter().map(Armed::name).collect::<Vec<_>>(),
            ["nightly"],
            "the host-scoped entry is armed and the fleet-scoped one is not"
        );
        assert_eq!(notes.len(), 1, "and the one that is not is named");
        assert!(
            notes[0].contains("invoices")
                && notes[0].contains("fleet")
                && notes[0].contains("compare-and-set"),
            "the note names the entry and the operation the store is missing: {}",
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
            None,
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

    /// Drive the ticker over one `* * * * *` entry in `overlap`'s mode, through two consecutive
    /// minutes, stopping after `passes` of them.
    ///
    /// The clock is a parameter and stands still a tenth of a second before each minute, so the two
    /// waits the ticker computes are real and short; `holds` is what each fire spends on its own
    /// task, and setting it far longer than those waits is the only way to hold one run open across
    /// the next minute — which is the state every mode below is about. A read past the last instant
    /// answers the last instant, which is the tick's tail parking with nothing due.
    fn drive(
        overlap: &str,
        holds: Duration,
        passes: usize,
    ) -> (Rc<Watcher>, Rc<RefCell<Vec<Armed>>>) {
        let clocks = [
            instant("2026-01-01T00:00:59.9Z"),
            instant("2026-01-01T00:01:00Z"),
            instant("2026-01-01T00:01:59.9Z"),
            instant("2026-01-01T00:02:00Z"),
        ];
        let roster = Rc::new(RefCell::new(arm(
            &[Schedule {
                overlap: Some(overlap.to_owned()),
                ..entry("nightly", "* * * * *", "host")
            }],
            &clocks[0],
            None,
            |note| panic!("nothing to report at boot, and it said: {note}"),
        )));
        let watcher = Rc::new(Watcher {
            holds,
            ..Watcher::new()
        });
        let reads = Rc::new(Cell::new(0_usize));
        let seen = Rc::new(Cell::new(0_usize));
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let roster = Rc::clone(&roster);
            let watcher = Rc::clone(&watcher);
            move |_ctx| {
                tick_on_this_core(
                    &mut roster.borrow_mut(),
                    &watcher,
                    None,
                    move || {
                        let read = reads.get();
                        reads.set(read + 1);
                        clocks[read.min(clocks.len() - 1)].clone()
                    },
                    move || {
                        let pass = seen.get();
                        seen.set(pass + 1);
                        if pass + 1 < passes {
                            ControlFlow::Continue(())
                        } else {
                            ControlFlow::Break(())
                        }
                    },
                )
                .expect("the ticker ran as a task");
            }
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        (watcher, roster)
    }

    /// The instant an entry fires next, as the assertions below read it.
    fn next_fire(roster: &Rc<RefCell<Vec<Armed>>>) -> Option<String> {
        roster.borrow()[0]
            .next()
            .map(|next| next.timestamp().to_string())
    }

    /// § 6's `skip`, the default: an entry whose previous run is still going has this fire dropped,
    /// and is rearmed anyway.
    ///
    /// Both halves matter and only the second is cheap to get wrong. A ticker that dropped the fire
    /// but left the entry due would make the next wait zero and spin for as long as the run lasts,
    /// which is worse than the overlap it was avoiding; a ticker that started the run beside its
    /// predecessor is what § 6 says a job that is already behind must not do.
    ///
    /// Two passes: the first minute starts a run that holds its task for far longer than the second
    /// minute's wait, so the second pass is the overlap.
    #[test]
    fn an_overlapping_fire_is_dropped_rather_than_started_beside_its_previous_run() {
        let (watcher, roster) = drive("skip", Duration::from_millis(750), 2);

        assert_eq!(
            watcher.started.get(),
            1,
            "the second minute asked for no isolate at all: the fire was dropped before one \
             could be built, not started and abandoned"
        );
        assert_eq!(
            watcher.logged.borrow().as_slice(),
            ["nightly ok=true ran nightly"],
            "one run finished, and the ticker waited it out rather than taking it down"
        );
        let noted = watcher.noted.borrow();
        assert_eq!(noted.len(), 1, "the drop is named once: {noted:?}");
        assert!(
            noted[0].contains("nightly") && noted[0].contains("skip"),
            "the note names the entry and the mode that dropped it: {}",
            noted[0]
        );
        assert_eq!(
            next_fire(&roster),
            Some("2026-01-01T00:03:00Z".to_owned()),
            "a dropped fire rearms like any other: the entry is not left due, which would make \
             the next wait zero and spin for as long as the run lasts"
        );
    }

    /// § 6's `queue`: the overlapping fire is **held**, and it starts the moment the run before it
    /// ends rather than at the next minute the expression names.
    ///
    /// The third pass is the one this case exists for. Its wait is a whole minute of the schedule's
    /// own clock and it returns in three quarters of a second, because what a held fire waits on is
    /// a run *ending* — a ticker that slept out the interval instead would pass every assertion
    /// below except the one that says two runs finished, and would be `skip` with a delay.
    #[test]
    fn a_queued_fire_starts_when_the_run_before_it_ends_rather_than_at_the_next_minute() {
        let (watcher, roster) = drive("queue", Duration::from_millis(750), 3);

        assert_eq!(
            watcher.started.get(),
            2,
            "the overlapping fire was held rather than dropped, and it ran"
        );
        assert_eq!(
            watcher.logged.borrow().len(),
            2,
            "both runs finished, one after the other and never beside each other: {:?}",
            watcher.logged.borrow()
        );
        assert!(
            watcher.noted.borrow().is_empty(),
            "a held fire is not a dropped one, so there is nothing to report: {:?}",
            watcher.noted.borrow()
        );
        assert_eq!(
            next_fire(&roster),
            Some("2026-01-01T00:03:00Z".to_owned()),
            "holding a fire does not hold the schedule: the entry rearmed from the clock"
        );
    }

    /// § 6's `kill`: the running isolate is cancelled, its teardown is waited for, and only then
    /// does the new run start.
    ///
    /// The ordering is the claim. A ticker that cancelled and started in the same breath would show
    /// two started runs here as well, so what pins it is the run that did **not** log: the first
    /// fire is torn down where it parked and never reaches [`Fires::ran`], which is `rule:concurrency/cancellation-runs-no-user-code`'s
    /// rule that cancellation runs no user code — and the second run's own completion proves the
    /// teardown finished rather than being merely asked for.
    #[test]
    fn a_killed_run_is_torn_down_and_waited_for_before_the_new_one_starts() {
        let (watcher, roster) = drive("kill", Duration::from_millis(750), 2);

        assert_eq!(
            watcher.started.get(),
            2,
            "the second minute started its run"
        );
        assert_eq!(
            watcher.logged.borrow().as_slice(),
            ["nightly ok=true ran nightly"],
            "one completion, and it is the second run's: the first was cancelled at its safepoint \
             and ran nothing after it"
        );
        assert!(
            watcher.noted.borrow().is_empty(),
            "nothing was dropped, so nothing is reported: {:?}",
            watcher.noted.borrow()
        );
        assert_eq!(
            next_fire(&roster),
            Some("2026-01-01T00:03:00Z".to_owned()),
            "the entry rearmed from the clock, as it does under every mode"
        );
    }

    /// § 6's three modes are read at boot, and a word that is none of them is armed as `skip` and
    /// named.
    ///
    /// `nvs_config::schedule::validate` refuses that word before this can be reached, and the
    /// note is here for the reason the roster's other unreachable notes are: it makes a hole in
    /// that argument visible, where the silent alternative is an entry running a mode nobody chose.
    /// The three that are spelled correctly are asserted beside it, because a reader that took the
    /// fallback branch for all four would look identical from the roster alone.
    #[test]
    fn an_overlap_word_that_is_none_of_the_three_is_armed_as_skip_and_named() {
        let mut notes = Vec::new();
        let armed = arm(
            &[
                Schedule {
                    overlap: Some("replace".to_owned()),
                    ..entry("invoices", "@daily", "host")
                },
                Schedule {
                    overlap: Some("queue".to_owned()),
                    ..entry("nightly", "@daily", "host")
                },
                Schedule {
                    overlap: Some("kill".to_owned()),
                    ..entry("hourly", "@hourly", "host")
                },
                entry("weekly", "@weekly", "host"),
            ],
            &instant("2026-01-01T00:00:00Z"),
            None,
            |note| notes.push(note.to_owned()),
        );

        assert_eq!(
            armed.iter().map(|entry| entry.overlap).collect::<Vec<_>>(),
            [Overlap::Skip, Overlap::Queue, Overlap::Kill, Overlap::Skip],
            "each word is read as the mode it names, and the entry that wrote none gets the default"
        );
        assert_eq!(
            notes.len(),
            1,
            "and only the word § 6 does not name is reported"
        );
        assert!(
            notes[0].contains("invoices") && notes[0].contains("replace"),
            "the note names the entry and what it asked for: {}",
            notes[0]
        );
    }
}
