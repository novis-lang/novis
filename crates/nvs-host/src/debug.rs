//! The controller of one debug session: breakpoints, pause, resume, and the
//! stop each task reports
//! (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`).
//!
//! A program compiled under the debug switch has a stop site at every
//! statement, and its unit has one armed byte per site in a [`SiteTable`]. A
//! [`Debugger`] arms the sites of each breakpoint's line, and
//! [`Debugger::attach`] installs the stop handler a context runs at an armed
//! site and at a [`SafepointFlags::DEBUG_BREAK`] poll. The handler sends a
//! [`Stop`] and parks the task on its own stack until [`Debugger::resume`].
//!
//! A session outlives a recompile
//! (`rule:config/an-edit-reaches-the-next-request-without-a-restart`). Each
//! compiled unit is a [`DebugUnit`], and [`Debugger::load`] adds the new one
//! and resolves every breakpoint again by file and line against it. The
//! session keeps a breakpoint as its file and line, never as a site, because
//! an edit renumbers the sites. A request is attached with the unit it runs,
//! and finishes on that unit with its sites and its lines. A breakpoint set
//! later is armed in every unit still running.
//!
//! A stop is the request tree's (`rule:testing/a-stop-pauses-the-whole-task-tree`).
//! The handler is set on the tree, so every context of it runs the same one,
//! a `spawn worker` child on another core included. A task that stops at a
//! breakpoint raises [`SafepointFlags::DEBUG_BREAK`] on the tree's word, and
//! every other task of the tree stops at its next poll. The flag stays raised
//! until [`Debugger::resume`] lowers it. Each task sends its own [`Stop`], and
//! every [`Stop`] of one stop has the same [`Stop::round`], so a debugger
//! tells its client once per round, with each task as a thread.
//!
//! While the tree is stopped, its wall deadline is held
//! ([`Ctx::hold_deadlines`]): the watchdog and `nvs run`'s clock read no
//! deadline, and the resume adds the time stopped to it. No CPU time is
//! charged, because a stopped task does not run on its thread.
//!
//! A task on a core with a reactor gives the core back while it is stopped, so
//! the other requests on that core keep running. A context with no reactor
//! beneath it blocks its thread instead: `nvs run`'s one program, or a unit
//! test. A served core always has a reactor, so a stop never blocks a core
//! that serves other requests. A stopped task that is cancelled ends at once,
//! through [`Ctx::cancel`], and runs no more of its code.
//!
//! Each frame of a [`Stop`] has its file, its line and its variables. The
//! handler reads the variables on the task's own stack, before it parks,
//! because the slots are valid only there. Each value is the bounded tree a
//! dump shows ([`nvs_runtime::record::node`]), and a `secret` variable is
//! [`Node::Redacted`]
//! (`rule:security/redaction-reaches-the-tools-own-renderings`).
//!
//! The controller is `Send` and `Sync`, so a session drives it from its own
//! thread while the program runs on a core.
//!
//! **What it spends:** one [`SiteLine`] per statement and one
//! [`DebugFunction`] per function per unit, held while the compiled unit or a
//! request on it lives, one weak unit per load, freed at the next load once
//! the unit has ended, one weak safepoint view per
//! attached request, freed at the next attach once the request has ended, and
//! one wake per stopped task, freed by the resume. A stop holds the tree of
//! every variable of every frame until the session drops it. A program that
//! is not debugged spends nothing.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak, mpsc};

use nvs_render::Node;
use nvs_runtime::{
    Ctx, Fault, Frame, SafepointFlags, SiteTable, StopCause, Tag, WeakSafepointView,
};
use rustc_hash::FxHashMap;

use crate::reactor::RemoteWake;
use crate::scheduler::{TaskId, Waiting};

/// Where one stop site is in the source: the file a breakpoint names, and its
/// 1-based line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteLine {
    /// The file, spelled the way a breakpoint names it.
    pub file: String,
    /// The 1-based line the statement starts on.
    pub line: u32,
}

