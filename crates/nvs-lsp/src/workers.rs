//! The threads that analyse the index's waiting entries while the server's
//! own thread answers the client.
//!
//! `rule:ide/the-server-is-synchronous` keeps the store, the index and every
//! answer on one thread. This module moves only the analysis of a waiting
//! entry off it. A worker takes a [`Job`], analyses the entry through the
//! job's [`Snapshot`] with a session and a configuration cache of its own, and
//! sends back the [`Analysis`], which owns everything it holds. It never sends
//! the `Analysed`, which shares its interner through an `Rc`. The index files
//! each result on the server's thread in entry path order
//! ([`crate::SymbolIndex::advance`]), so the index is the one the serial loop
//! builds.
//!
//! There are `available_parallelism() - 1` workers, and at least one. They are
//! started when the first job is sent and stopped once no entry waits, so a
//! settled server has no worker thread.
//!
//! # What it costs
//!
//! One session and one configuration cache per worker, for as long as
//! entries wait, freed when the queue empties. Each job copies the open
//! buffers' text and shares one copy of the set of indexed files with every
//! job sent while the index did not change. That memory buys the latency of a
//! cold index, which the priority ordering allows.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use rustc_hash::FxHashSet;

use crate::document::{Entry, Snapshot};
use crate::index::{Analysis, analyse_entry};
use crate::session::Session;
use crate::tree_cache::TreeCache;

/// One entry for a worker to analyse.
pub(crate) struct Job {
    /// The entry point, under `canonical_key`.
    pub(crate) entry: PathBuf,
    /// What the analysis reads from the store, taken when the job was sent.
    pub(crate) snapshot: Snapshot,
    /// The version of the entry's open buffer, or `NO_BUFFER`.
    pub(crate) version: i32,
    /// Every file the index held when the job was sent.
    pub(crate) indexed: Arc<FxHashSet<PathBuf>>,
    /// The files the entry itself owned then. Its turn drops them before the
    /// result is filed, so the worker records them again.
    pub(crate) owned: Vec<PathBuf>,
    /// The index's change counter when the job was sent.
    pub(crate) change: u64,
}

/// What a worker sends back for one [`Job`].
pub(crate) struct Done {
    /// The job's entry.
    pub(crate) entry: PathBuf,
    /// The job's change counter.
    pub(crate) change: u64,
    /// The analysis, or `None` when the entry could not be loaded.
    pub(crate) analysis: Option<Analysis>,
}

/// A result is sent from the worker that built it to the server's thread.
const _: () = {
    const fn sendable<T: Send>() {}
    sendable::<Job>();
    sendable::<Done>();
};

/// The workers, the jobs they run, and the results that wait for their turn.
pub(crate) struct Pool {
    /// How many workers run at most.
    size: usize,
    /// The job channel. `None` while no worker runs, and dropping it is what
    /// stops them.
    jobs: Option<(Sender<Job>, Receiver<Job>)>,
    /// The side the workers send results on, cloned into each.
    done: Sender<Done>,
    /// The side the server's thread reads results from.
    results: Receiver<Done>,
    /// One handle per running worker.
    threads: Vec<JoinHandle<()>>,
    /// Each entry sent and not yet filed: the change counter its job carried,
    /// and its result once it arrived.
    in_flight: BTreeMap<PathBuf, (u64, Option<Done>)>,
    /// The last entry sent ahead of its turn, so the next look for one starts
    /// after it.
    pub(crate) cursor: Option<PathBuf>,
    /// The set of indexed files the last job shared, and the index generation
    /// it was copied at.
    pub(crate) view: Option<(u64, Arc<FxHashSet<PathBuf>>)>,
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pool")
            .field("size", &self.size)
            .field("running", &self.threads.len())
            .field("in_flight", &self.in_flight.len())
            .finish_non_exhaustive()
    }
}

impl Pool {
    /// A pool of `size` workers, at least one, none of them started.
    pub(crate) fn new(size: usize) -> Self {
        let (done, results) = crossbeam_channel::unbounded();
        Self {
            size: size.max(1),
            jobs: None,
            done,
            results,
            threads: Vec::new(),
            in_flight: BTreeMap::new(),
            cursor: None,
            view: None,
        }
    }

    /// `available_parallelism() - 1` workers, and at least one: the server's
    /// own thread keeps a core for the client.
    pub(crate) fn sized_to_the_machine() -> Self {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        Self::new(cores.saturating_sub(1))
    }

    /// Whether another job can be sent now.
    pub(crate) fn has_room(&self) -> bool {
        self.in_flight.len() < self.size
    }

