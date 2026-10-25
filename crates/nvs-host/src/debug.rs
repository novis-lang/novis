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
//! A breakpoint may carry a [`BreakpointTest`]: its condition, hit count and
//! log message (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). Each
//! time a unit arms the breakpoint, the test is bound to the slot table of
//! every function with a site on its line, and a test that does not bind
//! leaves the breakpoint unverified with its message and arms no site. At an
//! armed site the bound test runs over the innermost frame's slots, and the
//! task stops only where it says so. So a condition costs nothing at a site
//! no breakpoint armed.
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
//! A step is one task's ([`Debugger::step`]). The session names the stop to
//! step from, and the resume wakes every task. The task of that stop sets a
//! [`Step`] on its own context before it runs on, so only its own sites stop
//! it. At a site that no breakpoint armed, the handler stops the task only
//! where [`Step::stops_at`] says and the site's file is not one
//! [`Debugger::skip_files`] skips. A step over or out from a frame that
//! returns shows the value it returned in the next stop ([`Stop::returned`]).
//! An `await` needs nothing of its own: the task's stack stays as it was
//! while it waits, the step stays on its context, and its siblings have
//! contexts of their own.
//!
//! A thrown exception stops its task where it is thrown, while every frame is
//! still linked, when [`Debugger::exception_filters`] says so: every throw,
//! or only one that no frame of the task catches. Each unit knows which
//! classes the `try` blocks around each site catch, so the handler reads off
//! the frame chain which frame catches the exception, if any. A throw that a
//! frame in a [`Debugger::skip_files`] file catches never stops.
//!
//! The controller is `Send` and `Sync`, so a session drives it from its own
//! thread while the program runs on a core.
//!
//! **What it spends:** one [`SiteLine`] per statement and one
//! [`DebugFunction`] per function per unit, one list of caught classes per
//! site inside a `try`, held while the compiled unit or a
//! request on it lives, one weak unit per load, freed at the next load once
//! the unit has ended, one weak safepoint view per
//! attached request, freed at the next attach once the request has ended, and
//! one wake per stopped task, freed by the resume. Each loaded unit holds
//! one bound test per armed site whose breakpoint has one, freed when the
//! breakpoint is replaced or the unit ends. A step holds one entry
//! until its task runs on. A stop holds the tree of
//! every variable of every frame until the session drops it. A program that
//! is not debugged spends nothing.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak, mpsc};

use nvs_render::Node;
use nvs_runtime::{
    Ctx, Fault, Frame, SafepointFlags, SiteTable, Step, StepKind, StopCause, Tag, WeakSafepointView,
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
    /// Its representation, which a breakpoint's condition is checked against.
    pub ty: VariableType,
}

/// The representation of a [`DebugVariable`], as far as a breakpoint's
/// condition is checked against it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableType {
    /// `bool`.
    Bool,
    /// `int`.
    Int,
    /// `uint`.
    Uint,
    /// `float`.
    Float,
    /// `string`.
    Str,
    /// Any other representation: an object, an array, a union.
    Other,
}

/// One line of a [`Debugger::set_breakpoints`] call, and whether any stop site
/// is on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    /// The 1-based line asked for.
    pub line: u32,
    /// Whether a statement starts on the line and its test binds there, so
    /// the breakpoint can stop.
    pub verified: bool,
    /// Why the breakpoint is not verified, where its test did not bind.
    pub message: Option<String>,
}

/// One breakpoint a [`Debugger::set_breakpoints`] call sets: its line, and
/// what it tests before it stops.
#[derive(Clone)]
pub struct BreakpointSpec {
    /// The 1-based line.
    pub line: u32,
    /// Its condition, hit count and log message, or `None` for a breakpoint
    /// that always stops.
    pub test: Option<Arc<dyn BreakpointTest>>,
}

impl BreakpointSpec {
    /// One breakpoint with no test on each of `lines`.
    #[must_use]
    pub fn lines(lines: &[u32]) -> Vec<Self> {
        lines
            .iter()
            .map(|&line| Self { line, test: None })
            .collect()
    }
}

impl std::fmt::Debug for BreakpointSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BreakpointSpec")
            .field("line", &self.line)
            .field("test", &self.test.is_some())
            .finish()
    }
}

