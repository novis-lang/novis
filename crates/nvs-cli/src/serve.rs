//! `nvs serve`: one worker per core, every socket `[server] listen` names, and
//! every request running one entry file as
//! `rule:security/isolate-shares-nothing`'s isolate.
//!
//! [`nvs_server::serve::serve_on_this_core`] is the loop and
//! `rule:concurrency/one-future-per-connection`
//! is what drives a connection on it; what this module owns is what only the
//! binary can supply — the socket the loop accepts on, the clock it holds a
//! connection to, the handler that says which isolate a request is, and
//! [`Scheduled`], which says the same for a `[[schedule]]` entry that fires
//! beside it ([`nvs_server::schedule`], `rule:config/a-scheduled-run-is-a-root-isolate`). Every one is the same
//! split: this crate has the front end, so turning a path into a program is
//! here, and the loop and the ticker are there.
//!
//! # Decision: the configuration's mounts are the table, and a bare file is a table of one
//!
//! `rule:http-server/a-path-is-never-derived-from-a-url`
//! is the server's governing rule — a request *selects* an entry point from
//! a set enumerated before it arrived and may never construct one — and § 4's
//! mount table is that set. A tree that writes `[[server.mount]]` gets **the
//! whole of it**: [`nvs_config::mount::expand`] walked § 3's globs against the
//! disk, and this command binds a socket over the table that produced. A tree
//! that writes none is served through [`one_mount`], the file named on the
//! command line at `/` with its own directory as the mount root — not `expand`'s
//! implicit mount, which names a default entry nobody here was told to serve.
//!
//! **The argument does not override the table; it has to be in it.** A `<file>`
//! that is not one of the mounted entries is refused before the socket exists,
//! because a command told to serve a file and then serving a different
//! application is the one outcome neither reading wants. What the argument
//! always decides is *which* configuration this is: `rule:config/every-matching-app-block-applies-least-specific-first` layers the
//! `[[app]]` blocks that match it, and there is no second spelling for that.
//!
//! **Every mounted entry is compiled before the socket is bound**, which is what
//! § 2 buys — a program that does not compile is a start that fails rather than
//! a request that does — and every request then goes through
//! [`nvs_server::Table::select`]'s five steps. `[server] static` and `dispatch`
//! are read off the tree as written, which is
//! [`nvs_server::Table::from_config`]'s fail-closed reading of them, and a step 3
//! selection is answered by [`nvs_server::statics`]: one static policy, the same
//! one a proxied origin serves under.
//!
//! # Decision: every entry is bound, and the flag is the last word
//!
//! `[server] listen` is a flat array (§ 5) and
//! `rule:http-server/the-accept-fan-out-is-one-worker-per-core` binds **every**
//! entry of it, in the order written, before anything accepts: [`addresses`]
//! resolves the set and [`bind_all`] binds it, so a deployment answering on two
//! ports is two listening sockets rather than one and a note about the other.
//! [`handles_for`] then gives every worker its own handle on every one of them
//! and [`serve_on_worker`] is what a core does with them — one accept loop per
//! socket, on that core's own scheduler. `--listen` and `--port` override the
//! file, on § 5's own sentence; they conflict with each other, because two
//! spellings of one address is a question guessing an answer to would be worse
//! than refusing.
//!
//! A Unix-domain entry classifies (`nvs_config::server::Listen::Unix`) and is
//! then refused **once**, over the whole set and before a socket exists,
//! because [`nvs_host::NvsListener`] accepts on TCP alone today. A refusal
//! taken where a listener is bound would report one deployment mistake once per
//! core and leave the process half-listening while it did. That refusal moves
//! the day there is a listener for one; the classification does not.
//!
//! **What it spends**, per `rule:programs/memory-priority`:
//! one compiled unit per mounted entry, held for the life of the process and
//! shared by every request that runs it (`rule:security/isolate-shares-nothing`'s "shares immutable compiled code",
//! which is [`crate::script`]'s cache and nothing else), plus whatever the accept
//! loop holds per connection in flight. Nothing accumulates per request answered.
//! One more thread for the process, not one per core, and one watched entry per
//! *running* core: [`nvs_host::Watchdog`], which every worker registers itself
//! with once it holds a reactor and deregisters from when it ends. A stall
//! report costs a core nothing at all — it is read off the deadline each accept
//! loop already publishes. `rule:errors/on-limit`'s CPU ceiling costs it two
//! stores per request, one when a request's isolate starts and one when it ends
//! ([`nvs_host::Isolate::watched_by`]), and one word of the watched entry to
//! hold them in; nothing is written at a safepoint, and nothing accumulates per
//! request answered.
//!
//! # Known gap: a served request carries no configuration, so it has no ceiling
//!
//! The publication above is the whole mechanism and it stops nothing yet,
//! because what it publishes is the tree's ceiling and a served request's tree
//! has none: a connection's context is built by `nvs_server`'s accept loop and
//! is never handed the snapshot this command booted on, so `Ctx::cpu_limit` is
//! `0`, every `[limits]` key reads as absent and `nvs_runtime::capability`
//! denies every capability an entry asks for. `rule:config/the-config-is-an-immutable-snapshot`
//! is the rule that is not met — "a request clones the `Arc` when it starts and
//! reads from that clone for its whole life" — and closing it is what makes
//! every ceiling on this path live, not the publication.
//! — owner: M6
//!

use std::cell::Cell;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use jiff::Zoned;
use nvs_config::mount::Mounted;
use nvs_config::server::{Listen, capacity_for, listen_on, waits_for, workers_for};
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_host::{Isolate, NvsListener, Output};
use nvs_runtime::script::{Program, Resolver as _};
use nvs_runtime::{Ctx, Inbound, OutputSink, TaskRoot, Value};
use nvs_server::{
    Admission, Arrived, Ceiling, Cors, Incoming, OnDisk, Origin, Reply, Request, Resolved, Secure,
    Serving, Table, Trusted, What,
};

use crate::script::Compiler;

