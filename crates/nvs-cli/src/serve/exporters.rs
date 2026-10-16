//! `[metrics]` and `[trace]`'s exporters while the server runs: the scrape
//! listener and the two pushes, on the one core the boot gave them, rebuilt
//! when a reload changes either block (`rule:config/reloadability-is-its-own-field`).
//!
//! **Each exporter runs under a drain of its own.** [`Exporters::watch`] is a
//! task on that core, and every exporter is a child task of it holding a
//! detached `nvs_server::Draining`. Every [`POLL`], the watcher loads the
//! published snapshot. Where it moved, both blocks are resolved again through
//! the same functions the boot used (`super::scrape_address`,
//! `super::trace_collector`, `super::metrics_collector`):
//!
//! - An exporter whose address or endpoint is unchanged keeps running.
//! - One that changed is replaced. The new one is started first, so a new
//!   scrape socket is bound before the old one closes, and then the old one's
//!   drain begins. A push that is stopped pushes what it holds on its way out,
//!   exactly as it does when the process stops.
//! - One the new tree no longer names is stopped the same way.
//! - **A change that cannot be followed keeps the exporter that is running.** A
//!   scrape address that will not bind, or an `otlp` exporter with no endpoint,
//!   is logged and changes nothing. The reload that published the tree has
//!   already answered, so the log is the only place this is said.
//!
//! **A process stop ends them all.** The watcher begins every exporter's drain
//! and waits for each to end, so the final push a stop owes still happens.
//!
//! The registry each core counts into is not rebuilt here:
//! `nvs_server::serve_on_this_core` meters its core again when the snapshot it
//! serves under moves, and adopts the registry it holds, so no counter resets.
//!
//! Cost: one snapshot load per [`POLL`] on one core, and during a replacement
//! two exporters of one kind, for as long as the old one's last push takes.

use std::cell::Cell;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::server::Waits;
use nvs_runtime::host::Woken;
use nvs_runtime::{Ctx, OutputSink, TaskRoot};
use nvs_server::{Draining, Endpoint, Signal};

use super::Socket;

/// The longest the watcher waits before it looks at the published snapshot
/// again, which is how long a reload's new `[metrics]` or `[trace]` takes to
/// reach the exporters. A process stop does not wait for it.
const POLL: Duration = Duration::from_secs(1);

/// How often the watcher looks at whether the exporters it stopped have ended.
const REAPED: Duration = Duration::from_millis(50);

/// One running exporter: what it was started for, and the drain that stops it.
///
/// Dropping it begins that drain, so replacing an `Option<Running>` stops the
/// exporter it held.
struct Running<K> {
    key: K,
    stop: Draining,
}

impl<K> Drop for Running<K> {
    fn drop(&mut self) {
        self.stop.begin();
    }
}

/// Counts one exporter task out when it ends, however it ends.
struct Live(Rc<Cell<usize>>);

impl Drop for Live {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// The exporters on this core, and what each was started for.
pub(super) struct Exporters {
    waits: Waits,
    /// How many exporter tasks have not ended yet, replaced ones included.
    live: Rc<Cell<usize>>,
    /// Set when a scrape loop stops on its own listener, which fails the core
    /// as a stopped accept loop does.
    stopped: Rc<Cell<bool>>,
    scrape: Option<Running<SocketAddr>>,
    traces: Option<Running<String>>,
    series: Option<Running<String>>,
}

impl Exporters {
    /// No exporter yet. `stopped` is the core's own flag for an accept loop
    /// that stopped on its listener.
    pub(super) fn new(waits: Waits, stopped: Rc<Cell<bool>>) -> Self {
        Self {
            waits,
            live: Rc::new(Cell::new(0)),
            stopped,
            scrape: None,
            traces: None,
            series: None,
        }
    }

    /// Starts what the boot resolved from `booted`, then follows `current`
    /// until `draining` begins, and returns once every exporter has ended.
    ///
    /// Called on a task, because every exporter is spawned as its child.
    pub(super) fn watch(
        mut self,
        current: &nvs_config::Current,
        booted: Arc<nvs_config::Snapshot>,
        scrapes: Option<Socket>,
        traces: Option<Endpoint>,
        series: Option<Endpoint>,
        draining: &Draining,
    ) {
        let exporters = &mut self;
        // The boot resolved this same tree, so the address is known to resolve.
        if let (Some(socket), Ok(Some(address))) = (scrapes, super::scrape_address(&booted.config))
        {
            exporters.scrape = exporters.scraping(address, socket);
        }
        exporters.traces = traces.and_then(|endpoint| exporters.pushing(endpoint, Signal::Traces));
        exporters.series = series.and_then(|endpoint| exporters.pushing(endpoint, Signal::Metrics));

        let mut seen = booted;
        loop {
            {
                // A stop wakes the wait, so it is not delayed by `POLL`.
                let _woken = nvs_host::wake_at_drain(draining.bit());
                if !draining.is_draining()
                    && matches!(
                        nvs_host::timer::wait_until(Instant::now() + POLL),
                        Woken::Cancelled
                    )
                {
                    break;
                }
            }
            if draining.is_draining() {
                break;
            }
            let published = current.load();
            if Arc::ptr_eq(&published, &seen) {
                continue;
            }
            seen = published;
            exporters.follow(&seen.config);
        }

        exporters.scrape = None;
        exporters.traces = None;
        exporters.series = None;
        while exporters.live.get() > 0 {
            if matches!(nvs_host::sleep(REAPED), Woken::Cancelled) {
                break;
            }
        }
    }