/// What a breakpoint tests at an armed site before the task stops: its
/// condition, its hit count and its log message. The session binds it to
/// the slot table of each function that has a site on its line, once per
/// loaded unit.
pub trait BreakpointTest: Send + Sync {
    /// The test for the sites of `function`, or the message that leaves the
    /// breakpoint unverified.
    ///
    /// # Errors
    ///
    /// The message, when the test cannot run against `function`'s slots.
    fn bind(&self, function: &DebugFunction) -> Result<Arc<dyn SiteTest>, String>;
}

/// A [`BreakpointTest`] bound to one function's slot table.
pub trait SiteTest: Send + Sync {
    /// Whether the task stops, given its innermost frame's slots. Runs on the
    /// task's own stack, where the slots are valid, and runs no program code.
    fn stops(&self, slots: &[nvs_runtime::Value]) -> bool;
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
    /// [`Debugger::stop_on_entry`] asked the program to stop before it runs,
    /// and this is the first stop site it reached.
    Entry,
    /// A [`Debugger::step`] of this task reached the site it stops at.
    Step,
    /// An exception was thrown, and [`Debugger::exception_filters`] says to
    /// stop for it. [`Stop::exception`] says which.
    Exception,
}

/// The exception a task stopped for, at [`StopReason::Exception`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoppedException {
    /// Its class's name.
    pub class: String,
    /// Its message.
    pub message: String,
    /// Whether no frame of the task catches it.
    pub uncaught: bool,
}

/// The value a function returned, shown at the stop after a step left it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoppedReturn {
    /// The function's name, or empty for an index the session was not told.
    pub function: String,
    /// The value as a record tree, or [`Node::Redacted`] where it may be
    /// `secret`.
    pub value: Node,
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
    /// This stop's number in the session, which [`Debugger::step`] names.
    pub id: u64,
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
    /// The value the frame a step started in returned, when the step left
    /// that frame.
    pub returned: Option<StoppedReturn>,
    /// The exception the task stopped for, at [`StopReason::Exception`].
    pub exception: Option<StoppedException>,
}

/// One compiled unit of a debugged program: its stop sites, where each one is,
/// and its functions. A recompile gives a new one, and a request runs on the
/// one it was attached with until it ends.
pub struct DebugUnit {
    sites: Arc<SiteTable>,
    lines: Vec<SiteLine>,
    /// The index of the function each site is in, indexed as `lines` is.
    owners: Vec<u32>,
    functions: Vec<DebugFunction>,
    /// The classes the `try` blocks around a site catch, for each site
    /// inside one, sorted by site.
    catches: Vec<(u32, Vec<String>)>,
    /// The bound test of each armed site whose breakpoint has one.
    tests: Mutex<FxHashMap<u32, Arc<dyn SiteTest>>>,
}

impl std::fmt::Debug for DebugUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugUnit")
            .field("sites", &self.sites)
            .field("lines", &self.lines)
            .field("functions", &self.functions)
            .finish_non_exhaustive()
    }
}

impl DebugUnit {
    /// The unit whose stop sites are `sites`. `lines` is where each site is,
    /// and `owners` the index of the function each site is in, both indexed
    /// the same way: the caller builds them from
    /// `nvs_ir::Program::stmt_lines` and each function's statement count.
    /// `functions` is the unit's functions by index, from
    /// `nvs_codegen::Unit::debug_function` and
    /// `nvs_codegen::Unit::debug_slots`. `catches` is
    /// `nvs_ir::Program::stmt_catches`: the classes the `try` blocks around
    /// each site catch, for each site inside one.
    #[must_use]
    pub fn new(
        sites: Arc<SiteTable>,
        lines: Vec<SiteLine>,
        owners: Vec<u32>,
        functions: Vec<DebugFunction>,
        mut catches: Vec<(u32, Vec<String>)>,
    ) -> Arc<Self> {
        debug_assert_eq!(sites.len(), lines.len(), "one line per stop site");
        debug_assert_eq!(owners.len(), lines.len(), "one owner per stop site");
        catches.sort_by_key(|(site, _)| *site);
        Arc::new(Self {
            sites,
            lines,
            owners,
            functions,
            catches,
            tests: Mutex::default(),
        })
    }

    /// The classes the `try` blocks around `site` catch.
    fn catches_at(&self, site: u32) -> &[String] {
        self.catches
            .binary_search_by_key(&site, |(at, _)| *at)
            .map_or(&[][..], |index| self.catches[index].1.as_slice())
    }