/// One function of the debugged unit, at its index in
/// `nvs_ir::Program::functions`: what a frame record's index names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugFunction {
    /// The function's name in the unit.
    pub name: String,
    /// One entry per slot of its slot table, in slot order.
    pub slots: Vec<DebugVariable>,
}

/// One slot of a [`DebugFunction`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugVariable {
    /// The variable's name, without its `$`.
    pub name: String,
    /// Whether its type is `secret`, so its value is never shown.
    pub secret: bool,
}

/// One line of a [`Debugger::set_breakpoints`] call, and whether any stop site
/// is on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    /// The 1-based line asked for.
    pub line: u32,
    /// Whether a statement starts on the line, so the breakpoint can stop.
    pub verified: bool,
}

/// Why a task stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// The task reached a site a breakpoint armed.
    Breakpoint,
    /// [`Debugger::pause`] asked the task to stop at its next poll.
    Pause,
    /// Another task of its request tree stopped at a breakpoint, and this one
    /// stopped at its next poll.
    Tree,
}

/// One call in flight of a stopped task, innermost first in [`Stop::frames`].
#[derive(Clone, Debug, PartialEq)]
pub struct StoppedFrame {
    /// The function's index in the unit.
    pub function: u32,
    /// The function's name, or empty for an index the session was not told.
    pub name: String,
    /// The program-wide number of the stop site the call last reached.
    pub site: u32,
    /// Where that site is, or `None` for a site the session was not told.
    pub at: Option<SiteLine>,
    /// Every variable with a value at that site, in slot order.
    pub variables: Vec<StoppedVariable>,
}

/// One variable of a [`StoppedFrame`].
#[derive(Clone, Debug, PartialEq)]
pub struct StoppedVariable {
    /// The variable's name, without its `$`.
    pub name: String,
    /// Its value as a record tree, or [`Node::Redacted`] for a `secret` one.
    pub value: Node,
}

/// What a task reports when it stops.
#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    /// The task that stopped, or `None` for a context with no scheduler
    /// beneath it.
    pub task: Option<TaskId>,
    /// Why it stopped.
    pub reason: StopReason,
    /// Which stop of the session this is. Every task that one breakpoint or
    /// one pause stops has the same round, and each resume starts the next.
    pub round: u64,
    /// The calls in flight compiled under the debug switch, innermost first.
    pub frames: Vec<StoppedFrame>,
}

/// One compiled unit of a debugged program: its stop sites, where each one is,
/// and its functions. A recompile gives a new one, and a request runs on the
/// one it was attached with until it ends.
#[derive(Debug)]
pub struct DebugUnit {
    sites: Arc<SiteTable>,
    lines: Vec<SiteLine>,
    functions: Vec<DebugFunction>,
}

impl DebugUnit {
    /// The unit whose stop sites are `sites`. `lines` is where each site is,
    /// indexed the same way: the caller builds it from
    /// `nvs_ir::Program::stmt_lines`. `functions` is the unit's functions by
    /// index, from `nvs_codegen::Unit::debug_function` and
    /// `nvs_codegen::Unit::debug_slots`.
    #[must_use]
    pub fn new(
        sites: Arc<SiteTable>,
        lines: Vec<SiteLine>,
        functions: Vec<DebugFunction>,
    ) -> Arc<Self> {
        debug_assert_eq!(sites.len(), lines.len(), "one line per stop site");
        Arc::new(Self {
            sites,
            lines,
            functions,
        })
    }

    /// Arms or disarms every site of `file` on each of `lines`. Answers one
    /// [`Breakpoint`] per line, in order.
    fn arm(&self, file: &str, lines: &[u32], on: bool) -> Vec<Breakpoint> {
        lines
            .iter()
            .map(|&line| {
                let mut verified = false;
                for (site, _) in (0u32..)
                    .zip(&self.lines)
                    .filter(|(_, at)| at.file == file && at.line == line)
                {
                    self.sites.arm(site, on);
                    verified = true;
                }
                Breakpoint { line, verified }
            })
            .collect()
    }
}

