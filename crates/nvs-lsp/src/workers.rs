//! The threads that analyse the index's waiting entries while the server's
//! own thread answers the client.
//!
//! `rule:ide/the-server-is-synchronous` keeps the store, the index and every
//! answer on one thread. This module moves only the analysis of a waiting
//! entry off it. A worker takes a [`Job`], analyses the entry through the
//! job's [`Snapshot`] with a session and a configuration cache of its own, and
//! sends back what it found, which owns everything it holds. It never sends
//! the `Analysed`, which shares its interner through an `Rc`. The index files
//! each result on the server's thread in entry path order
//! ([`crate::SymbolIndex::advance`]), so the index is the one the serial loop
//! builds.
//!
//! # Decision: the workers split one graph, and never check a file twice
//!
//! A job is one of two [`Task`]s. A **plan** resolves an entry's graph, which
//! parses its files and reads their declarations and checks no body, and
//! returns the graph's files. A **part** analyses the entry and checks the
//! bodies of the files it names alone. The index plans the entry whose graph
//! it does not know, then splits the graph's files that no entry indexed and
//! no part in flight claims into one part per worker, in path order. An entry
//! is split only once every entry before it in path order was, so the first
//! entry in path order to reach a file is the one that checks it, as in the
//! serial loop. The claims are kept here, on the server's thread, so nothing
//! shared is locked.
//!
//! A part leaves out every file outside it. A left-out file this worker never
//! checked counts no anonymous functions, so the names of the anonymous
//! functions in a later file may differ from a whole check's. The index
//! records no anonymous function's name, so the records are the same.
//!
//! There are `available_parallelism() - 1` workers, and at least one. They are
//! started when the first job is sent and stopped once no entry waits, so a
//! settled server has no worker thread.
//!
//! # What it costs
//!
//! One session and one configuration cache per worker, for as long as
//! entries wait, freed when the queue empties. A worker keeps its session
//! between jobs, so the workers together hold about one copy of the checked
//! output of the files they checked. A worker reads the server's parses
//! (`crate::parse_cache`), so the parses are kept once, on the server's
//! thread. Each job copies the open buffers' text, and each part one set of
//! the paths it checks. A plan is one extra resolve
//! of a graph no entry indexed yet. That memory buys the latency of a cold
//! index, which the priority ordering allows.
//!
//! Measured with `bun nv lsp-start --copies 12 --cold --edits 5` on 16 cores,
//! so 15 workers, over 972 files: a cold index parses and checks each file
//! once and peaks at 786 MB paged, 463 MB private once it settles. When each
//! worker checked every body it read, it peaked at 5752 MB. A surface edit
//! checks each file once more and peaks at about 1020 MB paged.
//!
//! **Declined:** a cap on the number of workers. The memory grew with the
//! workers because each one checked the same bodies again. A cap hid that
//! cause and made the cold index slower on every machine.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use rustc_hash::FxHashSet;

use crate::document::{Entry, Snapshot, Work};
use crate::index::{Analysis, analyse_entry, plan_entry};
use crate::parse_cache::{Made, Shared};
use crate::session::Session;
use crate::tree_cache::TreeCache;

/// What a [`Job`] does with its entry.
pub(crate) enum Task {
    /// Resolve the entry's graph, check no body, and return the graph's files.
    Plan,
    /// Analyse the entry, check the bodies of these files alone, and build
    /// their records.
    Part(Arc<FxHashSet<PathBuf>>),
}

/// One entry for a worker to plan or analyse.
pub(crate) struct Job {
    /// The entry point, under `canonical_key`.
    pub(crate) entry: PathBuf,
    /// What the analysis reads from the store, taken when the job was sent.
    pub(crate) snapshot: Snapshot,
    /// The version of the entry's open buffer, or `NO_BUFFER`.
    pub(crate) version: i32,
    /// What the job does.
    pub(crate) task: Task,
    /// The part's place among the entry's parts. A plan is part `0`.
    pub(crate) part: usize,
    /// The index's change counter when the job was sent.
    pub(crate) change: u64,
    /// The server's pins and parses, which the worker reads before it parses.
    pub(crate) parses: Shared,
}