/// `nvs serve <file>` — resolve the tree, build § 4's mount table, compile every
/// entry in it, bind the socket and run the accept loop until this process is
/// stopped.
///
/// The order is the whole of § 2's rule: the configuration is refused before
/// anything is compiled, every mounted entry is compiled before anything is
/// bound, and the socket exists only once there is something for it to answer
/// with.
pub(crate) fn run(
    path: &Path,
    listen: Option<&str>,
    port: Option<u16>,
    config: &[PathBuf],
    init: crate::config::Init,
) -> ExitCode {
    // `rule:config/the-config-is-an-immutable-snapshot`'s snapshot, resolved exactly as `nvs run` resolves it and
    // for the same reason: a tree that does not resolve is a refusal to start.
    // The `[server]` block is `Boot`-class as a whole (`rule:http-server/the-server-block-is-boot-class`), so this
    // is the only time it is read.
    let mut sources = SourceMap::new();
    let (snapshot, origins) = match crate::config::boot_origins(config, path, &mut sources, init) {
        Ok(both) => both,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    if !snapshot.warnings.is_empty() {
        let mut diags = Diagnostics::new();
        for warning in &snapshot.warnings {
            diags.report(warning.clone());
        }
        crate::render_diagnostics(&mut diags, &sources);
    }
    // The outbound TLS client, built before a worker exists: it is the
    // process's one answer to whose certificates a handler believes
    // (`rule:security/one-tls-client`), and every core shares it.
    // `crate::config::install_tls_client` owns why the call is here.
    if let Err(diagnostic) = crate::config::install_tls_client(&snapshot) {
        return report(diagnostic, &sources);
    }

    // `rule:core-classes/temporary-dir-orphan-sweep`'s orphan sweep, the moment the root is knowable and long
    // before a socket exists. [`sweep_orphans`] owns why it is here, why no
    // other subcommand does it, and why nothing it finds can refuse this start.
    sweep_orphans(&snapshot.config);

    // Both halves of § 5 this loop can keep today. Neither can refuse here in
    // practice — `nvs_config::server::validate` is the boot pass and it ran
    // above — but a resolution reported twice is better than one swallowed,
    // and the origins map `boot_origins` kept is what points a refusal at the
    // line it came from.
    let waits = match waits_for(&snapshot.config, &origins) {
        Ok(waits) => waits,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    let configured = match listen_on(&snapshot.config, &origins) {
        Ok(entries) => entries,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    // `rule:http-server/admission-is-arithmetic-not-a-number`: the ceiling is the smaller of what the file asked for and
    // what this machine affords at the per-request cap, and a clamp is logged
    // **once, here**, naming both directives and both numbers. Silently is what
    // that section rejected — the observed capacity of a small instance changes
    // with the arithmetic, and an operator surprised by that should be able to
    // find out why from a line rather than from a benchmark.
    let capacity = match capacity_for(&snapshot.config, &origins) {
        Ok(capacity) => capacity,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    let ceiling = Ceiling::of(&capacity);
    if let Some(note) = ceiling.clamp_note() {
        eprintln!("note: {note}");
    }
    // `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s core count, read at boot
    // beside the valve because the key is `Boot`-class with the rest of the block. It is a bound
    // and not a request for one — a written count is neither raised to this machine's parallelism
    // nor clamped down to it — so the only thing it can refuse is the `0` that leaves nothing
    // accepting.
    let workers = match workers_for(&snapshot.config, &origins) {
        Ok(workers) => workers,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    // `rule:http-server/secure-headers-with-nothing-written`'s header set, resolved once beside the valve: with nothing
    // written under `[http.headers]` it is the whole of what every response this
    // server writes carries beside its body, and `nvs_server::secure` owns the
    // details § 1 calls decisions rather than transcription.
    // `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`: who may assert a client address or a scheme, resolved once
    // here beside the two above. An entry that names no network is dropped and
    // reported — `nvs_server::forwarded`'s module doc owns why dropping is the
    // fail-safe direction, and this is the boot that has somewhere to say so.
    let (trusted, unreadable) = Trusted::of(
        snapshot
            .config
            .server
            .as_ref()
            .and_then(|server| server.trusted_proxies.as_deref())
            .unwrap_or_default(),
    );
    for entry in unreadable {
        eprintln!(
            "note: [server] trusted_proxies entry {entry:?} names no address or network, and is ignored"
        );
    }
    // Read here because the resolved set is about to become the server's, and
    // § 6's boot `Warn` below needs the address this process actually binds —
    // which the flags have not had their say over yet.
    let nobody_trusted = trusted.is_empty();
    // `rule:http-server/cors-is-closed-until-origins-are-named`: closed until `[http.cors] origins` names somebody, which is
    // what a tree that wrote no `[http.cors]` resolves to — `nvs_server::cors`
    // owns what closed means and where the refusal is taken.
    let serving = Serving::new(
        Arc::new(Admission::new(&ceiling)),
        Arc::new(Secure::of(snapshot.config.http.as_ref())),
        Arc::new(trusted),
        Arc::new(Cors::of(snapshot.config.http.as_ref())),
        // `rule:config/the-config-is-an-immutable-snapshot`'s tree, handed to
        // the accept loop beside the policies this boot resolved out of it. It
        // crosses here rather than through [`Isolate`] because `nvs-host` names
        // no configuration crate at all, and this is the argument every
        // connection — and so every request — is already served under.
        Arc::clone(&snapshot),
    );
    let wanted = match addresses(&configured, listen, port) {
        Ok(wanted) => wanted,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    // `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s boot `Warn`: `production`, nothing bound but the loopback,
    // and nobody trusted. That is the shape of a proxied deployment that forgot
    // the directive — nothing off this machine can reach it except through a
    // proxy, and it is about to answer that proxy's address as every client's.
    //
    // A warning and not a refusal, because the same three facts also describe a
    // correct single-machine deployment that has no proxy at all, and this
    // server cannot tell those apart. It is asked of the addresses actually
    // bound and of all of them, rather than of `[server] listen`, so a set with
    // one entry reachable off this machine — `--listen 0.0.0.0:80`, or a second
    // written line — is not warned at, and it is asked only
    // of a tree that wrote a `[server]` block, because a directive can only be
    // forgotten out of a block somebody wrote. `nvs serve app.nvs` with no
    // configuration at all is § 1's *development* server and matches all three
    // facts on the way to matching nothing, and a line every such run prints is
    // a line every operator learns to skip.
    let started_in = snapshot
        .config
        .mode
        .as_ref()
        .and_then(|mode| mode.default.as_deref())
        .unwrap_or(nvs_config::mode::PRODUCTION);
    if nobody_trusted
        && snapshot.config.server.is_some()
        && started_in == nvs_config::mode::PRODUCTION
        && wanted.iter().all(|addr| addr.ip().is_loopback())
    {
        eprintln!(
            "warning: [server] trusted_proxies is empty and every address this server binds ({}) \
             is loopback, so no forwarded header is read and the proxy's own address is what \
             `Core\\Request::clientIp()` will answer; write the proxy's address or network there",
            wanted
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // § 4's table. A tree that writes `[[server.mount]]` is served through the
    // whole of it — `expand` has already walked § 3's globs against the disk —
    // and one that writes none is the file named on the command line, at `/`,
    // with the directory it sits in as the mount root. The module doc's
    // § *Decision* owns which of the two a run gets and why the argument still
    // has the last word over neither.
    let deployed = snapshot
        .config
        .server
        .as_ref()
        .is_some_and(|server| !server.mount.is_empty());
    let mounts = if deployed {
        match nvs_config::mount::expand(&snapshot.config, &origins, &crate::config::LocalFiles) {
            Ok(mounts) => mounts,
            Err(diagnostic) => return report(diagnostic, &sources),
        }
    } else {
        match one_mount(path) {
            Ok(mount) => vec![mount],
            Err(refusal) => {
                eprintln!("error: {refusal}");
                return ExitCode::FAILURE;
            }
        }
    };
    // A file this table cannot reach is a request nobody could make: the command
    // was told to serve it, so a table that does not mount it is a refusal
    // rather than a server quietly answering with somebody else's application.
    if deployed && !mounts.iter().any(|mount| mount.entry == snapshot.entry) {
        eprintln!(
            "error: `{}` is not one of the {} entries `[[server.mount]]` mounts; serve one of \
             those, or remove the blocks to serve this file alone",
            path.display(),
            mounts.len()
        );
        return ExitCode::FAILURE;
    }
    // This thread's table, for the enumeration below alone. Every worker builds
    // its own from the same mounts, because a `Table` is an `Rc` graph and
    // belongs to the thread that reads it.
    let table = Rc::new(Table::from_config(mounts.clone(), &snapshot.config));

    // § 2, kept before the socket exists: **every** path this server can execute
    // is compiled now, so a program that does not compile is a start that fails
    // rather than a request that does. That is the rule the enumeration exists
    // for, and it costs one compile per mounted entry at boot rather than one
    // per entry per request. The front end renders its own diagnostics
    // (`script`'s module doc), so the message here is the summary. What a later
    // request re-checks is `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s question and the same module doc's: this
    // compiler carries the `[opcache]` block, so an edited entry is recompiled
    // for the requests that resolve it after the edit.
    //
    // An `Arc` rather than an `Rc` because one cache serves the fleet: the unit
    // it publishes is `Send` and `Sync` (`nvs_codegen::Unit`), so the compile
    // paid for here is the only one any core makes, and a per-core cache would
    // have made the count above one per core. Built once, before anything is
    // bound, and handed to every accept loop by clone.
    let compiler = Arc::new(Compiler::new(&snapshot.config));
    for mounted in table.mounts() {
        if let Err(message) = compiler.resolve(&mounted.entry.to_string_lossy()) {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    }

    // Every address the set named, bound here and all of it before any worker
    // exists. That order is the rule's: a bind taken by the core that reached
    // the entry would report one wrong address once per core, and would leave
    // the process listening on whichever entries it got to first.
    let bound = match bind_all(&wanted) {
        Ok(bound) => bound,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    // One line per socket, and the path as it was written rather than the
    // canonical one the table holds: an operator reads these against the
    // command they typed. The address is the listener's own, so an entry
    // written with port `0` prints the port the platform chose; the address
    // asked for is the fallback for a platform that will not answer.
    for (listener, requested) in bound.iter().zip(&wanted) {
        let addr = listener.local_addr().unwrap_or(*requested);
        println!("listening on http://{addr} — {}", path.display());
    }

    // The fan-out itself: one worker per core, each taking its own handle on
    // every socket bound above, so a connection is accepted by whichever core
    // reaches it first rather than by one core that hands it on.
    let mut rows = match handles_for(&bound, workers) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("error: could not give every worker its own listener handle: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The workers' sockets now: this thread accepts on nothing and keeps no
    // descriptor it does not use.
    drop(bound);
    // Said once as the fleet starts, on the platforms that have something to
    // say: `nvs_host::cpuclock`'s docs own which those are, and why none of
    // them gets a wall-clock ceiling wearing the CPU one's name instead.
    if let Some(note) = nvs_host::cpuclock::no_ceiling_note() {
        eprintln!("{note}");
    }
    let cpus = nvs_host::cpus();
    if cpus.is_empty() {
        // A host that enumerates no CPU offers no `CpuId` to pin to, and that
        // is the one shape this fan-out cannot take. Every listener is then
        // accepted on by this thread, which is a server — rather than a start
        // that failed over a number the platform would not answer.
        eprintln!(
            "note: this host enumerates no CPU, so `[server] workers` cannot be honoured and this \
             thread accepts on every listener alone"
        );
        let mut sched = nvs_host::Scheduler::new();
        let core = Core {
            listeners: rows.swap_remove(0),
            compiler,
            mounts,
            snapshot,
            waits,
            serving,
            ticks: true,
            // Unpinned, and a `CpuId` is only ever handed out by `cpus()`, so
            // there is no core to report a stall against and nothing to watch.
            watched: None,
        };
        return if serve_on_worker(&mut sched, core) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    // One watchdog for the process rather than one per core, as
    // `rule:http-server/a-wedged-core-is-detected-by-its-deadline` states it. Held
    // in this frame so that it outlives every worker and its thread is joined
    // where the fleet is joined, rather than by whichever core happened to end
    // last; its own thread starts with the first registration a core makes.
    let watchdog = Arc::new(nvs_host::Watchdog::new());
    let mut running = Vec::with_capacity(rows.len());
    for (index, listeners) in rows.into_iter().enumerate() {
        // A count above this machine's parallelism is started rather than
        // clamped, so two workers can share a CPU: honouring the number the
        // operator wrote is what the key means, and the cores cycle.
        let cpu = cpus[index % cpus.len()];
        let core = Core {
            listeners,
            compiler: Arc::clone(&compiler),
            mounts: mounts.clone(),
            snapshot: Arc::clone(&snapshot),
            waits,
            serving: serving.clone(),
            // One roster and so one ticker, on the first worker: a schedule
            // armed per core would fire every entry once per core.
            ticks: index == 0,
            watched: Some((cpu, Arc::clone(&watchdog))),
        };
        match nvs_host::Worker::spawn(cpu, move |sched| serve_on_worker(sched, core)) {
            Ok(worker) => running.push(worker),
            Err(error) => {
                eprintln!(
                    "error: could not start a worker on CPU {}: {error}",
                    cpu.raw()
                );
                return ExitCode::FAILURE;
            }
        }
    }
    // Every worker's own answer, and this process's is all of them: a core that
    // stopped on its listener is a failed run however its neighbours ended.
    let mut served = true;
    for worker in running {
        match worker.join() {
            Ok(ended) => served &= ended,
            Err(_) => {
                eprintln!("error: a worker thread ended in a panic");
                served = false;
            }
        }
    }
    if served {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// What one worker is handed, and the whole of it: everything a core holds of
/// its own is built from this on the worker's own thread rather than moved onto
/// it, because the table, the handler and the scheduler's tasks are all `!Send`
/// by construction ([`nvs_host::Worker`]).
struct Core {
    /// This core's own handle on every socket the boot bound, one per
    /// `[server] listen` entry.
    listeners: Vec<std::net::TcpListener>,
    /// The fleet's one compiled-unit cache, so a source compiles once for the
    /// process rather than once per core.
    compiler: Arc<Compiler>,
    /// § 4's mounts, which every core turns into its own [`Table`].
    mounts: Vec<Mounted>,
    /// The tree this process booted on, read by every core and written by none.
    snapshot: Arc<nvs_config::Snapshot>,
    /// § 5's waits, copied because they are `Boot`-class and nothing reloads
    /// them under a connection.
    waits: nvs_config::server::Waits,
    /// The valve, the header set and the proxy list — one of each for the
    /// process, shared by clone rather than one per core.
    serving: Serving,
    /// Whether this worker arms `rule:config/a-scheduled-run-is-a-root-isolate`'s
    /// roster and ticks it. True on exactly one of them: a schedule armed per
    /// core would fire every entry once per core.
    ///
    /// A flag rather than the roster itself, because an `Armed` holds an `Rc`
    /// and so is one of the things a worker builds rather than is handed.
    ticks: bool,
    /// This core's place in `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s
    /// watched set: the CPU it is pinned to, and the process's one watchdog,
    /// which it registers with itself once it holds a reactor to read a
    /// deadline off. `None` on the host that enumerates no CPU, where nothing
    /// is pinned and so a stall has no core to be named against.
    watched: Option<(nvs_host::CpuId, Arc<nvs_host::Watchdog>)>,
}

/// One core's whole server: its own mount table over the fleet's compiler, one
/// accept loop per listener, the reactor those loops park on, and the schedule
/// ticker if this is the worker the roster went to.
///
/// Answers whether this core ended cleanly, which is what the boot's exit code
/// is made of — a core that stopped on its own listener is a failed run
/// whatever its neighbours did.
fn serve_on_worker(sched: &mut nvs_host::Scheduler, core: Core) -> bool {
    let Core {
        listeners,
        compiler,
        mounts,
        snapshot,
        waits,
        serving,
        ticks,
        watched,
    } = core;
    let stopped = Rc::new(Cell::new(false));
    // This core's own table over § 4's set, which is the same set on every
    // core: what is per-core is the structure, because an `Rc` graph belongs to
    // the thread that reads it.
    let table = Rc::new(Table::from_config(mounts, &snapshot.config));

    // Every request goes through § 4's five steps, and what they chose is either
    // a file to send — `nvs_server::statics`, the same policy a configured
    // deployment serves under — or a file to run as `rule:security/isolate-shares-nothing`'s isolate, the same
    // type `spawn script` runs and deliberately not a second isolation path
    // (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s crate doc). With one mount at `/` and `dispatch = "entry"`
    // that is step 5 every time and the resolve is a cache hit on the unit
    // compiled above, so what it costs per request is one `Program` over shared
    // code.
    // The one state the probe reports, and the accept loop below is what writes
    // it: this command never asks the loop to stop yet, so it reads `false` for
    // the whole of a run and § 5's `503` half arrives with the control socket
    // that can ask (`rule:config/no-network-control-surface`).
    // The *process's* bit, because this command is the process: an application
    // reads the same one through `Core\Server::isDraining()`, which has no
    // handle to have been given (`nvs_runtime::drain`).
    let draining = nvs_server::Draining::process();
    // The reactor is what a parked coroutine is woken by, and every connection
    // parks. It is installed ahead of the handler rather than beside the accept
    // loops that park on it because this is the line a `DeadlineView` comes
    // from, and the registration made from one is what the handler publishes a
    // request through. Nothing has started by here: every task below is spawned
    // onto a scheduler that first turns at `run_until_idle`, so the order in
    // this function decides what is in scope and never what is running.
    let reactor = match nvs_host::Reactor::new() {
        Ok(reactor) => reactor,
        Err(error) => {
            eprintln!("error: could not start the reactor: {error}");
            return false;
        }
    };
    let installed = nvs_host::reactor::install(reactor);
    // `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s registration,
    // made here because this is the line that produces the `DeadlineView` and
    // this is the thread whose turning it describes. On that half the watchdog
    // reads the view and this core writes nothing further for it, on any path:
    // what it watches is the deadline table every accept loop below already
    // keeps.
    //
    // An `Rc` because it is also the module's other half — the publisher a
    // served request is charged against `rule:errors/on-limit`'s CPU ceiling
    // through — and the handler hands a clone of it to every isolate it builds.
    // The clock those publications are charged from is taken inside `register`,
    // on this thread, which is the thread every one of them runs on.
    //
    // The handle deregisters on drop, so the watched set is the cores that are
    // running rather than the cores that were started — a finished core's
    // frozen deadline would otherwise read as a wedged one forever.
    let watched = watched
        .zip(nvs_host::reactor::with_current(|reactor| {
            reactor.deadline_view()
        }))
        .map(|((cpu, watchdog), view)| Rc::new(watchdog.register(cpu, view)));
    let handler = Rc::new({
        let compiler = Arc::clone(&compiler);
        let table = Rc::clone(&table);
        let draining = draining.clone();
        let watched = watched.clone();
        move |request: Request<Incoming>, origin: Origin| {
            // Ahead of the table, because a verb `Core\Http\Method` does not
            // carry names no application on this server rather than none at
            // this path: `Reply::not_implemented` owns why that is a `501` and
            // not the route table's `405`. Asked of `nvs_stdlib::request`,
            // which is the roster's one home — the door does not keep a list.
            if !nvs_stdlib::request::is_known_verb(request.method().as_str()) {
                return Reply::not_implemented();
            }
            let selected = match table.select(&request, &OnDisk) {
                // Step 0, ahead of every mount: § 5's probe says the process is
                // alive, which is a fact this loop holds and no program is asked
                // for. That is why it is answered here rather than by an entry —
                // an application that will not compile is exactly when the
                // question is being asked.
                Some(Resolved::Health) => return Reply::health(&draining),
                Some(Resolved::Mounted(selected)) => selected,
                // § 4 step 1's third arrow. There is no mount for it and so
                // nothing to run: not a program's `404` but the table's.
                None => return Reply::not_found(),
            };
            // § 4 step 2's remainder and the row it came off, both taken before
            // the branch below moves the rest of the selection. The remainder is
            // the path the application is written against; the row is borrowed,
            // so keeping it costs a word and is what `rule:routing/a-request-reads-its-mount` is answered
            // from further down.
            let mount = selected.mount;
            let stripped = selected.path;
            let file = match selected.what {
                What::Run(file) => file,
                // Step 3 chose a file to *send*, and sending it is one policy
                // this command shares with every other deployment rather than a
                // development reading of one (`nvs_server::statics`). Nothing is
                // run for it: it is already an answer.
                What::Static(file) => {
                    return nvs_server::statics::send(&file, request.headers(), &OnDisk);
                }
            };
            // Both halves of what the selected unit is: the code to run, and
            // `rule:routing/matched-once-before-the-handler`'s table to match against before it does.
            // `crate::script::Compiled` owns why the cache holds the second
            // one at all.
            let (program, routes): (Program, Option<Arc<nvs_runtime::routes::Routes>>) =
                match compiler.compiled(&file.to_string_lossy()) {
                    Ok((program, routes)) => (program, Some(routes)),
                    // Reachable because step 4 can name a file the boot compile
                    // never saw: under `dispatch = "path"` a `.nvs` under the mount
                    // root compiles on the request that first asks for it. It is a
                    // failing program rather than a panic because a handler answers
                    // with a reply and not with a `Result`: `rule:security/isolate-shares-nothing`'s failure is a
                    // value, and the accept loop turns one into this request's
                    // `500`. It matches against nothing: a table is a product of
                    // the compile that did not happen.
                    Err(message) => (
                        Box::new(move |ctx: &mut Ctx, _args| {
                            ctx.set_pending(message);
                            Value::null()
                        }),
                        None,
                    ),
                };
            // The carrier, built here because this is the last point at which
            // the arrived request and step 2's remainder are both in hand, and
            // handed to the isolate rather than to this loop's own context —
            // `Isolate::answering` owns that direction. It interprets nothing:
            // the verb is the token the peer wrote, the query is everything
            // after the `?` undecoded, and a header is one entry per field
            // line in arrival order. The names are `hyper`'s, so they are
            // lower-cased — a `HeaderName` is normalised on the way in and
            // there is no spelling left to preserve; `Core\Request::header`
            // compares case-insensitively regardless (RFC 9110 § 5.1).
            let mut inbound = Inbound::new(
                request.method().as_str(),
                &stripped,
                request.uri().query().unwrap_or(""),
            );
            for (name, value) in request.headers() {
                inbound.push_header(name.as_str(), value.as_bytes());
            }
            // And who it came from, which this handler is *told* rather than
            // reading: `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s walk ran on the connection, before the
            // ceiling and before this closure, because the answer decides
            // policy on responses no handler ever sees. Both of its answers
            // land here together — `Inbound::set_peer` owns why they are one
            // call — and `Core\Request::clientIp()` and `::scheme()` are what
            // read them back.
            inbound.set_peer(origin.client(), origin.scheme());
            // `rule:observability/an-inbound-traceparent-is-continued`'s trace, off the header lines just pushed: continued
            // where the peer sent a `traceparent` this understands, and a new
            // root where it did not. `nvs_server::trace` owns why the door
            // reads it and why a bad header is never a refusal; every request
            // has an id either way, because `Ctx::new` drew one before this
            // carrier existed.
            nvs_server::trace::take(&mut inbound);
            // `rule:routing/a-request-reads-its-mount`'s mount, which is the other half of what step 2 did:
            // the prefix taken off the path above, and § 3's captures of the row
            // that took it. `nvs_server::mount::carry` owns why the door writes
            // them rather than the program deriving them — a program that could
            // derive its own prefix would be reading the deployment out of the
            // request target, which is the one thing a mounted application is
            // written not to do.
            nvs_server::mount::carry(mount, &mut inbound);
            // `rule:routing/matched-once-before-the-handler`'s match, here because this is the first point at
            // which the request and the unit that will answer it are both in
            // hand, and the last one before application code exists to have
            // run. `nvs_server::route` owns why the door takes it rather than
            // the program, and why nothing is written for a request the table
            // does not claim.
            if let Some(routes) = &routes {
                nvs_server::route::take(routes, &mut inbound);
            }
            // Split only here: everything above reads the request whole, and
            // the body is the one part of it that does not go where the rest
            // does.
            let (head, incoming) = request.into_parts();
            // And the body, which crosses as `nvs_runtime::RequestBody` and not
            // as bytes — `rule:http-server/request-body-and-upload-total-are-two-caps`, and that trait's own docs are the
            // argument. It is split rather than handed over: `hyper`'s
            // `Incoming` is polled with the *connection's* context and the
            // isolate is a peer task, so what the carrier gets is the pulling
            // half and what goes back with the reply is the half the connection
            // keeps (`nvs_server::body`). A declared length already over § 5's
            // cap is refused here, before a program exists to be given it.
            let supply = match nvs_server::body::of(&head.headers, incoming) {
                Arrived::Absent => None,
                Arrived::TooLarge => return Reply::too_large(),
                Arrived::Streaming(supply, pull) => {
                    inbound.set_body(pull);
                    Some(supply)
                }
            };
            let isolate = Isolate::new(program, Value::null(), Output::Capture).answering(inbound);
            // `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s
            // other half, on the one type that can carry it to a context that
            // does not exist yet: what the request is charged through is its own
            // tree's safepoint handle and its own ceiling, and `Isolate::start`
            // is where both are first in hand. A core the boot could not name a
            // CPU for registers nothing and hands nothing over, which is the
            // same host on which nothing is pinned and a stall has no core to be
            // reported against.
            let isolate = match &watched {
                Some(watched) => isolate.watched_by(Rc::clone(watched)),
                None => isolate,
            };
            Reply::Run(isolate, supply)
        }
    });

    // The loop runs *as a task*, which is not a formality: every connection it
    // accepts is a child of it (`rule:concurrency/a-child-belongs-to-the-calling-task`), and `serve_on_this_core` refuses
    // to run anywhere else. Its own context writes nothing — a connection's
    // bytes are its request's isolate's, captured and handed back as data (ADR
    // 0088 § 3) — so `OutputSink::Sink` is what it holds rather than stdout.
    // `rule:config/a-scheduled-run-is-a-root-isolate`'s roster, armed on the one
    // worker that ticks it and armed here rather than at the boot, because an
    // `Armed` holds an `Rc` and cannot cross onto this thread. `nvs_server::arm`
    // is where the refusal for a `fleet` entry this host will not run lives, and
    // it is still named while an operator is reading the start; a tree with no
    // `[[schedule]]` arms nothing and spawns no ticker, which is why this costs
    // a walk of an empty vector and no task at all.
    //
    // `None` for `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s lease, and this binary is the one place that
    // answer can be given: `nvs-server` names no `nvs-stdlib`, so the store a
    // fleet entry would be held in is reachable from here and nowhere else.
    // What is missing is the operation rather than the store — `Core\Cache`'s
    // shared tier is `put` and `get` (`rule:concurrency/a-cached-value-is-copied-across-the-boundary`) and neither is a
    // set-if-absent — so there is nothing to implement `nvs_server::Leases`
    // with yet, and § 3's fallback holds: every fleet entry is left unarmed and
    // named. The moment that tier gains a compare-and-set, the implementation
    // is a few lines here and no change at all in the ticker.
    let mut armed = if ticks {
        nvs_server::arm(&snapshot.config.schedule, &Zoned::now(), None, |note| {
            eprintln!("note: {note}");
        })
    } else {
        Vec::new()
    };
    if !armed.is_empty() {
        println!(
            "arming {} scheduled entr{}",
            armed.len(),
            if armed.len() == 1 { "y" } else { "ies" }
        );
        // A second task on *this* scheduler and not a second scheduler: the
        // ticker sleeps out its interval on a core the accept loop is still
        // serving on, and each fire is a child task of it (`rule:concurrency/a-child-belongs-to-the-calling-task`).
        // `TaskRoot::Request` for the same reason the accept loop holds it — a
        // fault under a fire belongs to that run and must not retire the worker
        // the requests are being served by.
        let fires = Rc::new(Scheduled);
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            // `Zoned::now` and not a fixed instant: § 6's missed interval is
            // skipped rather than replayed, which is the ticker asking the clock
            // for every fire and never counting from the last one.
            let ticked =
                nvs_server::tick_on_this_core(&mut armed, &fires, None, Zoned::now, || {
                    ControlFlow::Continue(())
                });
            if let Err(error) = ticked {
                eprintln!("error: the schedule ticker stopped: {error}");
            }
        });
    }
    // `rule:concurrency/one-process-serves-requests-schedules-and-jobs`'s third
    // subsystem, armed where the ticker is and for the ticker's reasons.
    let queue_workers = arm_queue_workers(sched, &snapshot, ticks, &draining);
    if queue_workers > 0 {
        println!(
            "arming {queue_workers} queue worker{}",
            if queue_workers == 1 { "" } else { "s" }
        );
    }
    // One task per listener, because a parked accept loop answers one socket
    // and every entry of `[server] listen` is bound. They share this core, its
    // handler and the units behind it; the other cores are running this same
    // set of loops on their own handles on these same descriptors, and the
    // kernel's accept queue is what decides which of them takes a connection.
    for handle in listeners {
        let addr = handle.local_addr().ok();
        let mut listener = match NvsListener::from_std(handle) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("error: this core could not take its handle on a socket: {error}");
                return false;
            }
        };
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let stopped = Rc::clone(&stopped);
            let draining = draining.clone();
            let handler = Rc::clone(&handler);
            let serving = serving.clone();
            move |_ctx| {
                // `ControlFlow::Continue` forever: nothing yet asks this command to
                // stop. Both of `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s
                // deployments run until the process ends — the proxied production
                // origin as much as the laptop — so this is not a development
                // shortcut but the absence of a caller. The two that will ask are
                // each their own slice: `Core\Signal`'s handler, which enters this
                // same drain rather than a second state machine, and the control
                // socket `rule:config/the-config-is-an-immutable-snapshot` gives
                // this command. Until one lands, the tail below is unreachable,
                // `Draining::begin` is never called, and an instance ends by being
                // killed mid-request.
                let served = nvs_server::serve_on_this_core(
                    &mut listener,
                    &handler,
                    waits,
                    &serving,
                    &draining,
                    // The same place the boot's own notes go: this command is the
                    // logger the server crate deliberately is not.
                    |note| eprintln!("note: {note}"),
                    || ControlFlow::Continue(()),
                );
                if let Err(error) = served {
                    match addr {
                        Some(addr) => {
                            eprintln!("error: the accept loop on {addr} stopped: {error}");
                        }
                        None => eprintln!("error: an accept loop stopped: {error}"),
                    }
                    stopped.set(true);
                }
            }
        });
    }

    // **`run_until_idle` is not this loop by itself, and a server is the first
    // caller for which that matters.** It returns as soon as one blocking poll
    // wakes nothing — which is what a connection's own socket reports once the
    // task that owned it has finished — and its docs say the report states that
    // case so the caller can decide. This is the caller: the accept loop is a
    // parked task for as long as it is serving, so a report with anything still
    // parked is a turn to take rather than an end. The one that ends this
    // process is `parked == 0`, which needs the accept loop itself to have
    // returned.
    let ran = nvs_runtime::script::scoped(compiler.as_ref(), || {
        loop {
            match nvs_host::run_until_idle(sched) {
                Ok(report) if report.parked > 0 => {}
                Ok(_) => return Ok(()),
                Err(error) => return Err(error),
            }
        }
    });
    drop(installed);
    if let Err(error) = ran {
        eprintln!("error: the scheduler stopped: {error}");
        return false;
    }
    !stopped.get()
}

/// What this core arms of `rule:core-classes/queue-storage-is-a-table`'s `[queue]`: the bounds the
/// boot resolved and the `[db.<name>]` block whose tables those jobs live in, or nothing.
///
/// **`ticks` gates this for the reason it gates the roster above.** `workers` is a count per
/// *instance* and never per core (`rule:concurrency/who-runs-a-job-is-configuration`), so arming it
/// on every core would turn an operator's `workers = 4` into four times this machine's parallelism —
/// on the one key whose job is to say how many connections a deployment holds open against its
/// database. Today this command turns one scheduler on one core and the two coincide; this is where
/// that has to be honoured rather than discovered.
///
/// [`nvs_config::queue::queue_for`] is the resolution the boot already accepted, so a refusal is
/// impossible by the time this runs and `.ok()` swallows none; `run_run` reads it exactly this way,
/// which is what makes moving queue work between the two binaries operational and never
/// behavioural. `workers = 0` is § 2's enqueue-only deployment and arms nothing, which is also what
/// a tree writing no `[queue]` block at all costs.
fn queue_on_this_core(
    config: &nvs_config::Config,
    ticks: bool,
) -> Option<(nvs_config::queue::QueueBounds, nvs_config::tree::Database)> {
    if !ticks {
        return None;
    }
    nvs_config::queue::queue_for(config, &std::collections::BTreeMap::new())
        .ok()
        .flatten()
        .filter(|bounds| bounds.workers > 0)
        .and_then(|bounds| {
            let block = config.db.get(&bounds.connection)?.clone();
            Some((bounds, block))
        })
}

/// [`crate::worker::start`]'s tasks on this core's scheduler, and how many of them there are.
///
/// Called beside the ticker and **before the accept loop is spawned**, for the ticker's own reason:
/// a task queued here runs on the core the requests are served on, and both are queued before
/// anything is accepted so that neither waits out a connection to take its first turn.
///
/// Two things differ from the ticker three lines above. **The drain**, because a served instance has
/// no script whose exit could set [`crate::worker::Workers::stop`] — a worker reading the drain is
/// what lets this process end at all, since one that ignored it would be a task always parked and
/// so a server nothing but a kill could stop. **The root**, which is
/// [`crate::worker::start`]'s `TaskRoot::Worker` where the ticker holds `TaskRoot::Request`.
///
/// No lease and no `nvs_server::Leases`, unlike `arm`: `rule:concurrency/claiming-is-one-statement`
/// puts the mutual exclusion in the database, so a fleet of instances each running their own
/// workers is the intended deployment rather than the hazard a `fleet` schedule entry would be.
fn arm_queue_workers(
    sched: &mut nvs_host::Scheduler,
    snapshot: &Arc<nvs_config::Snapshot>,
    ticks: bool,
    draining: &nvs_server::Draining,
) -> u32 {
    let Some((bounds, block)) = queue_on_this_core(&snapshot.config, ticks) else {
        return 0;
    };
    crate::worker::start(
        sched,
        &crate::worker::Workers::draining(draining.clone()),
        &bounds,
        &block,
        snapshot,
    );
    bounds.workers
}

/// `rule:config/a-scheduled-run-is-a-root-isolate`'s fire, from the side only this binary can answer.
///
/// The ticker in `nvs-server` owns *when* a `[[schedule]]` entry runs and *where* —
/// a task of its own, with its own context, so that a run taking an hour is not
/// why the next minute's entry is late. What is left is what needs a compiler
/// and a logger: which isolate the entry's `script` is, and what its result
/// says. Both are this crate's, for the reason the module doc gives for the
/// handler.
///
/// Nothing is carried on it: the resolver is installed for the whole run
/// (`nvs_runtime::script::scoped` below), so a fire reaches the same compiler and
/// the same compiled-unit cache a request does, and a scheduled script that is
/// also a mounted entry is a cache hit rather than a second compile.
struct Scheduled;

impl nvs_server::Fires for Scheduled {
    fn isolate(&self, entry: &nvs_server::Armed, ctx: &mut Ctx) -> Option<Isolate> {
        // Resolved per fire and not once at boot, because `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s unit swap is
        // the point: an entry that fires nightly picks up an edited script at the
        // next fire, exactly as a request picks it up at the next request. What
        // the cache makes cheap is the *repeat*, not the first one.
        let program = match nvs_runtime::script::resolve(ctx, entry.script()) {
            Ok(program) => program,
            Err(refused) => {
                eprintln!(
                    "warning: the scheduled entry `{}` was not run: {refused}",
                    entry.name()
                );
                return None;
            }
        };
        // § 5: the script is handed its entry's `name` and reads it back through
        // `Core\Script::args()` — there is no scheduler-specific accessor.
        // Ownership: `NvsStr::new` makes the one reference `Isolate::new`
        // consumes, so nothing here releases anything.
        let args = Value::str(nvs_runtime::NvsStr::new(entry.name().as_bytes()));
        // `Output::Capture` because a scheduled run's `echo` is its own captured
        // output rather than this process's stdout (`rule:tooling/echo-always-has-a-sink`'s table), and
        // that capture is what the line below reports.
        Some(Isolate::new(program, args, Output::Capture))
    }

    fn ran(&self, entry: &nvs_server::Armed, done: &nvs_host::Completion) {
        if done.ok {
            // § 5: the result is logged and not delivered — nothing is waiting for
            // it. A returned `string` is rendered; anything else is reported as
            // having returned, because rendering an arbitrary value is
            // `Core\Debug`'s job and a scheduled run has no sink to hand it to.
            let returned = done.value.as_text().unwrap_or("");
            eprintln!(
                "note: the scheduled entry `{}` ran, {} byte(s) of output{}{returned}",
                entry.name(),
                done.output.len(),
                if returned.is_empty() {
                    ""
                } else {
                    ", returning "
                }
            );
            return;
        }
        // `error` is present exactly when `ok` is false — `nvs_host::Completion`'s
        // own doc — so the fallback is unreachable for a completion this host
        // produced, and is written rather than asserted because a schedule whose
        // failures are silent is the failure `rule:config/scheduled-work-is-a-config-block`'s boot refusals exist to
        // prevent.
        match &done.error {
            Some(failure) => eprintln!(
                "warning: the scheduled entry `{}` threw {}: {}",
                entry.name(),
                failure.class,
                failure.message
            ),
            None => eprintln!(
                "warning: the scheduled entry `{}` did not return and named no failure",
                entry.name()
            ),
        }
    }

    fn note(&self, note: &str) {
        eprintln!("note: {note}");
    }
}

/// `rule:core-classes/temporary-dir-orphan-sweep`'s orphan sweep — run at a server boot and by hand from
/// `nvs tmp clean`, and the reason a hard-killed script's leftovers ever go away.
///
/// **Boot, and no other invocation.** A `nvs run` does not sweep and neither
/// does any other subcommand: taxing every CLI start with a walk of the root to
/// insure against a rare hard kill prices the common case for the exceptional
/// one. The deliberate consequence, which § 4 states rather than regrets, is
/// that a machine where no server ever boots keeps a killed script's directory
/// until an operator runs `nvs tmp clean`.
///
/// **Before traffic, and as early as the root is knowable.** It sits with the
/// configuration rather than with the mounts because the owned root is the
/// host's and not the application's — nothing it deletes belongs to a program
/// this boot is about to compile, and a boot that goes on to fail has still done
/// no harm, since the predicate is liveness and a live owner's entry is never
/// touched ([`nvs_runtime::sweep::orphans`]).
///
/// **It cannot refuse the start.** Every failure here is a log line: a root that
/// cannot be listed is nothing to sweep, and a deletion the platform refuses —
/// on Windows, routinely a handle an indexer is holding — leaves the entry
/// standing for the next boot to try again. A server that would not start
/// because a stale directory could not be removed is the outage this sweep
/// exists to avoid, not one it may cause.
///
fn sweep_orphans(config: &nvs_config::Config) {
    let root = nvs_runtime::capability::temp_root(Some(config));
    for (path, error) in nvs_runtime::sweep::refusals(nvs_runtime::sweep::orphans(&root)) {
        eprintln!(
            "note: a leftover temporary directory could not be removed: {} ({error})",
            path.display()
        );
    }
}

/// `rule:http-server/a-request-resolves-in-five-steps`'s one row for a tree that mounts nothing: the file named on the
/// command line, mounted at `/`, with the directory it sits in as the mount
/// root.
///
/// Not [`nvs_config::mount::expand`]'s implicit mount, which names § 3's default
/// entry — a file this command was not told to serve and may not even have. § 2's
/// rule is about the set being enumerated before a request arrives rather than
/// about how many rows it has, and a set of one is still a set. The module doc's
/// § *Decision* is the whole of why.
///
/// Canonical on both sides: § 4 steps 3 and 4 compare a resolved remainder
/// against the mount root, and a `starts_with` between a canonical path and a
/// written one answers `false` for every file in the tree.
///
/// # Errors
///
/// A path that cannot be canonicalized — absent, or unreadable — and one that is
/// a filesystem root and so has no directory to be a mount root. Both are a
/// sentence rather than a [`nvs_diagnostics::Diagnostic`], for [`addresses`]'s
/// reason: the value came from a command line and there is no span to point into.
fn one_mount(path: &Path) -> Result<Mounted, String> {
    let entry = nvs_config::trust::canonical(path)
        .map_err(|why| format!("`{}` cannot be served: {why}", path.display()))?;
    let root = entry
        .parent()
        .ok_or_else(|| format!("`{}` is not a file in a directory", entry.display()))?
        .to_path_buf();
    Ok(Mounted {
        prefix: "/".to_string(),
        host: None,
        entry,
        root,
        origin: None,
        captures: Vec::new(),
    })
}

/// Every address this server binds: the file's entries in the order written,
/// then whichever flag had the last word.
///
/// `--listen` names one address and replaces the whole set, because that is
/// what overriding an array with a single spelling means. `--port` is the last
/// word for its own key alone and keeps each entry's host.
///
/// # Errors
///
/// A `--listen` that is not a literal address, and a Unix-domain entry there is
/// no listener for yet — the module doc owns both. **One refusal names every
/// entry it applies to**, because the set is read here, once, before any
/// listener is bound. The refusal is a sentence rather than a
/// [`nvs_diagnostics::Diagnostic`] when it came from a flag, because a command
/// line has no file and no span to point into.
fn addresses(
    configured: &[Listen],
    listen: Option<&str>,
    port: Option<u16>,
) -> Result<Vec<SocketAddr>, String> {
    if let Some(written) = listen {
        return written
            .parse::<SocketAddr>()
            .map(|addr| vec![addr])
            .map_err(|_| {
                format!(
                    "`--listen {written}` is not an address and a port; write `127.0.0.1:8000` or \
                     `[::1]:8000`"
                )
            });
    }
    let mut bound = Vec::with_capacity(configured.len());
    let mut unsupported = Vec::new();
    for entry in configured {
        let addr = match (entry, port) {
            (Listen::Tcp(addr), None) => *addr,
            // The flag is the last word for its own key alone: a port written
            // here keeps the host the file chose, so `--port` over a `0.0.0.0:80`
            // does not quietly narrow the deployment to loopback.
            (Listen::Tcp(addr), Some(port)) => SocketAddr::new(addr.ip(), port),
            // And over a Unix entry it names a whole address, since there is no
            // host in one to keep. `127.0.0.1` because that is § 5's own default
            // and the one a development machine means.
            (Listen::Unix(_), Some(port)) => {
                SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port))
            }
            (Listen::Unix(path), None) => {
                unsupported.push(format!("`{}`", path.display()));
                continue;
            }
        };
        // Two entries that resolve to one address are one socket: the platform
        // has no second one to give, and `--port` over a mixed set is how two
        // entries collapse into one. Port `0` is never a duplicate — it asks
        // for another free port, and the platform answers a different one.
        if addr.port() == 0 || !bound.contains(&addr) {
            bound.push(addr);
        }
    }
    if !unsupported.is_empty() {
        return Err(format!(
            "`[server] listen` asks for the Unix-domain socket{} {}, and this server accepts on \
             TCP alone today; write `--listen 127.0.0.1:8000` or a `host:port` entry",
            if unsupported.len() == 1 { "" } else { "s" },
            unsupported.join(", ")
        ));
    }
    // `listen_on` never answers with an empty list — an empty array is its own
    // refusal — so this is the file's set and not a server that accepts nothing.
    if bound.is_empty() {
        return Err("`[server] listen` named no address".to_owned());
    }
    Ok(bound)
}

/// Every address bound, in the order [`addresses`] gave them.
///
/// One call and one place, so a start that cannot have all of its sockets is a
/// start that fails: the listeners already bound are dropped on the way out of
/// the error, and nothing has accepted on any of them yet. They are `std`
/// listeners because a socket is bound before any core exists and each core
/// then takes its own handle on it ([`handles_for`],
/// [`nvs_host::NvsListener::from_std`]).
///
/// # Errors
///
/// The platform's, for the first address it refuses — already in use, or not
/// one of this host's — with that address named, because a refusal an operator
/// has to guess the subject of is one they read the configuration twice for.
fn bind_all(wanted: &[SocketAddr]) -> Result<Vec<std::net::TcpListener>, String> {
    wanted
        .iter()
        .map(|addr| {
            std::net::TcpListener::bind(addr)
                .map_err(|error| format!("could not listen on {addr}: {error}"))
        })
        .collect()
}

/// One row of handles per worker: every socket the boot bound, duplicated once
/// for each core that will accept on it.
///
/// A duplicate is the same socket — [`std::net::TcpListener::try_clone`] — so
/// the cores share one accept queue per listener and a connection goes to
/// whichever of them reaches it first, which is
/// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s first
/// sentence rather than a core handing work on. Taken here, before any worker
/// exists, so a platform that refuses is a start that fails rather than a core
/// quietly missing a listener.
///
/// # Errors
///
/// The platform's, for the first handle it will not duplicate.
fn handles_for(
    bound: &[std::net::TcpListener],
    workers: usize,
) -> std::io::Result<Vec<Vec<std::net::TcpListener>>> {
    (0..workers)
        .map(|_| {
            bound
                .iter()
                .map(std::net::TcpListener::try_clone)
                .collect::<std::io::Result<Vec<_>>>()
        })
        .collect()
}

/// One configuration refusal, rendered with the line it came from.
fn report(diagnostic: nvs_diagnostics::Diagnostic, sources: &SourceMap) -> ExitCode {
    let mut diags = Diagnostics::new();
    diags.report(diagnostic);
    crate::render_diagnostics(&mut diags, sources);
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::{
        Compiler, Ctx, Isolate, Listen, Output, OutputSink, SocketAddr, TaskRoot, Value, addresses,
        bind_all, handles_for, sweep_orphans, workers_for,
    };
    use std::collections::BTreeMap;
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn tcp(written: &str) -> Listen {
        Listen::Tcp(written.parse::<SocketAddr>().expect("a literal address"))
    }

    /// A tree whose `[queue]` names a SQLite file, which is a block that needs no
    /// server to be reachable — and is never opened here anyway, since a worker's
    /// handshake is its first act once it is *given a turn* and no case below
    /// runs one. A TOML literal string, because a Windows path is backslashes and
    /// a basic string would read them as escapes.
    fn queue_over_sqlite(workers: u32, name: &str) -> String {
        let path = std::env::temp_dir().join(format!("nvs-serve-{name}.db"));
        format!(
            "[db.jobs]\ndriver = 'sqlite'\npath = '{}'\n\n[queue]\nconnection = 'jobs'\nworkers = \
             {workers}\n",
            path.display()
        )
    }

    /// The typed tree one written block deserializes into — the boot reads a
    /// `Config` and this command's own resolution needs a file on disk, which a
    /// case about one key should not have to lay out.
    fn config_of(written: &str) -> nvs_config::Config {
        written
            .parse::<toml::Table>()
            .expect("the case writes valid TOML")
            .try_into()
            .expect("the case writes a block this tree has")
    }

    /// `rule:core-classes/temporary-dir-orphan-sweep`'s boot sweep, asserted from both directions in one case
    /// because the sweep has exactly one way to be wrong in each.
    ///
    /// The dead owner's entry going is the feature. The live owner's entry
    /// **and its contents** staying is the invariant — an age rule, a looser
    /// name parse or a liveness question answered the wrong way round all
    /// delete a running process's directory out from under it, and § 4 allows
    /// under-deleting and never the reverse. The operator's own directory is
    /// the third: this walk is over a root Novis owns, but it still does not
    /// touch a name it did not write.
    #[test]
    fn serve_boot_removes_a_dead_owners_entry_and_skips_a_live_one() {
        let root = std::env::temp_dir().join(format!("nvs-serve-sweep-{}", std::process::id()));
        // Above every platform's pid ceiling, so no process can be holding it
        // and the answer is not a race with anything this machine is running.
        let dead = root.join(format!("nvs-{}-0123456789abcdef", i32::MAX - 1));
        let live = root.join(format!("nvs-{}-0123456789abcdef", std::process::id()));
        let theirs = root.join("notes");
        for path in [&root, &dead, &live, &theirs] {
            std::fs::create_dir_all(path).expect("the platform root is writable");
        }
        let held = live.join("still-in-use");
        std::fs::write(&held, b"a live owner's file").expect("the case writes into its own entry");

        sweep_orphans(&config_of(&format!(
            "[io]\ntemp_root = '{}'\n",
            root.display()
        )));

        assert!(
            !dead.exists(),
            "the owner of {} is gone, so the boot reclaims it",
            dead.display()
        );
        assert!(
            held.is_file(),
            "this process is alive, so {} is untouchable — contents and all",
            live.display()
        );
        assert!(
            theirs.is_dir(),
            "a name this runtime never wrote is not the sweep's to delete: {}",
            theirs.display()
        );

        std::fs::remove_dir_all(&root).expect("the case removes what it made");
    }

    /// `rule:http-server/the-server-block-is-boot-class`'s own sentence, in
    /// both directions the flags have and the one refusal between them —
    /// unchanged by the fan-out, which is the half this case exists to hold.
    ///
    /// `--listen` replaces the configured set whole rather than merging into
    /// it, which is what overriding an array with one spelling means; it is
    /// asserted against a set that is nothing like it and longer than it, so a
    /// reading that kept the entries the flag did not mention would fail here.
    /// `--port` is the last word for its own key alone, so every entry keeps
    /// the host the file chose — the half a flag replacing whole addresses
    /// would get wrong, since a deployment listening on every interface still
    /// is one after it. And the two conflict at the command line, so there is
    /// never a set the two of them would have to be merged into.
    #[test]
    fn listen_and_port_flags_still_override_the_file_and_still_conflict() {
        use clap::Parser as _;

        let configured = vec![tcp("0.0.0.0:80"), tcp("127.0.0.1:8000")];
        assert_eq!(
            addresses(&configured, Some("127.0.0.1:9001"), None).expect("a literal address"),
            vec!["127.0.0.1:9001".parse::<SocketAddr>().expect("parses")]
        );
        assert!(addresses(&configured, Some("localhost:9001"), None).is_err());
        assert_eq!(
            addresses(&configured, None, Some(9001)).expect("an address per entry"),
            vec![
                "0.0.0.0:9001".parse::<SocketAddr>().expect("parses"),
                "127.0.0.1:9001".parse::<SocketAddr>().expect("parses"),
            ]
        );
        assert_eq!(
            addresses(&configured, None, None).expect("the file's own set"),
            vec![
                "0.0.0.0:80".parse::<SocketAddr>().expect("parses"),
                "127.0.0.1:8000".parse::<SocketAddr>().expect("parses"),
            ]
        );

        assert!(
            crate::Cli::try_parse_from([
                "nvs",
                "serve",
                "app.nvs",
                "--listen",
                "127.0.0.1:9001",
                "--port",
                "9002",
            ])
            .is_err(),
            "two spellings of one address are refused rather than merged"
        );
        assert!(
            crate::Cli::try_parse_from(["nvs", "serve", "app.nvs", "--listen", "127.0.0.1:9001"])
                .is_ok(),
            "`--listen` alone is the last word"
        );
        assert!(
            crate::Cli::try_parse_from(["nvs", "serve", "app.nvs", "--port", "9002"]).is_ok(),
            "`--port` alone is the last word"
        );
    }

    /// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s first
    /// sentence, from both of the ends it has: the set every entry resolves to,
    /// and the sockets that set leaves listening.
    ///
    /// The bind half is what a resolution answering three addresses and a boot
    /// binding the first of them would still pass without, so the case connects
    /// to each listener's own address — which no unbound port answers. Port `0`
    /// for each, so the platform picks three free ones and this case races
    /// nothing else on the machine running it.
    #[test]
    fn every_entry_of_server_listen_is_bound_rather_than_the_first() {
        let configured = vec![tcp("0.0.0.0:80"), tcp("127.0.0.1:8000"), tcp("[::1]:8100")];
        assert_eq!(
            addresses(&configured, None, None).expect("a literal set"),
            vec![
                "0.0.0.0:80".parse::<SocketAddr>().expect("parses"),
                "127.0.0.1:8000".parse::<SocketAddr>().expect("parses"),
                "[::1]:8100".parse::<SocketAddr>().expect("parses"),
            ]
        );

        let ephemeral = vec![tcp("127.0.0.1:0"), tcp("127.0.0.1:0"), tcp("127.0.0.1:0")];
        let wanted = addresses(&ephemeral, None, None).expect("three loopback entries");
        assert_eq!(wanted.len(), 3);
        let listeners = bind_all(&wanted).expect("three free ports on the loopback");
        let mut answered = Vec::new();
        for listener in &listeners {
            let addr = listener
                .local_addr()
                .expect("a bound socket knows its own address");
            std::net::TcpStream::connect(addr)
                .unwrap_or_else(|error| panic!("nothing is listening on {addr}: {error}"));
            answered.push(addr);
        }
        answered.sort();
        answered.dedup();
        assert_eq!(answered.len(), 3, "three entries are three sockets");
    }

    /// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s second
    /// half: the count is `[server] workers`, and every one of those workers
    /// holds its own handle on every socket the boot bound.
    ///
    /// Driven through `nvs_host::Worker` rather than asserted on the rows
    /// alone, because a handle that does not survive the move onto another
    /// thread is exactly the failure this shape exists to avoid: each worker
    /// answers with the addresses it can see from its own core, and they are
    /// the same sockets in the same order. The sockets outliving every worker
    /// is the other half — a row is a duplicate of the descriptor and not the
    /// only one, so a core that ends closes nothing for its neighbours.
    #[test]
    fn one_worker_is_spawned_per_core_and_each_takes_its_own_listener_handle() {
        let configured = vec![tcp("127.0.0.1:0"), tcp("127.0.0.1:0")];
        let bound = bind_all(&addresses(&configured, None, None).expect("two loopback entries"))
            .expect("two free ports on the loopback");
        let listening: Vec<SocketAddr> = bound
            .iter()
            .map(|listener| listener.local_addr().expect("a bound socket"))
            .collect();

        let workers = workers_for(&config_of("[server]\nworkers = 3\n"), &BTreeMap::new())
            .expect("a written count was refused");
        let rows = handles_for(&bound, workers).expect("a handle per socket per worker");
        assert_eq!(rows.len(), 3, "one row of handles per worker");

        let cpus = nvs_host::cpus();
        if cpus.is_empty() {
            // No `CpuId` to pin to is the one host `run` serves from this
            // thread instead, and there is no worker here to assert about.
            return;
        }
        let mut running = Vec::new();
        for (index, row) in rows.into_iter().enumerate() {
            let cpu = cpus[index % cpus.len()];
            running.push(
                nvs_host::Worker::spawn(cpu, move |_sched| {
                    row.iter()
                        .map(|listener| listener.local_addr().expect("this core's own handle"))
                        .collect::<Vec<_>>()
                })
                .expect("the platform started a worker"),
            );
        }
        for worker in running {
            assert_eq!(
                worker.join().expect("a worker ended in a panic"),
                listening,
                "every worker sees every socket, in the order they were bound"
            );
        }
        for addr in &listening {
            std::net::TcpStream::connect(addr)
                .unwrap_or_else(|error| panic!("{addr} stopped listening: {error}"));
        }
    }

    /// The Unix-domain refusal, taken over the whole set before any listener
    /// exists rather than by the core that reached the entry: two entries are
    /// one message naming both, and the flag still gets past it in the
    /// meantime, because a configuration a proxy should prefer must not simply
    /// fail to start with nothing to try.
    #[test]
    fn a_unix_domain_entry_is_refused_once_rather_than_once_per_core() {
        let configured = vec![
            tcp("127.0.0.1:8000"),
            Listen::Unix(PathBuf::from("/run/nvs.sock")),
            Listen::Unix(PathBuf::from("/run/nvs-admin.sock")),
        ];
        let refusal = addresses(&configured, None, None).expect_err("a socket was bound");
        assert!(
            refusal.contains("/run/nvs.sock") && refusal.contains("/run/nvs-admin.sock"),
            "one refusal names every entry it applies to: {refusal}"
        );
        assert_eq!(
            refusal.matches("accepts on TCP alone").count(),
            1,
            "one deployment mistake is one refusal: {refusal}"
        );
        assert_eq!(
            addresses(
                &[Listen::Unix(PathBuf::from("/run/nvs.sock"))],
                None,
                Some(8080)
            )
            .expect("the flag is the last word"),
            vec!["127.0.0.1:8080".parse::<SocketAddr>().expect("parses")]
        );
    }

    /// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s count, from every direction
    /// it has, because the key's whole content is which answer wins.
    ///
    /// The default is asserted against this machine's own parallelism rather than against a number
    /// the case picked, since a resolution that answered a constant — `1`, or the ADR's example —
    /// would pass against anything else. The written count is asserted **above** the machine's own
    /// as well as below it: a clamp against the box is exactly what the goal's § *Standing
    /// decisions* refuses, and only the high side can tell one from a bound that merely happens to
    /// agree. `0` is the one refusal, on `listen = []`'s reasoning reached from the other end.
    #[test]
    fn a_server_workers_key_bounds_the_count_and_defaults_to_available_parallelism() {
        let origins = BTreeMap::new();
        let machine = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
        for written in ["", "[server]\nlisten = [\"127.0.0.1:8000\"]\n"] {
            assert_eq!(
                workers_for(&config_of(written), &origins).expect("an unwritten count was refused"),
                machine,
                "for {written:?}"
            );
        }
        for count in [1, machine, machine + 7] {
            assert_eq!(
                workers_for(
                    &config_of(&format!("[server]\nworkers = {count}\n")),
                    &origins
                )
                .expect("a written count was refused"),
                count,
                "for {count}"
            );
        }
        let refused = workers_for(&config_of("[server]\nworkers = 0\n"), &origins)
            .expect_err("a server with no core to accept on was started");
        assert_eq!(refused.code, Some(nvs_diagnostics::code::E_NO_WORKERS));
    }

    /// How many requests one core serves in one arm of the measurement below.
    /// Enough that an arm lasts milliseconds rather than microseconds, since an
    /// arm shorter than the noise around it measures the noise, and no more than
    /// that, because the arm is paid for `ROUNDS` times on each side.
    const PER_CORE: usize = 400;

    /// The entry every request in that measurement runs, whose whole body is a
    /// bounded arithmetic loop: work a core does *itself*, with no syscall in it
    /// for the kernel to serialise the fleet on and nothing echoed for a sink to
    /// order. The loop's length is what puts one request far enough above the cost
    /// of spawning its task that an arm measures serving rather than scheduling.
    fn an_entry_that_costs_a_request() -> PathBuf {
        const SPINS: usize = 20_000;
        let dir = std::env::temp_dir().join(format!("nvs-serve-scale-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to write the entry in");
        let path = dir.join("entry.nvs");
        std::fs::write(
            &path,
            format!(
                "<?nvs\nint $total = 0;\nint $i = 0;\n\
                 while ($i < {SPINS}) {{ $total = $total + $i; $i = $i + 1; }}\n\
                 return $total;\n"
            ),
        )
        .expect("the entry is writable");
        path
    }

    /// One arm: a worker pinned to each of `cpus`, each serving [`PER_CORE`]
    /// requests off the one shared compiler, and how long the fleet took over the
    /// requests **alone** — every queue is spawned before the barrier and the
    /// clock starts after it, so [`nvs_host::Worker::spawn`] and the front end are
    /// both outside what is timed.
    ///
    /// A request here is what [`super::serve_on_worker`]'s handler does per
    /// request either side of `hyper`: the fleet's one `Arc<Compiler>` read for
    /// the unit, and that unit run as `rule:security/isolate-shares-nothing`'s
    /// isolate on this core's own scheduler.
    ///
    /// The ratio does not rest on the pinning, which is a best effort the OS may
    /// refuse ([`nvs_host::Worker::pinned`]): a worker is one thread and so is
    /// worth at most one core's throughput either way, which is what makes the
    /// one-core side of that ratio a floor rather than a hope.
    fn requests_on(cpus: &[nvs_host::CpuId], compiler: &Arc<Compiler>, path: &str) -> Duration {
        let answered = Arc::new(AtomicUsize::new(0));
        let ready = Arc::new(std::sync::Barrier::new(cpus.len() + 1));
        let mut running = Vec::with_capacity(cpus.len());
        for cpu in cpus {
            let compiler = Arc::clone(compiler);
            let answered = Arc::clone(&answered);
            let ready = Arc::clone(&ready);
            let path = path.to_owned();
            running.push(
                nvs_host::Worker::spawn(*cpu, move |sched| {
                    for _ in 0..PER_CORE {
                        let compiler = Arc::clone(&compiler);
                        let answered = Arc::clone(&answered);
                        let path = path.clone();
                        sched.spawn(
                            Ctx::new(OutputSink::Buffer(Vec::new())),
                            TaskRoot::Request,
                            move |ctx| {
                                let (program, _routes) =
                                    compiler.compiled(&path).expect("the entry compiles");
                                let done = Isolate::new(program, Value::null(), Output::Capture)
                                    .run(ctx)
                                    .expect("a null argument crosses");
                                if done.ok {
                                    answered.fetch_add(1, Ordering::Relaxed);
                                }
                            },
                        );
                    }
                    // Every worker's whole queue exists before any worker takes a
                    // turn of one, so the fleet serves together rather than one
                    // core finishing while the last is still spawning.
                    ready.wait();
                    sched.run();
                })
                .expect("the platform started a worker"),
            );
        }
        ready.wait();
        let started = Instant::now();
        for worker in running {
            worker.join().expect("a worker ended in a panic");
        }
        let took = started.elapsed();
        // Every request, answered by an isolate that ran to its end — which is
        // both halves of what an arm has to be true of before its duration means
        // anything, since a fleet that skipped its queue is the fastest of all.
        assert_eq!(
            answered.load(Ordering::Relaxed),
            cpus.len() * PER_CORE,
            "a request was not answered by an isolate that ran to its end"
        );
        took
    }

    /// A snapshot whose tree is the written one, which is what the boot hands
    /// every core.
    fn snapshot_of(written: &str) -> Arc<nvs_config::Snapshot> {
        Arc::new(nvs_config::Snapshot {
            config: config_of(written),
            ..Default::default()
        })
    }

    /// `[queue]` off the boot snapshot arms that many worker tasks, on the
    /// scheduler this command already turns.
    ///
    /// **What "before the accept loop is spawned" is read as here**: the arming
    /// returns with the scheduler holding those tasks and nothing else, which is
    /// the state the listener loop below it is then queued onto. Where the call
    /// sits in [`super::serve_on_worker`] is a line rather than a state, so it is
    /// not what this asserts — what it asserts is that arming needs no listener,
    /// no reactor and no request to have happened first.
    #[test]
    fn serve_arms_queue_workers_from_the_boot_snapshot_before_the_accept_loop_is_spawned() {
        let snapshot = snapshot_of(&queue_over_sqlite(2, "arms-from-the-snapshot"));
        let mut sched = nvs_host::Scheduler::new();
        let armed = super::arm_queue_workers(
            &mut sched,
            &snapshot,
            true,
            &nvs_server::Draining::detached(),
        );
        assert_eq!(
            armed, 2,
            "the boot snapshot's `[queue] workers = 2` armed {armed} worker(s)"
        );
        assert_eq!(
            sched.tracked_tasks(),
            2,
            "the scheduler the accept loops are spawned onto holds {} task(s) after the queue was \
             armed on it",
            sched.tracked_tasks()
        );
    }

    /// `workers` is a count per instance, so the core that ticks is the core that
    /// arms and every other core arms nothing.
    ///
    /// Both halves, because the count alone would pass on a build that armed it
    /// everywhere: a thirty-two-core host reading `workers = 4` per core is
    /// a hundred and twenty-eight connections against a database an operator
    /// sized for four (`rule:concurrency/who-runs-a-job-is-configuration`).
    #[test]
    fn workers_is_armed_once_per_instance_and_never_once_per_core() {
        let config = config_of(&queue_over_sqlite(4, "once-per-instance"));
        let (bounds, _) =
            super::queue_on_this_core(&config, true).expect("the core that ticks arms the queue");
        assert_eq!(
            bounds.workers, 4,
            "the ticking core armed {} worker(s) where the operator wrote four",
            bounds.workers
        );
        assert!(
            super::queue_on_this_core(&config, false).is_none(),
            "a core that does not tick armed a second set of workers, so `workers` is a count per \
             core rather than per instance"
        );
    }

    /// A tree with no `[queue]` block arms nothing and spawns no task, which is
    /// the ticker's shape: an `Option` read at boot and no cost beyond it.
    #[test]
    fn a_tree_with_no_queue_block_arms_no_worker_and_spawns_no_task() {
        let snapshot = snapshot_of("[server]\nlisten = ['127.0.0.1:8000']\n");
        assert!(
            super::queue_on_this_core(&snapshot.config, true).is_none(),
            "a tree writing no `[queue]` block resolved one anyway"
        );
        let mut sched = nvs_host::Scheduler::new();
        let armed = super::arm_queue_workers(
            &mut sched,
            &snapshot,
            true,
            &nvs_server::Draining::detached(),
        );
        assert_eq!(armed, 0, "{armed} worker(s) armed off a tree with no queue");
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "a tree with no `[queue]` block spawned {} task(s)",
            sched.tracked_tasks()
        );
    }

    /// `workers = 0` arms no worker and is not an error: it is the enqueue-only
    /// deployment `rule:core-classes/queue-storage-is-a-table` names, which is how
    /// an operator separates the machines that accept requests from the ones that
    /// drain the queue.
    ///
    /// The resolution is asserted to succeed as well as to arm nothing, because a
    /// refusal would also arm nothing — and would take the whole boot with it,
    /// over a tree that is spelled the way the rule spells it.
    #[test]
    fn workers_zero_arms_no_worker_and_is_not_an_error() {
        let config = config_of(&queue_over_sqlite(0, "enqueue-only"));
        let resolved = nvs_config::queue::queue_for(&config, &BTreeMap::new())
            .expect("`workers = 0` is a deployment and not a refusal");
        assert_eq!(
            resolved.map(|bounds| bounds.workers),
            Some(0),
            "the tree's `workers = 0` did not survive resolution"
        );
        assert!(
            super::queue_on_this_core(&config, true).is_none(),
            "an enqueue-only instance armed a worker"
        );
    }

    /// `rule:http-server/the-accept-fan-out-is-one-worker-per-core` as a number:
    /// the fan-out exists to serve more requests per second than one core can, and
    /// one that does not is a fan-out to delete rather than to keep and explain.
    ///
    /// **The margin is named here as [`SCALES_BY`] and the assertion is against
    /// it**, because "faster" with no floor under it passes on noise. Four times is
    /// the ideal and nothing reaches it: `nvs_host::cpus` enumerates *logical*
    /// CPUs, so the four cores this asks for are two physical ones on any machine
    /// that pairs them — and a second thread on a core already saturated with
    /// arithmetic adds a fraction of a core rather than one — while a box doing
    /// something else at the time lends less again. The floor is therefore not set
    /// near the ideal but where it stays true of the *worst* honest machine: four
    /// hyperthreads on two cores, under load, is what `1.5` leaves room for.
    ///
    /// What it has to separate that from is a fan-out that does not scale at all —
    /// a lock every request takes, a compile per core, one core accepting for the
    /// fleet — and every one of those lands at or under `1.0`, which is the gap the
    /// number sits in. It is a floor and not a target: the tree's own per-request
    /// cost is what decides how far above it a given run lands, and closing that
    /// distance is a perf question this test does not answer.
    ///
    /// **What the arms measure** is [`requests_on`]'s doc: every per-request cost
    /// above the socket. Not the accept and not the message parse, because driving
    /// those takes a client, and a loopback client fast enough not to be the
    /// bottleneck is a second fleet — the test would be measuring itself. That
    /// every core accepts on its own handle rather than through one is
    /// `one_worker_is_spawned_per_core_and_each_takes_its_own_listener_handle`'s
    /// claim, and this is the other half of it.
    ///
    /// **The arms are the best of `ROUNDS`, interleaved**, which is what makes this
    /// survive a box under load: noise only ever makes a run slower, so the
    /// shortest of several is the least contaminated estimate of each side, and
    /// interleaving keeps a slow patch of the machine from landing on one side of
    /// the ratio alone.
    #[test]
    fn serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names() {
        const CORES: usize = 4;
        const ROUNDS: usize = 3;
        /// Four cores serve at least half again the requests per second one
        /// serves. The paragraph above is why the floor is here and not at four.
        const SCALES_BY: f64 = 1.5;

        let cpus = nvs_host::cpus();
        if cpus.len() < CORES {
            // Fewer logical CPUs than the claim is about. There is nothing to
            // measure here rather than something to assert against a smaller
            // number: a ratio this machine cannot reach is not this fan-out's
            // failure, and `run` itself cycles the cores it was given.
            return;
        }
        let entry = an_entry_that_costs_a_request();
        let path = entry.to_string_lossy().into_owned();
        // The `[opcache]` defaults on purpose, which is the warm server this is
        // about: a compiler written to re-hash the file on every resolve would put
        // a `stat` in every request and measure the filesystem instead. The one
        // compile is warmed here, so no arm carries it and both sides read the same
        // published unit.
        let compiler = Arc::new(Compiler::new(&nvs_config::Config::default()));
        let (warm, _routes) = compiler.compiled(&path).expect("the entry compiles");
        drop(warm);

        let alone = [cpus[0]];
        let fleet = &cpus[..CORES];
        let mut one_core = Duration::MAX;
        let mut four_cores = Duration::MAX;
        for _ in 0..ROUNDS {
            one_core = one_core.min(requests_on(&alone, &compiler, &path));
            four_cores = four_cores.min(requests_on(fleet, &compiler, &path));
        }

        // Requests per second, four cores over one: each side served `PER_CORE` per
        // core, so the fleet's throughput is `CORES` times its own arm's rate.
        let scaled = CORES as f64 * one_core.as_secs_f64() / four_cores.as_secs_f64();
        assert!(
            scaled >= SCALES_BY,
            "four cores served {scaled:.2}× one core's requests per second, under the {SCALES_BY}× \
             this test names: {PER_CORE} requests on one core took {one_core:?}, and {} on four \
             took {four_cores:?}",
            CORES * PER_CORE
        );
    }
}
