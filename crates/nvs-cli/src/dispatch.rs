//! `nvs serve` under the Windows service control manager: the dispatcher half
//! of `rule:packaging/a-service-answers-its-manager`, whose answering half is
//! [`crate::service::hosted`].
//!
//! The SCM starts the stored `ImagePath` (`crate::service::image_path`) as an
//! ordinary process and waits for that process to connect back through
//! `StartServiceCtrlDispatcherW`. A process that never does is ended at the
//! start timeout with error 1053, whatever it was serving — which is what an
//! installed service did before this module existed. So `serving` connects
//! first: under the SCM the dispatcher runs `platform::service_main` on a
//! thread of its own and returns when it ends, and on a console the connect
//! fails with `ERROR_FAILED_SERVICE_CONTROLLER_CONNECT` and the caller gets
//! its Rust closure back to run inline, which is `nvs serve` on a terminal exactly
//! as before. `nvs service run <name>` never comes here: it is the stored
//! argv run on a terminal, on purpose (`crate::service`).
//!
//! **What the SCM is told is [`Report`], and it is produced by the two seams
//! the rest of this binary already reports through.** [`Machine`] is a
//! [`Supervisor`] for the boot's `Ready` and the drain's `Stopping` — the
//! same two states a `Type=notify` unit is told on Linux — and a
//! [`hosted::Reporting`] for the checkpoints a stop advances. Both are values
//! on every platform so that a case drives the whole machine without an SCM,
//! which is [`hosted`]'s own reason; only `platform` is Windows, and it is
//! the thinnest layer there is: one struct into `SetServiceStatus`.
//!
//! **A drain a manager asked for is the same drain a signal begins.** The
//! handler answers `STOP` and `PRESHUTDOWN` through [`hosted::stop`], which
//! enters [`crate::stop::deliver_to`] — the one drain — and watches it on the
//! manager's behalf. The process it counts in-flight requests over is what
//! `nvs serve` installed ([`crate::reload::installed`]); a stop that arrives
//! before the boot got that far begins the drain directly and reports it,
//! since there is nothing in flight yet to count. The SCM is not told this
//! service accepts `PARAMCHANGE`: a reload starts only when the server's own
//! check notices a saved configuration file (`crate::reload`).
//!
//! What it spends: the dispatcher's own thread for the life of the process,
//! one short-lived thread per stop, and one status struct. Nothing per
//! request.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::service::hosted::{self, Progress};
use crate::service::{State, Supervisor};

/// How long the SCM is asked to wait for the boot — every listener bound and
/// every mounted entry compiled — before it gives up on the start.
///
/// Reported once, with the one checkpoint a boot has: the boot reports no
/// progress of its own, and a hint that is re-sent without a checkpoint
/// advancing is one the SCM stops believing.
const BOOT_WAIT: Duration = Duration::from_secs(120);

/// One thing the service manager is told, and the whole set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Report {
    /// The boot is running; `wait_hint` is how long the manager should give
    /// it.
    StartPending { wait_hint: Duration },
    /// Serving, and accepting a stop and a preshutdown.
    Running,
    /// The drain is running; `checkpoint` advances on every report and never
    /// repeats.
    StopPending { checkpoint: u32 },
    /// The process is about to exit. `exit` is `0` for a clean end and the
    /// service-specific code the event log shows otherwise.
    Stopped { exit: u32 },
}

/// Where a [`Report`] goes — `SetServiceStatus` on Windows, a list in a case.
pub(crate) trait Reporter: std::fmt::Debug + Send + Sync {
    /// Tell the manager `report`.
    fn report(&self, report: Report);
}