/// The controller of one debug session. Cloning it gives a second handle to
/// the same session.
#[derive(Clone, Debug)]
pub struct Debugger {
    shared: Arc<Shared>,
}

/// What every handle and every attached context share.
#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    /// Signalled by every resume, for a stopped context with no reactor.
    resumed: Condvar,
    stops: mpsc::Sender<Stop>,
}

#[derive(Debug, Default)]
struct State {
    /// The lines of each file's breakpoints, as the session set them.
    breakpoints: FxHashMap<String, Vec<u32>>,
    /// Every unit loaded into the session, the newest last. Weak, because a
    /// unit lives as long as its compiled code and the requests running it.
    units: Vec<Weak<DebugUnit>>,
    /// The safepoint word of every attached request, for a pause and a
    /// resume. Weak, because each tree holds this session's stop handler.
    requests: Vec<WeakSafepointView>,
    /// How many resumes there have been. A stopped task runs on once this
    /// moves past the count it stopped at.
    resumes: u64,
    /// The round a task last stopped at a breakpoint in.
    breakpoint_round: Option<u64>,
    /// The wake of every task parked on a core.
    parked: Vec<RemoteWake>,
}

impl Debugger {
    /// A session with no unit and no breakpoint yet. The receiver gets every
    /// [`Stop`].
    #[must_use]
    pub fn new() -> (Self, mpsc::Receiver<Stop>) {
        let (stops, receiver) = mpsc::channel();
        let shared = Shared {
            state: Mutex::default(),
            resumed: Condvar::new(),
            stops,
        };
        (
            Self {
                shared: Arc::new(shared),
            },
            receiver,
        )
    }

    /// Adds `unit` to the session as its newest unit, and arms every
    /// breakpoint again against it. A recompile calls this with the new unit.
    /// The units loaded before keep their sites armed for the requests still
    /// running on them.
    ///
    /// Returns each file's breakpoints resolved in `unit`, sorted by file. A
    /// breakpoint on a line that no longer has a statement is not
    /// [`Breakpoint::verified`].
    pub fn load(&self, unit: &Arc<DebugUnit>) -> Vec<(String, Vec<Breakpoint>)> {
        let mut state = self.shared.lock();
        state.units.retain(|unit| unit.strong_count() > 0);
        state.units.push(Arc::downgrade(unit));
        let mut resolved: Vec<_> = state
            .breakpoints
            .iter()
            .map(|(file, lines)| (file.clone(), unit.arm(file, lines, true)))
            .collect();
        resolved.sort_by(|(a, _), (b, _)| a.cmp(b));
        resolved
    }

    /// Sets the breakpoints of `file` to `lines`, replacing the ones it had,
    /// and arms every stop site on each line in every loaded unit. Returns
    /// one [`Breakpoint`] per line, in order, resolved in the newest unit.
    pub fn set_breakpoints(&self, file: &str, lines: &[u32]) -> Vec<Breakpoint> {
        let mut state = self.shared.lock();
        let before = state.breakpoints.remove(file).unwrap_or_default();
        let units: Vec<Arc<DebugUnit>> = state.units.iter().filter_map(Weak::upgrade).collect();
        let mut answer = lines
            .iter()
            .map(|&line| Breakpoint {
                line,
                verified: false,
            })
            .collect();
        for unit in &units {
            unit.arm(file, &before, false);
            answer = unit.arm(file, lines, true);
        }
        if !lines.is_empty() {
            state.breakpoints.insert(file.to_owned(), lines.to_vec());
        }
        answer
    }

    /// Asks every attached request to stop at its next poll.
    pub fn pause(&self) {
        self.shared
            .lock()
            .requests
            .retain(|view| view.request(SafepointFlags::DEBUG_BREAK));
    }