    /// Brings every exporter in line with `config`.
    fn follow(&mut self, config: &nvs_config::Config) {
        match super::scrape_address(config) {
            Err(refusal) => kept("[metrics]", &refusal),
            Ok(wanted) if wanted == self.scrape.as_ref().map(|running| running.key) => {}
            Ok(None) => {
                self.scrape = None;
                println!("scrapes are no longer answered");
            }
            Ok(Some(address)) => match super::scrape_bound(address) {
                Err(refusal) => kept("[metrics]", &refusal),
                Ok(socket) => {
                    if let Some(started) = self.scraping(address, socket) {
                        self.scrape = Some(started);
                    }
                }
            },
        }
        let traces = resolved(super::trace_collector(config), Signal::Traces);
        if let Some(replaced) = self.replaced(self.traces.as_ref(), traces, "[trace]") {
            self.traces = replaced;
        }
        let series = resolved(super::metrics_collector(config), Signal::Metrics);
        if let Some(replaced) = self.replaced(self.series.as_ref(), series, "[metrics]") {
            self.series = replaced;
        }
    }

    /// The push that `wanted` asks for in place of `running`, or `None` where
    /// `running` stays: the endpoint did not change, or the change cannot be
    /// followed.
    fn replaced(
        &self,
        running: Option<&Running<String>>,
        wanted: Result<Option<(Endpoint, Signal)>, String>,
        block: &str,
    ) -> Option<Option<Running<String>>> {
        let wanted = match wanted {
            Ok(wanted) => wanted,
            Err(refusal) => {
                kept(block, &refusal);
                return None;
            }
        };
        let key = wanted.as_ref().map(|(endpoint, _)| endpoint.to_string());
        if key.as_deref() == running.map(|running| running.key.as_str()) {
            return None;
        }
        match wanted {
            None => {
                println!("{block} no longer pushes");
                Some(None)
            }
            Some((endpoint, signal)) => self.pushing(endpoint, signal).map(Some),
        }
    }

    /// Starts the scrape loop on `socket`, which is bound to `address`.
    fn scraping(&self, address: SocketAddr, socket: Socket) -> Option<Running<SocketAddr>> {
        let named = socket.named();
        let mut listener = match socket.accepting() {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("error: this core could not take the scrape socket: {error}");
                return None;
            }
        };
        let waits = self.waits;
        let stopped = Rc::clone(&self.stopped);
        let answering = named.clone();
        let stop = self.spawn(move |stop| {
            let served = nvs_server::serve_scrapes_on_this_core(
                &mut *listener,
                waits,
                stop,
                |note| eprintln!("note: {note}"),
                || ControlFlow::Continue(()),
            );
            if let Err(error) = served {
                eprintln!("error: the scrape loop on {answering} stopped: {error}");
                stopped.set(true);
            }
        })?;
        println!("scrapes answered on {named}");
        Some(Running { key: address, stop })
    }

    /// Starts the push of `signal` to `endpoint`. It is the only thing in this
    /// process that dials that collector: a request hands its spans to the
    /// queue and is done with them, which keeps a collector's latency off every
    /// response.
    fn pushing(&self, endpoint: Endpoint, signal: Signal) -> Option<Running<String>> {
        let key = endpoint.to_string();
        let waits = self.waits;
        let stop = self.spawn(move |stop| {
            let report = |note: &str| eprintln!("note: {note}");
            match signal {
                Signal::Traces => {
                    nvs_server::push_queued_on_this_core(&endpoint, waits, stop, report)
                }
                Signal::Metrics => {
                    nvs_server::push_registry_on_this_core(&endpoint, waits, stop, report);
                }
            }
        })?;
        match signal {
            Signal::Traces => println!("traces pushed to {key}"),
            Signal::Metrics => println!("series pushed to {key}"),
        }
        Some(Running { key, stop })
    }

    /// Spawns `body` as a child of this task under a drain of its own, and
    /// returns that drain.
    fn spawn(&self, body: impl FnOnce(&Draining) + 'static) -> Option<Draining> {
        let stop = Draining::detached();
        let theirs = stop.clone();
        self.live.set(self.live.get() + 1);
        let live = Live(Rc::clone(&self.live));
        let spawned =
            nvs_host::spawn_child(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                let _live = live;
                body(&theirs);
            });
        // A Rust closure that was never run was dropped with its count.
        spawned.map(|_| stop)
    }
}

/// A push resolved from what the tree wrote, with the signal it carries.
fn resolved(
    written: Result<Option<String>, String>,
    signal: Signal,
) -> Result<Option<(Endpoint, Signal)>, String> {
    written?
        .as_deref()
        .map(|written| Endpoint::of(written, signal).map(|endpoint| (endpoint, signal)))
        .transpose()
}

/// Says that a reload's `block` could not be applied to its exporter, and that
/// the running one stays.
fn kept(block: &str, refusal: &str) {
    eprintln!(
        "error: the reloaded `{block}` is not applied to its exporter, which keeps running as it was: {refusal}"
    );
}