/// Which record a moment is, for the event log: the three lifecycle records,
/// and the two streams a process with no console would otherwise write into
/// nothing.
///
/// The ids are the message table's (`build/winres.rs`, `MESSAGE_IDS`), and
/// every record's text is its one insertion string. Id 4 is in the table and
/// no record here uses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    all(test, not(windows)),
    expect(
        dead_code,
        reason = "the cases construct the two records an exit code maps to; only Windows \
                  writes the other three"
    )
)]
pub(crate) enum Lifecycle {
    /// The boot finished and the service is serving.
    Started,
    /// The service ended cleanly.
    Stopped,
    /// The service ended before it ever served.
    FailedToStart,
    /// One line the server wrote to its standard output — what a terminal
    /// would have shown.
    Stdout,
    /// One line the server wrote to its standard error: a warning, a refused
    /// configuration, a diagnostic.
    Stderr,
}

/// The status machine: what each event this process already reports maps to.
///
/// The checkpoint is this type's, advanced on every pending report whichever
/// seam produced it, because the SCM reads a checkpoint that stands still as a
/// stop that has stalled — and a stop is reported by two seams at once, the
/// signal's `STOPPING=1` and the drain watcher's progress.
#[derive(Debug)]
pub(crate) struct Machine {
    reporter: Arc<dyn Reporter>,
    checkpoint: AtomicU32,
    /// Whether `Running` was ever reported, which is what separates a stop
    /// from a start that failed.
    served: AtomicBool,
    /// The last report, for an `INTERROGATE`.
    last: Mutex<Option<Report>>,
}

impl Machine {
    /// A machine reporting through `reporter`, before anything has happened.
    pub(crate) fn new(reporter: Arc<dyn Reporter>) -> Self {
        Self {
            reporter,
            checkpoint: AtomicU32::new(0),
            served: AtomicBool::new(false),
            last: Mutex::new(None),
        }
    }

    fn send(&self, report: Report) {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(report);
        self.reporter.report(report);
    }

    fn pending(&self) -> Report {
        Report::StopPending {
            checkpoint: self.checkpoint.fetch_add(1, Ordering::Relaxed) + 1,
        }
    }

    /// The boot has begun.
    pub(crate) fn started(&self) {
        self.send(Report::StartPending {
            wait_hint: BOOT_WAIT,
        });
    }

    /// Which lifecycle record a server that returned `code` is: a clean end,
    /// or a start that failed because nothing was ever served.
    pub(crate) fn lifecycle_of(&self, code: ExitCode) -> Lifecycle {
        if code == ExitCode::SUCCESS || self.served.load(Ordering::Relaxed) {
            Lifecycle::Stopped
        } else {
            Lifecycle::FailedToStart
        }
    }

    /// The server returned `code`, and the process is about to exit.
    ///
    /// **Everything the process still owes is done before this**, because the
    /// dispatcher returns on the main thread the moment the last service
    /// reports `STOPPED`, and the process exits with it — a record written or
    /// an outcome stored after this report is lost.
    pub(crate) fn ended(&self, code: ExitCode) {
        self.send(Report::Stopped {
            exit: u32::from(code != ExitCode::SUCCESS),
        });
    }

    /// Say again what was last said, which is what an `INTERROGATE` is owed.
    pub(crate) fn interrogated(&self) {
        let last = *self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(report) = last {
            self.reporter.report(report);
        }
    }
}

impl Supervisor for Machine {
    /// `Ready` is `Running`, `Stopping` is the first `StopPending`, and a
    /// reload has no state in the SCM's set — nor does the fleet's heartbeat,
    /// which is what a `WatchdogSec=` unit is owed and not this manager.
    fn told(&self, state: State) {
        match state {
            State::Ready => {
                self.served.store(true, Ordering::Relaxed);
                self.send(Report::Running);
            }
            State::Stopping => {
                let pending = self.pending();
                self.send(pending);
            }
            State::Reloading | State::Alive => {}
        }
    }
}

impl hosted::Reporting for Machine {
    /// Every checkpoint the drain watcher reports advances this machine's own
    /// count; `Stopped` from the watcher is not `Stopped` to the manager,
    /// because the process is still tearing down and [`Machine::ended`] is
    /// the one place that report is made.
    fn told(&self, progress: Progress) {
        match progress {
            Progress::Stopping { .. } => {
                let pending = self.pending();
                self.send(pending);
            }
            Progress::Stopped => {}
        }
    }
}