    /// Resumes every stopped task: lowers the stop on every attached request,
    /// adds the time stopped to its wall deadline, and wakes each parked task.
    pub fn resume(&self) {
        let parked = {
            let mut state = self.shared.lock();
            state.resumes += 1;
            // Under the lock, so a task that saw the flag and waits for the
            // lock reads it lowered and runs on.
            state.requests.retain(WeakSafepointView::release_debug_stop);
            std::mem::take(&mut state.parked)
        };
        self.shared.resumed.notify_all();
        for wake in parked {
            // A failed poke loses promptness, not the wake.
            let _ = wake.wake();
        }
    }

    /// Makes the request tree of `ctx`, which runs `unit`, stop under this
    /// session: at an armed site, and at the poll after a [`Self::pause`].
    /// Every context of the tree stops, a child spawned later included. Its
    /// frames are read against `unit` until the tree ends, whatever units the
    /// session loads after. The request stays attached until its tree ends or
    /// its stop handler is replaced.
    pub fn attach(&self, ctx: &mut Ctx, unit: &Arc<DebugUnit>) {
        {
            let mut state = self.shared.lock();
            state.requests.retain(|view| view.flags().is_some());
            state.requests.push(ctx.safepoint_view().downgrade());
        }
        let shared = Arc::clone(&self.shared);
        let unit = Arc::clone(unit);
        ctx.set_debug_stop(Some(Arc::new(move |ctx: &mut Ctx, cause| {
            shared.stop(ctx, cause, &unit)
        })));
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The stop handler: stops the whole tree at a breakpoint, reports the
    /// stop, then parks until a resume.
    fn stop(&self, ctx: &mut Ctx, cause: StopCause, unit: &DebugUnit) -> i32 {
        let frames: Vec<StoppedFrame> = ctx.frames().map(|frame| read(unit, frame)).collect();
        let mut state = self.lock();
        let reason = match cause {
            StopCause::Site => {
                // Raised under the lock, so the resume that ends this stop
                // lowers it after, and every other task of the tree stops at
                // its next poll until then.
                ctx.request_safepoint(SafepointFlags::DEBUG_BREAK);
                state.breakpoint_round = Some(state.resumes);
                StopReason::Breakpoint
            }
            // A resume lowered the flag while this task waited for the lock.
            StopCause::Break if !ctx.safepoint_flags().contains(SafepointFlags::DEBUG_BREAK) => {
                return nvs_runtime::OK;
            }
            StopCause::Break if state.breakpoint_round == Some(state.resumes) => StopReason::Tree,
            StopCause::Break => StopReason::Pause,
        };
        ctx.hold_deadlines();
        let stopped_at = state.resumes;
        // Sent under the lock, so a resume made after the session reads it
        // moves the count this task waits on.
        let _ = self.stops.send(Stop {
            task: crate::scheduler::current_task(),
            reason,
            round: stopped_at,
            frames,
        });
        while state.resumes == stopped_at {
            if let Some(wake) = crate::reactor::wake_this_task() {
                state.parked.push(wake);
                drop(state);
                if crate::scheduler::suspend_current(Waiting::Parked).cancelled() {
                    return match ctx.cancel() {
                        Fault::Pending(status) => status,
                        _ => nvs_runtime::FATAL,
                    };
                }
                state = self.lock();
            } else {
                state = self
                    .resumed
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        }
        nvs_runtime::OK
    }
}

/// One frame of a stopped task running `unit`, its variables rendered. Runs
/// on the task's stack, where `frame`'s slots are valid.
fn read(unit: &DebugUnit, frame: Frame<'_>) -> StoppedFrame {
    let function = usize::try_from(frame.function)
        .ok()
        .and_then(|index| unit.functions.get(index));
    let at = usize::try_from(frame.site)
        .ok()
        .and_then(|site| unit.lines.get(site))
        .cloned();
    StoppedFrame {
        function: frame.function,
        name: function.map(|f| f.name.clone()).unwrap_or_default(),
        site: frame.site,
        at,
        variables: function.map_or_else(Vec::new, |f| variables(f, frame.slots)),
    }
}

/// The variables of `function` that have a value in `slots`, rendered.
fn variables(function: &DebugFunction, slots: &[nvs_runtime::Value]) -> Vec<StoppedVariable> {
    function
        .slots
        .iter()
        .zip(slots)
        .filter(|(_, value)| value.tag() != Some(Tag::Unset))
        .map(|(variable, &value)| StoppedVariable {
            name: variable.name.clone(),
            value: if variable.secret {
                Node::Redacted
            } else {
                nvs_runtime::record::node(value)
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use nvs_runtime::TaskRoot;

    use super::*;
    use crate::reactor::{Reactor, install, run_until_idle};
    use crate::scheduler::Scheduler;

    /// A unit over `a.nvs` with a site on each of `lines`.
    fn unit(lines: &[u32]) -> Arc<DebugUnit> {
        let at = |&line| SiteLine {
            file: "a.nvs".to_owned(),
            line,
        };
        DebugUnit::new(
            Arc::new(SiteTable::new(lines.len())),
            lines.iter().map(at).collect(),
            Vec::new(),
        )
    }

    /// A session over three sites: two on line 2 of `a.nvs`, one on line 4.
    fn session() -> (Debugger, mpsc::Receiver<Stop>, Arc<DebugUnit>) {
        let unit = unit(&[2, 2, 4]);
        let (debugger, stops) = Debugger::new();
        debugger.load(&unit);
        (debugger, stops, unit)
    }

    #[test]
    fn a_secret_variable_renders_redacted_and_an_unset_one_is_left_out() {
        let variable = |name: &str, secret| DebugVariable {
            name: name.to_owned(),
            secret,
        };
        let function = DebugFunction {
            name: "main".to_owned(),
            slots: vec![
                variable("count", false),
                variable("later", false),
                variable("token", true),
            ],
        };
        let slots = [
            nvs_runtime::Value::int(3),
            nvs_runtime::Value::unset(),
            nvs_runtime::Value::int(42),
        ];
        let shown = variables(&function, &slots);
        assert_eq!(
            shown
                .iter()
                .map(|v| (v.name.as_str(), &v.value))
                .collect::<Vec<_>>(),
            [
                ("count", &nvs_runtime::record::node(slots[0])),
                ("token", &Node::Redacted),
            ]
        );
    }

    #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
    fn site(ctx: &mut Ctx) -> i32 {
        unsafe { nvs_runtime::nvs_debug_site(&raw mut *ctx) }
    }

    #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
    fn poll(ctx: &mut Ctx) -> i32 {
        unsafe { nvs_runtime::nvs_safepoint(&raw mut *ctx) }
    }

    #[test]
    fn a_breakpoint_arms_every_site_on_its_line_and_a_new_set_replaces_it() {
        let (debugger, _stops, unit) = session();
        let sites = &unit.sites;
        let set = debugger.set_breakpoints("a.nvs", &[2, 3]);
        assert_eq!(
            set,
            [
                Breakpoint {
                    line: 2,
                    verified: true
                },
                Breakpoint {
                    line: 3,
                    verified: false
                },
            ]
        );
        assert!(sites.is_armed(0) && sites.is_armed(1) && !sites.is_armed(2));

        let set = debugger.set_breakpoints("a.nvs", &[4]);
        assert!(set[0].verified);
        assert!(!sites.is_armed(0) && !sites.is_armed(1) && sites.is_armed(2));
        assert!(!debugger.set_breakpoints("b.nvs", &[4])[0].verified);
        assert!(sites.is_armed(2), "another file's set leaves this one's");
    }

    #[test]
    fn a_new_unit_takes_every_breakpoint_by_line_and_the_old_unit_keeps_its_own() {
        let (debugger, _stops, old) = session();
        debugger.set_breakpoints("a.nvs", &[2, 4]);

        // The edit moved line 2's statements to line 3, and kept line 4.
        let new = unit(&[3, 4]);
        let resolved = debugger.load(&new);
        let [(file, lines)] = resolved.as_slice() else {
            panic!("one file's breakpoints, got {resolved:?}");
        };
        assert_eq!(file, "a.nvs");
        assert_eq!(
            lines
                .iter()
                .map(|b| (b.line, b.verified))
                .collect::<Vec<_>>(),
            [(2, false), (4, true)]
        );
        assert!(!new.sites.is_armed(0) && new.sites.is_armed(1));
        assert!(
            old.sites.is_armed(0) && old.sites.is_armed(2),
            "the old unit keeps its sites"
        );

        // A new set reaches both units, and is resolved in the newest.
        assert!(debugger.set_breakpoints("a.nvs", &[3])[0].verified);
        assert!(new.sites.is_armed(0) && !new.sites.is_armed(1));
        assert!(!old.sites.is_armed(0) && !old.sites.is_armed(2));

        // A unit nothing holds any more is left out.
        drop(old);
        assert!(debugger.set_breakpoints("a.nvs", &[4])[0].verified);
        assert!(new.sites.is_armed(1));
    }

    #[test]
    fn a_stopped_task_gives_its_core_to_the_others_and_a_resume_continues_it() {
        let (debugger, stops, unit) = session();
        let (ran, after) = mpsc::channel();
        let controller = debugger.clone();
        let core = std::thread::spawn(move || {
            let mut sched = Scheduler::new();
            let _installed = install(Reactor::new().expect("the OS refused a poll"));
            let status = Rc::new(Cell::new(None));
            let seen = Rc::clone(&status);
            let mut ctx = Ctx::buffered();
            controller.attach(&mut ctx, &unit);
            sched.spawn(ctx, TaskRoot::Request, move |ctx| seen.set(Some(site(ctx))));
            sched.spawn(Ctx::buffered(), TaskRoot::Request, move |_| {
                ran.send("the other task ran").expect("the test is waiting");
            });
            let report = run_until_idle(&mut sched).expect("the loop failed");
            (status.get(), report.parked)
        });

        let stop = stops.recv().expect("the task stopped");
        assert_eq!(stop.reason, StopReason::Breakpoint);
        assert!(stop.task.is_some());
        assert_eq!(after.recv(), Ok("the other task ran"));
        debugger.resume();
        let (status, parked) = core.join().expect("the core ended");
        assert_eq!((status, parked), (Some(nvs_runtime::OK), 0));
    }

    #[test]
    fn a_pause_stops_an_attached_request_at_its_next_poll() {
        let (debugger, stops, unit) = session();
        let controller = debugger.clone();
        let (attached, ready) = mpsc::channel();
        let (paused, go) = mpsc::channel();
        let program = std::thread::spawn(move || {
            let mut ctx = Ctx::buffered();
            controller.attach(&mut ctx, &unit);
            attached.send(()).expect("the test is waiting");
            go.recv().expect("the test paused");
            assert!(ctx.safepoint_flags().contains(SafepointFlags::DEBUG_BREAK));
            let status = poll(&mut ctx);
            drop(ctx);
            status
        });

        ready.recv().expect("the request attached");
        debugger.pause();
        paused.send(()).expect("the program is waiting");
        let stop = stops.recv().expect("the request stopped");
        assert_eq!((stop.task, stop.reason), (None, StopReason::Pause));
        debugger.resume();
        assert_eq!(program.join().expect("the program ended"), nvs_runtime::OK);
        debugger.pause();
        assert!(
            debugger.shared.lock().requests.is_empty(),
            "a pause forgets a request that ended"
        );
    }

    /// A request with this `[limits]` block, read as a served request reads
    /// its snapshot.
    fn configured(written: &str) -> Ctx {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        let mut ctx = Ctx::buffered();
        ctx.set_config(Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    /// `rule:testing/a-stop-pauses-the-whole-task-tree`'s deadline half: a
    /// request stopped past its `wall_time` has no deadline to reach, and the
    /// resume moves the deadline by the time stopped. Real time passes here,
    /// because the time stopped is what is measured.
    #[test]
    fn a_stop_past_the_wall_time_does_not_end_the_request_and_moves_its_deadline() {
        let (debugger, stops, unit) = session();
        let controller = debugger.clone();
        let (attached, views) = mpsc::channel();
        let (start, go) = mpsc::channel();
        let program = std::thread::spawn(move || {
            let mut ctx = configured("[limits]\nwall_time = \"50ms\"\n");
            controller.attach(&mut ctx, &unit);
            attached
                .send(ctx.safepoint_view())
                .expect("the test is waiting");
            go.recv().expect("the test watches the deadline");
            let stopped = site(&mut ctx);
            (stopped, poll(&mut ctx), ctx.safepoint_flags())
        });

        let view = views.recv().expect("the request attached");
        let before = view
            .wall_deadline()
            .expect("a request with a `wall_time` has a deadline");
        // The clock that times a request waits on this wake, and reads the
        // deadline again each time it fires.
        let (moved, deadline_moved) = mpsc::channel();
        let wake = moved.clone();
        view.watch_wall_time(Box::new(move || {
            let _ = wake.send("held");
        }));
        start.send(()).expect("the program is waiting");

        let stop = stops.recv().expect("the request stopped");
        let stopped_at = Instant::now();
        assert_eq!(stop.reason, StopReason::Breakpoint);
        assert_eq!(deadline_moved.recv(), Ok("held"), "the stop woke the clock");
        assert_eq!(
            view.wall_deadline(),
            None,
            "a stopped request has no deadline"
        );
        std::thread::sleep(
            before.saturating_duration_since(Instant::now()) + Duration::from_millis(10),
        );
        assert!(
            Instant::now() > before,
            "the stop outlasted the original deadline"
        );
        assert_eq!(view.wall_deadline(), None);
        assert!(!view.flags().contains(SafepointFlags::WALL_TIME));

        view.watch_wall_time(Box::new(move || {
            let _ = moved.send("released");
        }));
        let resumed_at = Instant::now();
        debugger.resume();
        assert_eq!(
            deadline_moved.recv(),
            Ok("released"),
            "the resume woke the clock"
        );
        let after = view
            .wall_deadline()
            .expect("a resumed request has its deadline again");
        assert!(
            after >= before + (resumed_at - stopped_at),
            "the deadline moved by at least the time stopped"
        );
        let (stopped, next, flags) = program.join().expect("the program ended");
        assert_eq!((stopped, next), (nvs_runtime::OK, nvs_runtime::OK));
        assert!(
            !flags.intersects(SafepointFlags::WALL_TIME | SafepointFlags::DEBUG_BREAK),
            "a resumed request runs on with a clean word"
        );
    }

    /// A task's status and the output it wrote, once it ended.
    type Ended = Option<(i32, Vec<u8>)>;

    /// Spawns a task of the request tree on `sched` that writes `name` and
    /// `before`, stops at `stop`, writes `after`, and records its status and
    /// output in `seen`.
    fn spawn_writer(
        sched: &mut Scheduler,
        ctx: Ctx,
        stop: fn(&mut Ctx) -> i32,
        seen: &Rc<Cell<Ended>>,
        name: &'static str,
    ) {
        let seen = Rc::clone(seen);
        sched.spawn(ctx, TaskRoot::Request, move |ctx| {
            ctx.write_output(format!("{name} before\n").as_bytes())
                .expect("a buffer takes every write");
            let status = stop(ctx);
            ctx.write_output(format!("{name} after\n").as_bytes())
                .expect("a buffer takes every write");
            let output = ctx.take_buffered_output().expect("a buffered context");
            seen.set(Some((status, output)));
        });
    }

    /// Runs a request tree of two tasks on one core: the first stops at a
    /// site, the second polls after it. Beside it, an unattached request
    /// sends on `ran`. Gives each task's status and output, and how many
    /// tasks were left parked.
    fn run_a_tree_of_two(
        controller: Debugger,
        unit: Arc<DebugUnit>,
        ran: mpsc::Sender<&'static str>,
    ) -> std::thread::JoinHandle<(Ended, Ended, usize)> {
        std::thread::spawn(move || {
            let mut sched = Scheduler::new();
            let _installed = install(Reactor::new().expect("the OS refused a poll"));
            let mut first = Ctx::buffered();
            controller.attach(&mut first, &unit);
            let second = first.isolate(nvs_runtime::OutputSink::Buffer(Vec::new()));
            let first_seen = Rc::new(Cell::new(None));
            let second_seen = Rc::new(Cell::new(None));
            spawn_writer(&mut sched, first, site, &first_seen, "first");
            spawn_writer(&mut sched, second, poll, &second_seen, "second");
            sched.spawn(Ctx::buffered(), TaskRoot::Request, move |_| {
                ran.send("the other request ran")
                    .expect("the test is waiting");
            });
            let report = run_until_idle(&mut sched).expect("the loop failed");
            (first_seen.take(), second_seen.take(), report.parked)
        })
    }

    #[test]
    fn a_breakpoint_stops_every_task_of_its_tree_while_another_request_on_the_core_is_answered() {
        let (debugger, stops, unit) = session();
        let (ran, other) = mpsc::channel();
        let core = run_a_tree_of_two(debugger.clone(), unit, ran);

        let first = stops.recv().expect("the first task stopped");
        assert_eq!(first.reason, StopReason::Breakpoint);
        let second = stops.recv().expect("the second task stopped at its poll");
        assert_eq!(second.reason, StopReason::Tree);
        assert_eq!(second.round, first.round, "one stop, one round");
        assert!(first.task.is_some() && second.task.is_some());
        assert_ne!(first.task, second.task);
        assert_eq!(
            other.recv(),
            Ok("the other request ran"),
            "a request outside the tree is answered while it is stopped"
        );
        debugger.resume();

        let (first, second, parked) = core.join().expect("the core ended");
        assert_eq!(first.map(|(status, _)| status), Some(nvs_runtime::OK));
        assert_eq!(second.map(|(status, _)| status), Some(nvs_runtime::OK));
        assert_eq!(parked, 0);
    }

    #[test]
    fn a_resumed_tree_finishes_with_all_of_its_output_in_order() {
        let (debugger, stops, unit) = session();
        let (ran, other) = mpsc::channel();
        let core = run_a_tree_of_two(debugger.clone(), unit, ran);

        for _ in 0..2 {
            stops.recv().expect("each task of the tree stopped");
        }
        other.recv().expect("the other request ran");
        debugger.resume();

        let (first, second, _) = core.join().expect("the core ended");
        assert_eq!(
            first,
            Some((nvs_runtime::OK, b"first before\nfirst after\n".to_vec()))
        );
        assert_eq!(
            second,
            Some((nvs_runtime::OK, b"second before\nsecond after\n".to_vec()))
        );
    }

    /// A `spawn worker` child runs on another core, in its parent's tree.
    #[test]
    fn a_context_of_the_tree_on_another_thread_stops_at_its_next_poll_and_resumes_with_it() {
        let (debugger, stops, unit) = session();
        let controller = debugger.clone();
        let (handle, trees) = mpsc::channel();
        let program = std::thread::spawn(move || {
            let mut ctx = Ctx::buffered();
            controller.attach(&mut ctx, &unit);
            handle.send(ctx.tree_handle()).expect("the test is waiting");
            site(&mut ctx)
        });
        let tree = trees.recv().expect("the request attached");
        let first = stops.recv().expect("the request stopped");
        assert_eq!(first.reason, StopReason::Breakpoint);

        let worker = std::thread::spawn(move || {
            let mut ctx = Ctx::buffered();
            ctx.join_tree(tree);
            let stopped = poll(&mut ctx);
            (stopped, poll(&mut ctx))
        });
        let second = stops.recv().expect("the worker stopped at its poll");
        assert_eq!(
            (second.task, second.reason, second.round),
            (None, StopReason::Tree, first.round)
        );
        debugger.resume();

        assert_eq!(program.join().expect("the program ended"), nvs_runtime::OK);
        assert_eq!(
            worker.join().expect("the worker ended"),
            (nvs_runtime::OK, nvs_runtime::OK),
            "the worker ran on, and its next poll did not stop again"
        );
    }
}