    /// The file `site` is in, or `None` for a site the session was not told.
    fn file_of(&self, site: u32) -> Option<&str> {
        let site = usize::try_from(site).ok()?;
        self.lines.get(site).map(|at| at.file.as_str())
    }

    /// Arms or disarms every site of `file` on each breakpoint's line.
    /// Answers one [`Breakpoint`] per breakpoint, in order. A breakpoint
    /// whose test does not bind to a function on its line arms no site.
    fn arm(&self, file: &str, breakpoints: &[BreakpointSpec], on: bool) -> Vec<Breakpoint> {
        let mut tests = self.tests.lock().unwrap_or_else(PoisonError::into_inner);
        breakpoints
            .iter()
            .map(|breakpoint| {
                let line = breakpoint.line;
                let sites: Vec<u32> = (0u32..)
                    .zip(&self.lines)
                    .filter(|(_, at)| at.file == file && at.line == line)
                    .map(|(site, _)| site)
                    .collect();
                if !on {
                    for &site in &sites {
                        self.sites.arm(site, false);
                        tests.remove(&site);
                    }
                    return Breakpoint {
                        line,
                        verified: false,
                        message: None,
                    };
                }
                let bound = match self.bind(&sites, breakpoint.test.as_deref()) {
                    Ok(bound) => bound,
                    Err(message) => {
                        return Breakpoint {
                            line,
                            verified: false,
                            message: Some(message),
                        };
                    }
                };
                for (&site, test) in sites.iter().zip(bound) {
                    if let Some(test) = test {
                        tests.insert(site, test);
                    }
                    self.sites.arm(site, true);
                }
                Breakpoint {
                    line,
                    verified: !sites.is_empty(),
                    message: None,
                }
            })
            .collect()
    }

    /// `test` bound to the function of each of `sites`, once per function.
    fn bind(
        &self,
        sites: &[u32],
        test: Option<&dyn BreakpointTest>,
    ) -> Result<Vec<Option<Arc<dyn SiteTest>>>, String> {
        let Some(test) = test else {
            return Ok(vec![None; sites.len()]);
        };
        let mut bound: FxHashMap<u32, Arc<dyn SiteTest>> = FxHashMap::default();
        sites
            .iter()
            .map(|&site| {
                let owner = usize::try_from(site)
                    .ok()
                    .and_then(|site| self.owners.get(site))
                    .copied()
                    .unwrap_or(u32::MAX);
                if let Some(test) = bound.get(&owner) {
                    return Ok(Some(Arc::clone(test)));
                }
                let function = usize::try_from(owner)
                    .ok()
                    .and_then(|owner| self.functions.get(owner))
                    .ok_or_else(|| {
                        "The breakpoint's function has no variables to test.".to_owned()
                    })?;
                let site_test = test.bind(function)?;
                bound.insert(owner, Arc::clone(&site_test));
                Ok(Some(site_test))
            })
            .collect()
    }

    /// Whether a task at the armed `site` stops: its breakpoint has no test,
    /// or the test says so over `slots`.
    fn passes(&self, site: u32, slots: &[nvs_runtime::Value]) -> bool {
        let test = self
            .tests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&site)
            .cloned();
        // Outside the lock: a test may write an `output` event.
        test.is_none_or(|test| test.stops(slots))
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
    /// Each file's breakpoints, as the session set them.
    breakpoints: FxHashMap<String, Vec<BreakpointSpec>>,
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
    /// Whether the program is still to stop at its first stop site. Every
    /// attach raises a pause while it is set, and the first stop clears it.
    entry: bool,
    /// Whether a [`Debugger::pause`] came while no request was attached. The
    /// next attach raises it, and the next stop clears it.
    pause_pending: bool,
    /// The wake of every task parked on a core.
    parked: Vec<RemoteWake>,
    /// How many stops have been sent: the next stop's [`Stop::id`].
    stops_sent: u64,
    /// The stop a [`Debugger::step`] steps from, and its kind, until that
    /// stop's task runs on.
    step: Option<(u64, StepKind)>,
    /// The files a step does not stop in.
    skip: Option<SkipFiles>,
    /// Whether every thrown exception stops a task.
    stop_on_thrown: bool,
    /// Whether an exception no frame of its task catches stops the task.
    stop_on_uncaught: bool,
    /// Told each file's breakpoints every time a unit loads.
    on_load: Option<OnLoad>,
}