/// What a worker found for one [`Job`].
pub(crate) enum Found {
    /// The graph's files in the order it loaded them, and the work of
    /// resolving it. `None` when the entry could not be loaded.
    Plan(Option<(Vec<PathBuf>, Work)>),
    /// The part's analysis, or `None` when the entry could not be loaded.
    Part(Option<Analysis>),
}

/// What a worker sends back for one [`Job`].
pub(crate) struct Done {
    /// The job's entry.
    pub(crate) entry: PathBuf,
    /// The job's change counter.
    pub(crate) change: u64,
    /// The job's part.
    pub(crate) part: usize,
    /// What it found.
    pub(crate) found: Found,
    /// The parses it made, for the server to file.
    pub(crate) parses: Made,
}

/// A result is sent from the worker that built it to the server's thread.
const _: () = {
    const fn sendable<T: Send>() {}
    sendable::<Job>();
    sendable::<Done>();
};

/// One entry sent and not filed yet.
enum Flight {
    /// Its plan job runs.
    Planning {
        /// The change counter the job carried.
        change: u64,
    },
    /// Its parts run, or wait for its turn.
    Split {
        /// The change counter every part carried.
        change: u64,
        /// Each part's analysis once it arrived, in part order.
        parts: Vec<Option<Option<Analysis>>>,
        /// The files its parts check, held out of every later entry's parts.
        claims: Vec<PathBuf>,
    },
}

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
    /// Each entry sent and not filed yet.
    in_flight: BTreeMap<PathBuf, Flight>,
    /// Each file a split entry in flight checks.
    claimed: FxHashSet<PathBuf>,
    /// The graph each plan returned, for an entry not split yet.
    graphs: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// The work of the plans that arrived, for the index to count.
    work: Work,
    /// The parses the workers made, for the server to file.
    parses: Vec<Made>,
    /// The last entry split or passed ahead of its turn, so the next look for
    /// one starts after it.
    pub(crate) cursor: Option<PathBuf>,
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pool")
            .field("size", &self.size)
            .field("running", &self.threads.len())
            .field("in_flight", &self.in_flight.len())
            .field("claimed", &self.claimed.len())
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
            claimed: FxHashSet::default(),
            graphs: BTreeMap::new(),
            work: Work::default(),
            parses: Vec::new(),
            cursor: None,
        }
    }

    /// `available_parallelism() - 1` workers, and at least one: the server's
    /// own thread keeps a core for the client.
    pub(crate) fn sized_to_the_machine() -> Self {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        Self::new(cores.saturating_sub(1))
    }

    /// How many parts an entry is split into at most: one per worker.
    pub(crate) fn size(&self) -> usize {
        self.size
    }

    /// The jobs sent whose result has not arrived.
    fn running_jobs(&self) -> usize {
        self.in_flight
            .values()
            .map(|flight| match flight {
                Flight::Planning { .. } => 1,
                Flight::Split { parts, .. } => parts.iter().filter(|part| part.is_none()).count(),
            })
            .sum()
    }

    /// The jobs sent and not filed, arrived or not.
    fn unfiled_jobs(&self) -> usize {
        self.in_flight
            .values()
            .map(|flight| match flight {
                Flight::Planning { .. } => 1,
                Flight::Split { parts, .. } => parts.len(),
            })
            .sum()
    }

    /// Whether another entry can be sent now: a worker is free, and the
    /// results that wait for their turn stay under twice the pool's size.
    pub(crate) fn has_room(&self) -> bool {
        self.running_jobs() < self.size && self.unfiled_jobs() < 2 * self.size
    }

    /// Sends a plan job for its entry. Returns `false` when no worker runs and
    /// none could be started, and then nothing is in flight.
    pub(crate) fn send_plan(&mut self, job: Job) -> bool {
        let entry = job.entry.clone();
        self.in_flight
            .insert(entry.clone(), Flight::Planning { change: job.change });
        if self.dispatch(job) {
            return true;
        }
        self.in_flight.remove(&entry);
        false
    }

    /// Sends one entry's parts, which check `claims` between them. Returns
    /// `false` when no worker runs and none could be started, and then
    /// nothing is in flight.
    pub(crate) fn send_parts(&mut self, jobs: Vec<Job>, claims: Vec<PathBuf>) -> bool {
        let Some(first) = jobs.first() else {
            return false;
        };
        let entry = first.entry.clone();
        let change = first.change;
        self.graphs.remove(&entry);
        self.in_flight.insert(
            entry.clone(),
            Flight::Split {
                change,
                parts: jobs.iter().map(|_| None).collect(),
                claims: Vec::new(),
            },
        );
        for job in jobs {
            if !self.dispatch(job) {
                self.in_flight.remove(&entry);
                return false;
            }
        }
        self.claimed.extend(claims.iter().cloned());
        if let Some(Flight::Split { claims: kept, .. }) = self.in_flight.get_mut(&entry) {
            *kept = claims;
        }
        true
    }

    /// Puts `job` on the channel, and starts a worker for it when fewer than
    /// the pool's size run and fewer than the jobs that wait.
    fn dispatch(&mut self, job: Job) -> bool {
        let waiting = self.running_jobs();
        let (sender, receiver) = self.jobs.get_or_insert_with(crossbeam_channel::unbounded);
        if self.threads.len() < self.size && self.threads.len() < waiting {
            let receiver = receiver.clone();
            let done = self.done.clone();
            let started = std::thread::Builder::new()
                .name("nvs-lsp-index".to_owned())
                .spawn(move || work(&receiver, &done));
            // A thread the system would not start leaves the job in the
            // channel for the workers that run. With none, the entry is
            // analysed on the server's thread at its turn.
            match started {
                Ok(thread) => self.threads.push(thread),
                Err(_) if self.threads.is_empty() => return false,
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
    /// entry, which a refresh has moved the counter past, is dropped. A plan's
    /// graph is kept until the entry is split. The parses it made are kept
    /// for the server to file, whatever its counter, since a parse is right
    /// for its own text.
    pub(crate) fn receive(&mut self, done: Done) {
        let Done {
            entry,
            change: sent,
            part,
            found,
            parses,
        } = done;
        if !parses.is_empty() {
            self.parses.push(parses);
        }
        match (self.in_flight.get_mut(&entry), found) {
            (Some(Flight::Planning { change }), Found::Plan(graph)) if *change == sent => {
                self.in_flight.remove(&entry);
                // An entry that could not be loaded is planned as a graph of
                // its own file, so its turn analyses it and finds that out.
                let graph = graph.map_or_else(
                    || vec![entry.clone()],
                    |(graph, work)| {
                        self.work += work;
                        graph
                    },
                );
                self.graphs.insert(entry, graph);
            }
            (Some(Flight::Split { change, parts, .. }), Found::Part(analysis))
                if *change == sent =>
            {
                if let Some(slot @ None) = parts.get_mut(part) {
                    *slot = Some(analysis);
                }
            }
            _ => {}
        }
    }

    /// The parts of `entry`, in part order, taken out of flight once every
    /// one arrived. Its claims are released.
    pub(crate) fn take(&mut self, entry: &Path) -> Option<(u64, Vec<Option<Analysis>>)> {
        let Some(Flight::Split { parts, .. }) = self.in_flight.get(entry) else {
            return None;
        };
        if parts.iter().any(Option::is_none) {
            return None;
        }
        let Some(Flight::Split {
            change,
            parts,
            claims,
        }) = self.in_flight.remove(entry)
        else {
            return None;
        };
        for claim in &claims {
            self.claimed.remove(claim);
        }
        Some((change, parts.into_iter().flatten().collect()))
    }

    /// Whether `entry` was sent and a result of it has not arrived.
    pub(crate) fn is_running(&self, entry: &Path) -> bool {
        match self.in_flight.get(entry) {
            Some(Flight::Planning { .. }) => true,
            Some(Flight::Split { parts, .. }) => parts.iter().any(Option::is_none),
            None => false,
        }
    }

    /// Whether `entry`'s plan job runs.
    pub(crate) fn is_planning(&self, entry: &Path) -> bool {
        matches!(self.in_flight.get(entry), Some(Flight::Planning { .. }))
    }

    /// Whether `entry` was split and not filed yet.
    pub(crate) fn is_split(&self, entry: &Path) -> bool {
        matches!(self.in_flight.get(entry), Some(Flight::Split { .. }))
    }

    /// Whether a split entry in flight checks `file`.
    pub(crate) fn is_claimed(&self, file: &Path) -> bool {
        self.claimed.contains(file)
    }

    /// The graph a plan returned for `entry`, until it is split.
    pub(crate) fn graph(&self, entry: &Path) -> Option<&[PathBuf]> {
        self.graphs.get(entry).map(Vec::as_slice)
    }

    /// Forgets every entry `keep` does not keep: its flight, its claims and
    /// its plan. A result that arrives for it later is dropped.
    pub(crate) fn retain(&mut self, keep: impl Fn(&Path) -> bool) {
        let gone: Vec<PathBuf> = self
            .in_flight
            .keys()
            .filter(|entry| !keep(entry))
            .cloned()
            .collect();
        for entry in gone {
            if let Some(Flight::Split { claims, .. }) = self.in_flight.remove(&entry) {
                for claim in &claims {
                    self.claimed.remove(claim);
                }
            }
        }
        self.graphs.retain(|entry, _| keep(entry));
    }

    /// The work of the plans that arrived since the last call.
    pub(crate) fn take_work(&mut self) -> Work {
        std::mem::take(&mut self.work)
    }

    /// The parses the workers made since the last call.
    pub(crate) fn take_parses(&mut self) -> Vec<Made> {
        std::mem::take(&mut self.parses)
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
        if self.running_jobs() == 0 {
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
        self.claimed.clear();
        self.graphs.clear();
        self.parses.clear();
        self.cursor = None;
    }

    /// How many worker threads run.
    pub(crate) fn running(&self) -> usize {
        self.threads.len()
    }
}

/// One worker: runs each job it takes until the job channel closes. It keeps
/// its session between jobs.
fn work(jobs: &Receiver<Job>, done: &Sender<Done>) {
    let mut session = Session::default();
    let mut trees = TreeCache::default();
    while let Ok(job) = jobs.recv() {
        session.parses.adopt(job.parses);
        let at = Entry {
            path: &job.entry,
            key: &job.entry,
        };
        // A panic in one job is that entry's result, as an entry that could
        // not be loaded, and not a server that waits for ever. The caches it
        // may have left half written are started again.
        let found = catch_unwind(AssertUnwindSafe(|| match &job.task {
            Task::Plan => Found::Plan(plan_entry(
                &job.snapshot,
                &mut session,
                &mut trees,
                at,
                job.version,
            )),
            Task::Part(files) => Found::Part(
                analyse_entry(
                    &job.snapshot,
                    &mut session,
                    &mut trees,
                    at,
                    job.version,
                    |key| !files.contains(key),
                    files.contains(&job.entry),
                )
                .map(|(_, analysis)| analysis),
            ),
        }))
        .unwrap_or_else(|_| {
            session = Session::default();
            trees = TreeCache::default();
            match job.task {
                Task::Plan => Found::Plan(None),
                Task::Part(_) => Found::Part(None),
            }
        });
        let sent = done.send(Done {
            entry: job.entry,
            change: job.change,
            part: job.part,
            found,
            parses: session.parses.take_made(),
        });
        if sent.is_err() {
            return;
        }
    }
}
