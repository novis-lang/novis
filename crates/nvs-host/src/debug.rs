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
//! A task on a core with a reactor gives the core back while it is stopped, so
//! the other requests on that core keep running. A context with no reactor
//! beneath it blocks its thread instead: `nvs run`'s one program, or a unit
//! test. A stopped task that is cancelled ends at once, through
//! [`Ctx::cancel`], and runs no more of its code.
//!
//! The controller is `Send` and `Sync`, so a session drives it from its own
//! thread while the program runs on a core.
//!
//! **What it spends:** one [`SiteLine`] per statement per session, one
//! safepoint view per attached request, and one wake per stopped task, freed by
//! the resume. A program that is not debugged spends nothing.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc};

use nvs_runtime::{Ctx, Fault, SafepointFlags, SafepointView, SiteTable};
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
}

/// One call in flight of a stopped task, innermost first in [`Stop::frames`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoppedFrame {
    /// The function's index in the unit, which `nvs_codegen::Unit::debug_function`
    /// names.
    pub function: u32,
    /// The program-wide number of the stop site the call last reached.
    pub site: u32,
}

/// What a task reports when it stops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stop {
    /// The task that stopped, or `None` for a context with no scheduler
    /// beneath it.
    pub task: Option<TaskId>,
    /// Why it stopped.
    pub reason: StopReason,
    /// The calls in flight compiled under the debug switch, innermost first.
    pub frames: Vec<StoppedFrame>,
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
    sites: Arc<SiteTable>,
    lines: Vec<SiteLine>,
    state: Mutex<State>,
    /// Signalled by every resume, for a stopped context with no reactor.
    resumed: Condvar,
    stops: mpsc::Sender<Stop>,
}

#[derive(Debug, Default)]
struct State {
    /// The sites each file's breakpoints armed.
    breakpoints: FxHashMap<String, Vec<u32>>,
    /// The safepoint word of every attached request, for a pause.
    requests: FxHashMap<u64, SafepointView>,
    next_request: u64,
    /// How many resumes there have been. A stopped task runs on once this
    /// moves past the count it stopped at.
    resumes: u64,
    /// The wake of every task parked on a core.
    parked: Vec<RemoteWake>,
}