/// Runs `run` under the SCM if this process was started by it, or hands it
/// back for a console run.
///
/// `Ok` is the exit code the service ended with; `Err` is the Rust closure, untouched,
/// because the connect failed the one way that means *no manager here*. Any
/// other failure to connect is reported and is a failed start.
#[cfg(windows)]
pub(crate) fn serving(
    run: Box<dyn FnOnce() -> ExitCode + Send>,
) -> Result<ExitCode, Box<dyn FnOnce() -> ExitCode + Send>> {
    platform::serving(run)
}

#[cfg(windows)]
pub(crate) mod platform {
    //! The Windows layer: the connect, the handler, the status struct and the
    //! event-log record. Everything that reasons is above this module.

    use std::process::ExitCode;
    use std::sync::{Arc, Mutex, OnceLock, PoisonError};
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{
        ERROR_CALL_NOT_IMPLEMENTED, ERROR_FAILED_SERVICE_CONTROLLER_CONNECT,
        ERROR_SERVICE_SPECIFIC_ERROR, FALSE, NO_ERROR,
    };
    use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle};
    use windows_sys::Win32::System::EventLog::{
        DeregisterEventSource, EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE,
        EVENTLOG_WARNING_TYPE, RegisterEventSourceW, ReportEventW,
    };
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Services::{
        RegisterServiceCtrlHandlerExW, SERVICE_ACCEPT_PRESHUTDOWN, SERVICE_ACCEPT_STOP,
        SERVICE_CONTROL_INTERROGATE, SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STATUS,
        SERVICE_STATUS_HANDLE, SERVICE_STOP_PENDING, SERVICE_STOPPED, SERVICE_TABLE_ENTRYW,
        SERVICE_WIN32_OWN_PROCESS, SetServiceStatus, StartServiceCtrlDispatcherW,
    };

    use super::{Lifecycle, Machine, Report, Reporter};
    use crate::service::hosted;

    /// How long the SCM is asked to wait between two checkpoints of a drain.
    ///
    /// Comfortably above [`STOP_PACE`]'s poll, which is how often a checkpoint
    /// actually advances, so a machine that is busy does not read one late report
    /// as a stop that has stalled.
    const STOP_WAIT: Duration = Duration::from_secs(10);

    /// How a drain the SCM asked for is watched: polled once a second, and
    /// reported finished after the preshutdown window `nvs service install` asks
    /// for whatever is still in flight (`crate::service::registration`).
    const STOP_PACE: hosted::Pace = hosted::Pace {
        poll: Duration::from_secs(1),
        bound: Duration::from_secs(180),
    };

    /// The server to run, parked here between [`serving`] and
    /// [`service_main`], because the dispatcher calls the latter with the
    /// service's argv and nothing of ours.
    static RUN: Mutex<Option<Box<dyn FnOnce() -> ExitCode + Send>>> = Mutex::new(None);

    /// What the server returned, carried back from [`service_main`]'s thread.
    static OUTCOME: Mutex<Option<ExitCode>> = Mutex::new(None);

    /// The machine the handler reports through, set once by [`service_main`].
    static MACHINE: OnceLock<Arc<Machine>> = OnceLock::new();

    /// The service's name as the SCM spelled it, NUL-terminated, for the
    /// event-log source registered under it at install.
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();

    /// The status handle, which is the SCM's and outlives every report, and
    /// whether the *started* record was written yet.
    #[derive(Debug)]
    struct Scm {
        handle: usize,
        started: std::sync::atomic::AtomicBool,
    }

    impl Reporter for Scm {
        #[expect(
            unsafe_code,
            reason = "`SetServiceStatus` is the platform's only way to report a service's state"
        )]
        fn report(&self, report: Report) {
            let status = status_of(report);
            // SAFETY: a handle the SCM issued to this process and never
            // revokes, and a status struct that outlives the call. Nothing
            // is done with a failure, for `Supervisor::told`'s reason: a
            // server that stopped serving because its manager stopped
            // listening would be an outage bought for a status line.
            unsafe { SetServiceStatus(self.handle as SERVICE_STATUS_HANDLE, &raw const status) };
            // Once: `Running` is reported again after every reload, and the
            // record is about the boot.
            if report == Report::Running
                && !self
                    .started
                    .swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                log(Lifecycle::Started, "the service is serving");
            }
        }
    }

    /// One [`Report`] as the struct the SCM reads.
    fn status_of(report: Report) -> SERVICE_STATUS {
        let mut status = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: SERVICE_STOPPED,
            dwControlsAccepted: 0,
            dwWin32ExitCode: NO_ERROR,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 0,
        };
        match report {
            Report::StartPending { wait_hint } => {
                status.dwCurrentState = SERVICE_START_PENDING;
                status.dwCheckPoint = 1;
                status.dwWaitHint = millis(wait_hint);
            }
            Report::Running => {
                status.dwCurrentState = SERVICE_RUNNING;
                status.dwControlsAccepted = SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_PRESHUTDOWN;
            }
            Report::StopPending { checkpoint } => {
                status.dwCurrentState = SERVICE_STOP_PENDING;
                status.dwCheckPoint = checkpoint;
                status.dwWaitHint = millis(STOP_WAIT);
            }
            Report::Stopped { exit } => {
                status.dwCurrentState = SERVICE_STOPPED;
                if exit != 0 {
                    status.dwWin32ExitCode = ERROR_SERVICE_SPECIFIC_ERROR;
                    status.dwServiceSpecificExitCode = exit;
                }
            }
        }
        status
    }

    /// A duration as the milliseconds the SCM counts in, saturating.
    fn millis(duration: std::time::Duration) -> u32 {
        u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)
    }

    /// [`super::serving`], on the platform.
    #[expect(
        unsafe_code,
        reason = "`StartServiceCtrlDispatcherW` is the platform's only way to be a service"
    )]
    pub(crate) fn serving(
        run: Box<dyn FnOnce() -> ExitCode + Send>,
    ) -> Result<ExitCode, Box<dyn FnOnce() -> ExitCode + Send>> {
        *RUN.lock().unwrap_or_else(PoisonError::into_inner) = Some(run);
        // Ignored for a `SERVICE_WIN32_OWN_PROCESS` service, but may not be
        // null; `service_main` is handed the real name as its first argument.
        let empty: [u16; 1] = [0];
        let table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: empty.as_ptr().cast_mut(),
                lpServiceProc: Some(service_main),
            },
            SERVICE_TABLE_ENTRYW {
                lpServiceName: std::ptr::null_mut(),
                lpServiceProc: None,
            },
        ];
        // SAFETY: a table of two entries the last of which is the null
        // terminator, both outliving the call, which returns only once the
        // service has ended or the connect failed.
        let connected = unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) };
        if connected == FALSE {
            let error = std::io::Error::last_os_error();
            let run = RUN
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take()
                .expect(
                    "the service's Rust closure is taken only by `service_main`, which did not run",
                );
            if error.raw_os_error()
                == Some(i32::try_from(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT).unwrap_or(i32::MAX))
            {
                return Err(run);
            }
            eprintln!("error: could not connect to the service control manager: {error}");
            return Ok(ExitCode::FAILURE);
        }
        Ok(OUTCOME
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .unwrap_or(ExitCode::FAILURE))
    }

    /// The service's entry point, on the dispatcher's thread: register the
    /// handler, report the boot, run the server here, report the end.
    #[expect(
        unsafe_code,
        reason = "the platform calls this with a C argv, so its signature is the platform's"
    )]
    pub(crate) unsafe extern "system" fn service_main(argc: u32, argv: *mut *mut u16) {
        let name = if argc > 0 && !argv.is_null() {
            // SAFETY: the SCM hands `argc` NUL-terminated wide strings, the
            // first of which is the service's name.
            unsafe { wide_of(*argv) }
        } else {
            vec![0]
        };
        drop(NAME.set(name.clone()));
        // SAFETY: a NUL-terminated name that outlives the call and a handler
        // with the signature the platform declares; no context is passed.
        let handle = unsafe {
            RegisterServiceCtrlHandlerExW(name.as_ptr(), Some(handler), std::ptr::null())
        };
        let run = RUN.lock().unwrap_or_else(PoisonError::into_inner).take();
        if handle.is_null() {
            eprintln!(
                "error: could not register the service's control handler: {}",
                std::io::Error::last_os_error()
            );
            *OUTCOME.lock().unwrap_or_else(PoisonError::into_inner) = Some(ExitCode::FAILURE);
            return;
        }
        // Before anything is written: from here the server's console is the
        // event log, so a refused configuration is read where an
        // administrator looks rather than lost with a handle nobody holds.
        capture(STD_OUTPUT_HANDLE, Lifecycle::Stdout);
        capture(STD_ERROR_HANDLE, Lifecycle::Stderr);
        let machine = Arc::new(Machine::new(Arc::new(Scm {
            handle: handle as usize,
            started: std::sync::atomic::AtomicBool::new(false),
        })));
        drop(MACHINE.set(Arc::clone(&machine)));
        machine.started();
        // What `serve::run` reports `READY=1` and `STOPPING=1` to, installed
        // before it runs so its `Notify::from_env` finds this rather than
        // silence (`crate::service::Notify`).
        let supervisor: Arc<dyn crate::service::Supervisor> = machine.clone();
        crate::service::Notify::to(supervisor).install();
        let code = match run {
            Some(run) => run(),
            None => ExitCode::FAILURE,
        };
        // The record and the outcome first, the `STOPPED` report last:
        // `Machine::ended` owns why that order is the only one that works.
        log(
            machine.lifecycle_of(code),
            &format!(
                "the service ended with exit code {}",
                u32::from(code != ExitCode::SUCCESS)
            ),
        );
        *OUTCOME.lock().unwrap_or_else(PoisonError::into_inner) = Some(code);
        machine.ended(code);
    }

    /// The control handler, on a thread of the SCM's: it must return
    /// promptly, so anything that waits is given a thread of its own.
    #[expect(
        unsafe_code,
        reason = "the platform calls this, so its signature is the platform's"
    )]
    unsafe extern "system" fn handler(
        control: u32,
        _event: u32,
        _data: *mut core::ffi::c_void,
        _context: *mut core::ffi::c_void,
    ) -> u32 {
        let Some(machine) = MACHINE.get() else {
            return ERROR_CALL_NOT_IMPLEMENTED;
        };
        // `hosted::stops` is the mapping the rule's table is held to; what it
        // does not name is either the one control every service answers, by
        // saying again what it last said, or one this service does not
        // implement.
        if hosted::stops(control) {
            stop(Arc::clone(machine));
            NO_ERROR
        } else if control == SERVICE_CONTROL_INTERROGATE {
            machine.interrogated();
            NO_ERROR
        } else {
            ERROR_CALL_NOT_IMPLEMENTED
        }
    }

    /// Drains on a thread of its own, over the process `nvs serve` installed.
    ///
    /// A stop that arrives before the boot installed one begins the drain
    /// directly: there is nothing in flight to count, and the end of
    /// [`service_main`] is what reports `Stopped`.
    fn stop(machine: Arc<Machine>) {
        let spawned = std::thread::Builder::new()
            .name("nvs-scm-control".to_owned())
            .spawn(move || {
                let Some(process) = crate::reload::installed() else {
                    crate::stop::deliver();
                    return;
                };
                hosted::stop(
                    &*process,
                    &nvs_server::Draining::process(),
                    &*machine,
                    STOP_PACE,
                );
            });
        if spawned.is_err() {
            // No thread to watch the drain, so it is begun here and reported
            // once; the process's end reports the rest.
            crate::stop::deliver();
        }
    }

    /// Points the standard handle `which` at a pipe whose far end writes each
    /// line as one event-log record of kind `as_record`.
    ///
    /// A service has no console, so what the server prints — `listening on …`,
    /// a warning, a refused configuration's diagnostic — is written into a
    /// handle that goes nowhere. This is the SCM's spelling of what systemd
    /// does for every unit: the journal takes stderr. `std` asks the platform
    /// for the standard handle on every write, so the redirect holds for the
    /// process's life, and the reader is one thread per stream that ends with
    /// the process.
    ///
    /// A pipe that cannot be made leaves the handle as it was: the records are
    /// how a failure is read, and a start refused for want of them would be the
    /// failure with no way to read it.
    #[expect(
        unsafe_code,
        reason = "an anonymous pipe and `SetStdHandle` are `kernel32` calls with no spelling in `std`"
    )]
    fn capture(which: u32, as_record: Lifecycle) {
        use std::io::BufRead as _;
        use std::os::windows::io::FromRawHandle as _;

        let mut read = std::ptr::null_mut();
        let mut write = std::ptr::null_mut();
        // SAFETY: two out-parameters this frame owns, no security attributes,
        // the default buffer size.
        if unsafe { CreatePipe(&raw mut read, &raw mut write, std::ptr::null(), 0) } == FALSE {
            return;
        }
        // SAFETY: a handle this process just made; the previous standard
        // handle is a service's, which is nothing.
        if unsafe { SetStdHandle(which, write) } == FALSE {
            // SAFETY: both ends were made above and neither is held by
            // anything else.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(read);
                windows_sys::Win32::Foundation::CloseHandle(write);
            }
            return;
        }
        // SAFETY: the read end, owned from here by the file and closed with it.
        let reader = std::io::BufReader::new(unsafe { std::fs::File::from_raw_handle(read) });
        // Not joined: it ends when the write end does, which is the process
        // ending. A thread that could not be started leaves the lines in the
        // pipe's buffer, which is no worse than the handle they had.
        drop(
            std::thread::Builder::new()
                .name(match as_record {
                    Lifecycle::Stderr => "nvs-scm-stderr".to_owned(),
                    _ => "nvs-scm-stdout".to_owned(),
                })
                .spawn(move || {
                    for line in reader.lines().map_while(Result::ok) {
                        if !line.trim().is_empty() {
                            log(as_record, &line);
                        }
                    }
                }),
        );
    }

    /// The first wide string at `text`, NUL included.
    #[expect(
        unsafe_code,
        reason = "the service's name arrives as a C string from the platform"
    )]
    unsafe fn wide_of(text: *const u16) -> Vec<u16> {
        let mut out = Vec::new();
        let mut at = text;
        // SAFETY: the caller promises a NUL-terminated string.
        unsafe {
            while *at != 0 {
                out.push(*at);
                at = at.add(1);
            }
        }
        out.push(0);
        out
    }

    /// One of § 4's lifecycle records, written to the event-log source
    /// `nvs service install` registered under this service's name.
    ///
    /// The record's text is its one insertion string, so the viewer shows it
    /// with or without a message table behind the source. Nothing is done
    /// with a failure: the log is the second place an administrator looks,
    /// and the first is the service's status, which is already reported.
    #[expect(
        unsafe_code,
        reason = "writing an event-log record is an `advapi32` call with no spelling in `std`"
    )]
    pub(crate) fn log(moment: Lifecycle, text: &str) {
        let Some(name) = NAME.get() else {
            return;
        };
        let (kind, id) = match moment {
            Lifecycle::Started => (EVENTLOG_INFORMATION_TYPE, 1),
            Lifecycle::Stopped => (EVENTLOG_INFORMATION_TYPE, 2),
            Lifecycle::FailedToStart => (EVENTLOG_ERROR_TYPE, 3),

            Lifecycle::Stdout => (EVENTLOG_INFORMATION_TYPE, 5),
            Lifecycle::Stderr => (EVENTLOG_WARNING_TYPE, 6),
        };
        let text: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let strings = [text.as_ptr()];
        // SAFETY: a NUL-terminated source name on the local machine; one
        // NUL-terminated insertion string that outlives the call; no SID and
        // no binary data. The source is closed again whether or not the
        // record was written.
        unsafe {
            let source = RegisterEventSourceW(std::ptr::null(), name.as_ptr());
            if source.is_null() {
                return;
            }
            ReportEventW(
                source,
                kind,
                0,
                id,
                std::ptr::null_mut(),
                1,
                0,
                strings.as_ptr(),
                std::ptr::null(),
            );
            DeregisterEventSource(source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reporter a case reads back.
    #[derive(Debug, Default)]
    struct Recording(Mutex<Vec<Report>>);

    impl Reporter for Recording {
        fn report(&self, report: Report) {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(report);
        }
    }

    fn machine() -> (Machine, Arc<Recording>) {
        let recording = Arc::new(Recording::default());
        (
            Machine::new(Arc::clone(&recording) as Arc<dyn Reporter>),
            recording,
        )
    }

    fn reported(recording: &Recording) -> Vec<Report> {
        recording
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// `rule:packaging/a-service-answers-its-manager`, row 1, through both
    /// seams: the boot is `START_PENDING` then `RUNNING`; a stop is
    /// `STOP_PENDING` from the signal's `STOPPING=1` and again from every
    /// checkpoint the drain watcher reports, with one count that never
    /// repeats; and `STOPPED` is reported once, by the process ending, not by
    /// the watcher finishing.
    #[test]
    fn a_boot_a_stop_and_an_end_are_the_scm_states_in_order() {
        let (machine, recording) = machine();
        machine.started();
        Supervisor::told(&machine, State::Ready);
        Supervisor::told(&machine, State::Stopping);
        hosted::Reporting::told(
            &machine,
            Progress::Stopping {
                checkpoint: 1,
                in_flight: 2,
            },
        );
        hosted::Reporting::told(
            &machine,
            Progress::Stopping {
                checkpoint: 2,
                in_flight: 0,
            },
        );
        hosted::Reporting::told(&machine, Progress::Stopped);
        assert_eq!(machine.lifecycle_of(ExitCode::SUCCESS), Lifecycle::Stopped);
        machine.ended(ExitCode::SUCCESS);
        assert_eq!(
            reported(&recording),
            [
                Report::StartPending {
                    wait_hint: BOOT_WAIT
                },
                Report::Running,
                Report::StopPending { checkpoint: 1 },
                Report::StopPending { checkpoint: 2 },
                Report::StopPending { checkpoint: 3 },
                Report::Stopped { exit: 0 },
            ]
        );
    }

    /// A server that returned before it ever reported `Ready` failed to
    /// start: the SCM is told a service-specific exit, so the failure is in
    /// the System log rather than a start that silently timed out.
    #[test]
    fn a_boot_that_fails_is_a_failed_start_with_a_specific_exit_code() {
        let (machine, recording) = machine();
        machine.started();
        assert_eq!(
            machine.lifecycle_of(ExitCode::FAILURE),
            Lifecycle::FailedToStart
        );
        machine.ended(ExitCode::FAILURE);
        assert_eq!(
            reported(&recording).last(),
            Some(&Report::Stopped { exit: 1 })
        );
    }

    /// A server that served and then failed is a stop, not a failed start.
    #[test]
    fn a_server_that_served_and_then_failed_is_a_stop() {
        let (machine, _recording) = machine();
        machine.started();
        Supervisor::told(&machine, State::Ready);
        assert_eq!(machine.lifecycle_of(ExitCode::FAILURE), Lifecycle::Stopped);
    }

    /// A reload and the fleet's heartbeat have no state in the SCM's set, and
    /// an `INTERROGATE` is answered with what was last said.
    #[test]
    fn a_reload_reports_nothing_and_an_interrogate_repeats_the_last_report() {
        let (machine, recording) = machine();
        Supervisor::told(&machine, State::Ready);
        Supervisor::told(&machine, State::Reloading);
        Supervisor::told(&machine, State::Alive);
        Supervisor::told(&machine, State::Ready);
        machine.interrogated();
        assert_eq!(
            reported(&recording),
            [Report::Running, Report::Running, Report::Running]
        );
    }
}
