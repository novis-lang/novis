//! `nvs run`'s wall clock: the thread that stops a run at its `[limits] wall_time`
//! (`rule:errors/on-limit`).
//!
//! A served request is stopped at its `wall_time` by `nvs_host::watchdog`'s sweep, which
//! reads the deadline once per interval. A run is timed to the instant: one thread that
//! sleeps until the run's deadline, then raises [`nvs_runtime::SafepointFlags::WALL_TIME`] and expires the
//! deadline through the run's [`SafepointView`]. The flag stops compiled code at its next
//! poll, and the deadline stops a helper loop already running. A sleeping task reaches
//! neither, so `Core\Time::sleep` ends at the deadline itself. The `Core\Fatal::onLimit`
//! handler runs with the flag lowered, and the thread raises it once more after the
//! default `fatal_reserve_time`, which is the handler's slice.
//!
//! The deadline is the tree's ([`SafepointView::wall_deadline`]), so `Core\Config::set`
//! may move it. The thread files a wake through [`SafepointView::watch_wall_time`] before
//! each wait and reads the deadline again when it fires. A run that starts with no
//! `wall_time` starts no thread: the same wake starts one, the first time a `wall_time`
//! is set.
//!
//! **What it spends:** one thread for the length of a run that has a `wall_time`, asleep
//! until the deadline or until the number moves, and nothing for a run that never has one.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use nvs_runtime::{SafepointFlags, SafepointView};

/// What the clock's thread is told.
enum Event {
    /// The run's `wall_time` moved, so the deadline is read again.
    Moved,
    /// The run is over.
    Stop,
}

/// The clock for one run, stopped and joined when this drops.
pub(crate) struct WallClock {
    view: SafepointView,
    stop: Sender<Event>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Starts the clock for the run `view` is a view of: at once for a run with a
/// `wall_time`, and otherwise the first time one is set.
pub(crate) fn start(view: SafepointView) -> WallClock {
    let (stop, events) = std::sync::mpsc::channel();
    let thread = Arc::new(Mutex::new(None));
    let begin = {
        let view = view.clone();
        let moved = stop.clone();
        let thread = Arc::clone(&thread);
        move || {
            let spawned = std::thread::Builder::new()
                .name("nvs-wall-clock".to_owned())
                .spawn(move || watch(&view, &moved, &events));
            // A thread the platform will not start leaves the run under no
            // wall-time limit, as a platform with no CPU clock leaves it under no
            // CPU limit.
            if let Ok(handle) = spawned {
                *thread.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
            }
        }
    };
    if view.wall_deadline().is_some() {
        begin();
    } else {
        view.watch_wall_time(Box::new(begin));
    }
    WallClock { view, stop, thread }
}

/// The thread's body: waits for the deadline, reading it again each time it
/// moves, and stops the run when it passes.
fn watch(view: &SafepointView, moved: &Sender<Event>, events: &Receiver<Event>) {
    loop {
        // Filed before the deadline is read, so a move between the two still
        // sends the thread round again.
        let wake = moved.clone();
        view.watch_wall_time(Box::new(move || {
            let _ = wake.send(Event::Moved);
        }));
        let event = match view.wall_deadline() {
            Some(at) => {
                events.recv_timeout(at.saturating_duration_since(std::time::Instant::now()))
            }
            None => events.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match event {
            Ok(Event::Moved) => {}
            Ok(Event::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {
                view.request(SafepointFlags::WALL_TIME);
                view.expire_deadline();
                // `rule:errors/on-limit`'s handler runs with the flag lowered, so
                // it is raised once more after the default reserve. That stops a
                // handler that overruns its slice.
                let rest = events.recv_timeout(nvs_config::defaults::LIMITS_FATAL_RESERVE_TIME);
                if !matches!(rest, Ok(Event::Stop)) {
                    view.request(SafepointFlags::WALL_TIME);
                }
                return;
            }
        }
    }
}

impl Drop for WallClock {
    fn drop(&mut self) {
        // The wake still filed holds a sender, and for a run that never set a
        // `wall_time` it is the closure that would have started the thread.
        drop(self.view.unwatch_wall_time());
        let _ = self.stop.send(Event::Stop);
        let handle = self
            .thread
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}