impl Debugger {
    /// A session over the unit whose stop sites are `sites`. `lines` is where
    /// each site is, indexed the same way: the caller builds it from
    /// `nvs_ir::Program::stmt_lines`. The receiver gets every [`Stop`].
    #[must_use]
    pub fn new(sites: Arc<SiteTable>, lines: Vec<SiteLine>) -> (Self, mpsc::Receiver<Stop>) {
        debug_assert_eq!(sites.len(), lines.len(), "one line per stop site");
        let (stops, receiver) = mpsc::channel();
        let shared = Shared {
            sites,
            lines,
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

    /// Sets the breakpoints of `file` to `lines`, replacing the ones it had,
    /// and arms every stop site on each line. Returns one [`Breakpoint`] per
    /// line, in order.
    pub fn set_breakpoints(&self, file: &str, lines: &[u32]) -> Vec<Breakpoint> {
        let shared = &self.shared;
        let mut state = shared.lock();
        for site in state.breakpoints.remove(file).unwrap_or_default() {
            shared.sites.arm(site, false);
        }
        let mut armed = Vec::new();
        let answer = lines
            .iter()
            .map(|&line| {
                let before = armed.len();
                armed.extend(
                    (0u32..)
                        .zip(&shared.lines)
                        .filter(|(_, at)| at.file == file && at.line == line)
                        .map(|(site, _)| site),
                );
                Breakpoint {
                    line,
                    verified: armed.len() > before,
                }
            })
            .collect();
        for &site in &armed {
            shared.sites.arm(site, true);
        }
        if !armed.is_empty() {
            state.breakpoints.insert(file.to_owned(), armed);
        }
        answer
    }

    /// Asks every attached request to stop at its next poll.
    pub fn pause(&self) {
        for view in self.shared.lock().requests.values() {
            view.request(SafepointFlags::DEBUG_BREAK);
        }
    }

    /// Resumes every stopped task.
    pub fn resume(&self) {
        let parked = {
            let mut state = self.shared.lock();
            state.resumes += 1;
            std::mem::take(&mut state.parked)
        };
        self.shared.resumed.notify_all();
        for wake in parked {
            // A failed poke loses promptness, not the wake.
            let _ = wake.wake();
        }
    }

    /// Makes `ctx` stop under this session: at an armed site, and at the poll
    /// after a [`Self::pause`]. The request stays attached until `ctx` is
    /// dropped or its stop handler is replaced.
    pub fn attach(&self, ctx: &mut Ctx) {
        let id = {
            let mut state = self.shared.lock();
            let id = state.next_request;
            state.next_request += 1;
            state.requests.insert(id, ctx.safepoint_view());
            id
        };
        let attached = Attached {
            shared: Arc::clone(&self.shared),
            id,
        };
        ctx.set_debug_stop(Some(Box::new(move |ctx: &mut Ctx| {
            attached.shared.stop(ctx)
        })));
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The stop handler: reports the stop, then parks until a resume.
    fn stop(&self, ctx: &mut Ctx) -> i32 {
        let frames: Vec<StoppedFrame> = ctx
            .frames()
            .map(|frame| StoppedFrame {
                function: frame.function,
                site: frame.site,
            })
            .collect();
        let reason = match frames.first() {
            Some(frame) if self.sites.is_armed(frame.site) => StopReason::Breakpoint,
            _ => StopReason::Pause,
        };
        let mut state = self.lock();
        let stopped_at = state.resumes;
        // Sent under the lock, so a resume made after the session reads it
        // moves the count this task waits on.
        let _ = self.stops.send(Stop {
            task: crate::scheduler::current_task(),
            reason,
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

/// The stop handler's hold on its session, which detaches the request when
/// the handler is dropped.
struct Attached {
    shared: Arc<Shared>,
    id: u64,
}

impl Drop for Attached {
    fn drop(&mut self) {
        self.shared.lock().requests.remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use nvs_runtime::TaskRoot;

    use super::*;
    use crate::reactor::{Reactor, install, run_until_idle};
    use crate::scheduler::Scheduler;

    /// Three sites: two on line 2 of `a.nvs`, one on line 4.
    fn session() -> (Debugger, mpsc::Receiver<Stop>, Arc<SiteTable>) {
        let sites = Arc::new(SiteTable::new(3));
        let line = |line| SiteLine {
            file: "a.nvs".to_owned(),
            line,
        };
        let (debugger, stops) = Debugger::new(Arc::clone(&sites), vec![line(2), line(2), line(4)]);
        (debugger, stops, sites)
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
        let (debugger, _stops, sites) = session();
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
    fn a_stopped_task_gives_its_core_to_the_others_and_a_resume_continues_it() {
        let (debugger, stops, _sites) = session();
        let (ran, after) = mpsc::channel();
        let controller = debugger.clone();
        let core = std::thread::spawn(move || {
            let mut sched = Scheduler::new();
            let _installed = install(Reactor::new().expect("the OS refused a poll"));
            let status = Rc::new(Cell::new(None));
            let seen = Rc::clone(&status);
            let mut ctx = Ctx::buffered();
            controller.attach(&mut ctx);
            sched.spawn(ctx, TaskRoot::Request, move |ctx| seen.set(Some(site(ctx))));
            sched.spawn(Ctx::buffered(), TaskRoot::Request, move |_| {
                ran.send("the other task ran").expect("the test is waiting");
            });
            let report = run_until_idle(&mut sched).expect("the loop failed");
            (status.get(), report.parked)
        });

        let stop = stops.recv().expect("the task stopped");
        assert_eq!(stop.reason, StopReason::Pause, "no frame, so no breakpoint");
        assert!(stop.task.is_some());
        assert_eq!(after.recv(), Ok("the other task ran"));
        debugger.resume();
        let (status, parked) = core.join().expect("the core ended");
        assert_eq!((status, parked), (Some(nvs_runtime::OK), 0));
    }

    #[test]
    fn a_pause_stops_an_attached_request_at_its_next_poll() {
        let (debugger, stops, _sites) = session();
        let controller = debugger.clone();
        let (attached, ready) = mpsc::channel();
        let (paused, go) = mpsc::channel();
        let program = std::thread::spawn(move || {
            let mut ctx = Ctx::buffered();
            controller.attach(&mut ctx);
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
        assert!(
            debugger.shared.lock().requests.is_empty(),
            "a dropped context detaches"
        );
    }
}