    /// Sends `job`, and starts a worker for it when fewer than the pool's size
    /// run. Returns `false` when no worker runs and none could be started, and
    /// then the job is not in flight.
    pub(crate) fn send(&mut self, job: Job) -> bool {
        self.in_flight.insert(job.entry.clone(), (job.change, None));
        let (sender, receiver) = self
            .jobs
            .get_or_insert_with(crossbeam_channel::unbounded);
        if self.threads.len() < self.in_flight.len() {
            let receiver = receiver.clone();
            let done = self.done.clone();
            let started = std::thread::Builder::new()
                .name("nvs-lsp-index".to_owned())
                .spawn(move || work(&receiver, &done));
            // A thread the system would not start leaves the job in the
            // channel for the workers that run. With none, the job is
            // analysed on the server's thread at its turn.
            match started {
                Ok(thread) => self.threads.push(thread),
                Err(_) if self.threads.is_empty() => {
                    self.in_flight.remove(&job.entry);
                    return false;
                }
                Err(_) => {}
            }
        }
        // The receiving side is kept here beside the sender, so a send fails
        // only when the pool is gone.
        sender.send(job).is_ok()
    }

    /// The side results arrive on, cloned for a `select!`.
    pub(crate) fn results(&self) -> Receiver<Done> {
        self.results.clone()
    }

    /// Files `done` beside its entry. A result of an earlier job for the same
    /// entry, which a refresh has moved the counter past, is dropped.
    pub(crate) fn receive(&mut self, done: Done) {
        if let Some((change, slot @ None)) = self.in_flight.get_mut(&done.entry)
            && *change == done.change
        {
            *slot = Some(done);
        }
    }

    /// The result of `entry`'s job, taken out of flight, once it arrived.
    pub(crate) fn take(&mut self, entry: &Path) -> Option<Done> {
        if !matches!(self.in_flight.get(entry), Some((_, Some(_)))) {
            return None;
        }
        self.in_flight.remove(entry).and_then(|(_, done)| done)
    }

    /// Whether `entry` was sent and its result has not arrived.
    pub(crate) fn is_running(&self, entry: &Path) -> bool {
        matches!(self.in_flight.get(entry), Some((_, None)))
    }

    /// Whether `entry` was sent and not filed yet.
    pub(crate) fn is_sent(&self, entry: &Path) -> bool {
        self.in_flight.contains_key(entry)
    }

    /// Takes every result that already arrived, without waiting.
    pub(crate) fn collect(&mut self) {
        while let Ok(done) = self.results.try_recv() {
            self.receive(done);
        }
    }

    /// Waits for one result and files it. Returns `false` when every job in
    /// flight has its result already, so nothing more could arrive.
    pub(crate) fn wait(&mut self) -> bool {
        if self.in_flight.values().all(|(_, done)| done.is_some()) {
            return false;
        }
        match self.results.recv() {
            Ok(done) => {
                self.receive(done);
                true
            }
            Err(_) => false,
        }
    }

    /// Stops every worker, and waits for each to end. A worker ends once the
    /// job channel closes and its own job, if it has one, is done. The results
    /// nobody will file are dropped with it.
    pub(crate) fn stop(&mut self) {
        self.jobs = None;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        while self.results.try_recv().is_ok() {}
        self.in_flight.clear();
        self.cursor = None;
        self.view = None;
    }

    /// How many worker threads run.
    pub(crate) fn running(&self) -> usize {
        self.threads.len()
    }
}

/// One worker: analyses each job it takes until the job channel closes.
fn work(jobs: &Receiver<Job>, done: &Sender<Done>) {
    let mut session = Session::default();
    let mut trees = TreeCache::default();
    while let Ok(job) = jobs.recv() {
        let at = Entry {
            path: &job.entry,
            key: &job.entry,
        };
        let indexed = |key: &std::path::Path| {
            job.indexed.contains(key) && !job.owned.iter().any(|own| own == key)
        };
        // A panic in one analysis is that entry's result, as an entry that
        // could not be loaded, and not a server that waits for ever. The
        // caches it may have left half written are started again.
        let analysis = catch_unwind(AssertUnwindSafe(|| {
            analyse_entry(&job.snapshot, &mut session, &mut trees, at, job.version, indexed)
                .map(|(_, analysis)| analysis)
        }))
        .unwrap_or_else(|_| {
            session = Session::default();
            trees = TreeCache::default();
            None
        });
        let sent = done.send(Done {
            entry: job.entry,
            change: job.change,
            analysis,
        });
        if sent.is_err() {
            return;
        }
    }
}