/// What [`Debugger::on_load`] calls, after each [`Debugger::load`].
type LoadListener = dyn Fn(&[(String, Vec<Breakpoint>)]) + Send + Sync;

/// What [`Debugger::skip_files`] calls with a site's file.
type SkipTest = dyn Fn(&str) -> bool + Send + Sync;

/// The test of [`State::skip`], so `State` can still be printed.
#[derive(Clone)]
struct SkipFiles(Arc<SkipTest>);

impl std::fmt::Debug for SkipFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SkipFiles")
    }
}

/// The listener of [`State::on_load`], so `State` can still be printed.
#[derive(Clone)]
struct OnLoad(Arc<LoadListener>);

impl std::fmt::Debug for OnLoad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OnLoad")
    }
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
        let (resolved, listener) = {
            let mut state = self.shared.lock();
            state.units.retain(|unit| unit.strong_count() > 0);
            state.units.push(Arc::downgrade(unit));
            let mut resolved: Vec<_> = state
                .breakpoints
                .iter()
                .map(|(file, breakpoints)| (file.clone(), unit.arm(file, breakpoints, true)))
                .collect();
            resolved.sort_by(|(a, _), (b, _)| a.cmp(b));
            (resolved, state.on_load.clone())
        };
        // Outside the lock, so the listener may call back into the session.
        if let Some(OnLoad(listener)) = listener {
            listener(&resolved);
        }
        resolved
    }

    /// Calls `listener` with what [`Self::load`] returns, each time a unit
    /// loads. A client learns this way which breakpoints it set before the
    /// compile can stop. Replaces the listener set before.
    pub fn on_load(&self, listener: impl Fn(&[(String, Vec<Breakpoint>)]) + Send + Sync + 'static) {
        self.shared.lock().on_load = Some(OnLoad(Arc::new(listener)));
    }

    /// Sets the breakpoints of `file` to `breakpoints`, replacing the ones it
    /// had, and arms every stop site on each line in every loaded unit, its
    /// test bound to each function there. Returns one [`Breakpoint`] per
    /// breakpoint, in order, resolved in the newest unit.
    pub fn set_breakpoints(&self, file: &str, breakpoints: &[BreakpointSpec]) -> Vec<Breakpoint> {
        let mut state = self.shared.lock();
        let before = state.breakpoints.remove(file).unwrap_or_default();
        let units: Vec<Arc<DebugUnit>> = state.units.iter().filter_map(Weak::upgrade).collect();
        let mut answer = breakpoints
            .iter()
            .map(|breakpoint| Breakpoint {
                line: breakpoint.line,
                verified: false,
                message: None,
            })
            .collect();
        for unit in &units {
            unit.arm(file, &before, false);
            answer = unit.arm(file, breakpoints, true);
        }
        if !breakpoints.is_empty() {
            state
                .breakpoints
                .insert(file.to_owned(), breakpoints.to_vec());
        }
        answer
    }

    /// Makes the program stop at the first stop site it reaches, as
    /// [`StopReason::Entry`]. Every request attached from now until that stop
    /// starts with a pause raised, so the first of them to run a statement
    /// stops there.
    pub fn stop_on_entry(&self) {
        self.shared.lock().entry = true;
    }

    /// Asks every attached request to stop at its next poll. With none
    /// attached, the next request to attach stops at its first stop site, so a
    /// pause sent while the program compiles is not lost.
    pub fn pause(&self) {
        let mut state = self.shared.lock();
        state
            .requests
            .retain(|view| view.request(SafepointFlags::DEBUG_BREAK));
        if state.requests.is_empty() {
            state.pause_pending = true;
        }
    }

    /// Makes a step stop no task in a file `skip` returns `true` for. The
    /// test gets the file as a breakpoint names it. Replaces the test set
    /// before.
    pub fn skip_files(&self, skip: impl Fn(&str) -> bool + Send + Sync + 'static) {
        self.shared.lock().skip = Some(SkipFiles(Arc::new(skip)));
    }

    /// Sets the two exception filters. With `thrown`, every exception stops
    /// its task where it is thrown, except one a frame in a skipped file
    /// catches. With `uncaught`, an exception stops its task where it is
    /// thrown when no frame of that task catches it. Both start off.
    pub fn exception_filters(&self, thrown: bool, uncaught: bool) {
        let mut state = self.shared.lock();
        state.stop_on_thrown = thrown;
        state.stop_on_uncaught = uncaught;
    }

    /// Resumes every stopped task, and makes the task of the stop numbered
    /// `stop` step by `kind` from where it stopped. Every other task runs on
    /// as [`Self::resume`] makes it.
    pub fn step(&self, stop: u64, kind: StepKind) {
        self.shared.lock().step = Some((stop, kind));
        self.wake_all();
    }

    /// Resumes every stopped task: lowers the stop on every attached request,
    /// adds the time stopped to its wall deadline, and wakes each parked task.
    pub fn resume(&self) {
        self.shared.lock().step = None;
        self.wake_all();
    }

    fn wake_all(&self) {
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
            if state.entry || state.pause_pending {
                ctx.request_safepoint(SafepointFlags::DEBUG_BREAK);
            }
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

    /// The stop handler: stops the whole tree at a breakpoint or at the end
    /// of a step, reports the stop, then parks until a resume.
    fn stop(&self, ctx: &mut Ctx, cause: StopCause, unit: &DebugUnit) -> i32 {
        let site = ctx.frames().next().map(|frame| frame.site);
        let armed = cause == StopCause::Site
            && ctx.frames().next().is_some_and(|frame| {
                unit.sites.is_armed(frame.site) && unit.passes(frame.site, frame.slots)
            });
        if cause == StopCause::Site && !armed && !self.step_stops(ctx, site, unit) {
            return nvs_runtime::OK;
        }
        let exception = if cause == StopCause::Throw {
            match self.throw_stops(ctx, unit) {
                Some(exception) => Some(exception),
                None => return nvs_runtime::OK,
            }
        } else {
            None
        };
        let frames: Vec<StoppedFrame> = ctx.frames().map(|frame| read(unit, frame)).collect();
        let mut state = self.lock();
        let reason = match cause {
            StopCause::Site | StopCause::Throw => {
                // Raised under the lock, so the resume that ends this stop
                // lowers it after, and every other task of the tree stops at
                // its next poll until then.
                ctx.request_safepoint(SafepointFlags::DEBUG_BREAK);
                state.breakpoint_round = Some(state.resumes);
                if exception.is_some() {
                    StopReason::Exception
                } else if armed {
                    StopReason::Breakpoint
                } else {
                    StopReason::Step
                }
            }
            // A resume lowered the flag while this task waited for the lock.
            StopCause::Break if !ctx.safepoint_flags().contains(SafepointFlags::DEBUG_BREAK) => {
                return nvs_runtime::OK;
            }
            StopCause::Break if state.entry => {
                state.entry = false;
                StopReason::Entry
            }
            StopCause::Break if state.breakpoint_round == Some(state.resumes) => StopReason::Tree,
            StopCause::Break => StopReason::Pause,
        };
        state.pause_pending = false;
        // Any stop ends the task's step, and a later step starts from here.
        ctx.set_step(None);
        let returned = ctx.take_returned().map(|returned| StoppedReturn {
            function: usize::try_from(returned.function)
                .ok()
                .and_then(|index| unit.functions.get(index))
                .map(|function| function.name.clone())
                .unwrap_or_default(),
            value: returned.value,
        });
        ctx.hold_deadlines();
        let stopped_at = state.resumes;
        let id = state.stops_sent;
        state.stops_sent += 1;
        // Sent under the lock, so a resume made after the session reads it
        // moves the count this task waits on.
        let _ = self.stops.send(Stop {
            id,
            task: crate::scheduler::current_task(),
            reason,
            round: stopped_at,
            frames,
            returned,
            exception,
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
        if let Some((_, kind)) = state.step.take_if(|(stop, _)| *stop == id) {
            ctx.set_step(Some(Step {
                kind,
                depth: ctx.frame_depth(),
            }));
        }
        nvs_runtime::OK
    }

    /// The exception pending on `ctx`, when the filters say a task stops for
    /// it. The innermost frame whose statement is inside a `try` that catches
    /// its class catches it. One in a skipped file never stops the task, and
    /// with no such frame the exception is uncaught. Runs on the task's
    /// stack, where its frames are linked.
    fn throw_stops(&self, ctx: &Ctx, unit: &DebugUnit) -> Option<StoppedException> {
        let (thrown, uncaught, skip) = {
            let state = self.lock();
            (
                state.stop_on_thrown,
                state.stop_on_uncaught,
                state.skip.clone(),
            )
        };
        if !thrown && !uncaught {
            return None;
        }
        let catcher = ctx.frames().find(|frame| {
            unit.catches_at(frame.site)
                .iter()
                .any(|class| ctx.pending_conforms_to(class))
        });
        if let Some(frame) = catcher {
            let skipped = match (unit.file_of(frame.site), skip) {
                (Some(file), Some(SkipFiles(skip))) => skip(file),
                _ => false,
            };
            if !thrown || skipped {
                return None;
            }
        }
        Some(StoppedException {
            class: ctx.pending_class().unwrap_or_default(),
            message: ctx.pending().map(String::from).unwrap_or_default(),
            uncaught: catcher.is_none(),
        })
    }

    /// Whether the step of `ctx` stops at `site`, which no breakpoint armed:
    /// the step's kind allows the depth, and the site's file is not skipped.
    fn step_stops(&self, ctx: &Ctx, site: Option<u32>, unit: &DebugUnit) -> bool {
        let Some(step) = ctx.step() else {
            return false;
        };
        if !step.stops_at(ctx.frame_depth()) {
            return false;
        }
        let at = site
            .and_then(|site| usize::try_from(site).ok())
            .and_then(|site| unit.lines.get(site));
        let skip = self.lock().skip.clone();
        match (at, skip) {
            (Some(at), Some(SkipFiles(skip))) => !skip(&at.file),
            _ => true,
        }
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
            vec![0; lines.len()],
            Vec::new(),
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
    fn a_site_s_catches_are_found_by_site_whatever_order_they_arrive_in() {
        let classes = |names: &[&str]| names.iter().map(|&n| n.to_owned()).collect::<Vec<_>>();
        let unit = DebugUnit::new(
            Arc::new(SiteTable::new(4)),
            (1..=4)
                .map(|line| SiteLine {
                    file: "a.nvs".to_owned(),
                    line,
                })
                .collect(),
            vec![0; 4],
            Vec::new(),
            vec![
                (3, classes(&["LogicError"])),
                (1, classes(&["RuntimeError", "IOError"])),
            ],
        );
        assert_eq!(unit.catches_at(1), classes(&["RuntimeError", "IOError"]));
        assert_eq!(unit.catches_at(3), classes(&["LogicError"]));
        assert!(unit.catches_at(0).is_empty());
        assert!(unit.catches_at(2).is_empty());
        assert!(unit.catches_at(9).is_empty());
        assert_eq!(unit.file_of(3), Some("a.nvs"));
        assert_eq!(unit.file_of(9), None);
    }

    #[test]
    fn a_secret_variable_renders_redacted_and_an_unset_one_is_left_out() {
        let variable = |name: &str, secret| DebugVariable {
            name: name.to_owned(),
            secret,
            ty: VariableType::Int,
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
        let set = debugger.set_breakpoints("a.nvs", &BreakpointSpec::lines(&[2, 3]));
        assert_eq!(
            set,
            [
                Breakpoint {
                    line: 2,
                    verified: true,
                    message: None,
                },
                Breakpoint {
                    line: 3,
                    verified: false,
                    message: None,
                },
            ]
        );
        assert!(sites.is_armed(0) && sites.is_armed(1) && !sites.is_armed(2));

        let set = debugger.set_breakpoints("a.nvs", &BreakpointSpec::lines(&[4]));
        assert!(set[0].verified);
        assert!(!sites.is_armed(0) && !sites.is_armed(1) && sites.is_armed(2));
        assert!(!debugger.set_breakpoints("b.nvs", &BreakpointSpec::lines(&[4]))[0].verified);
        assert!(sites.is_armed(2), "another file's set leaves this one's");
    }

    #[test]
    fn a_new_unit_takes_every_breakpoint_by_line_and_the_old_unit_keeps_its_own() {
        let (debugger, _stops, old) = session();
        debugger.set_breakpoints("a.nvs", &BreakpointSpec::lines(&[2, 4]));

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
        assert!(debugger.set_breakpoints("a.nvs", &BreakpointSpec::lines(&[3]))[0].verified);
        assert!(new.sites.is_armed(0) && !new.sites.is_armed(1));
        assert!(!old.sites.is_armed(0) && !old.sites.is_armed(2));

        // A unit nothing holds any more is left out.
        drop(old);
        assert!(debugger.set_breakpoints("a.nvs", &BreakpointSpec::lines(&[4]))[0].verified);
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
