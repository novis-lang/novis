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
//! with once it holds a reactor and deregisters from when it ends. It reads the
//! deadline each accept loop already publishes, so a core writes nothing for it
//! on any path.
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
) -> ExitCode {
    // `rule:config/the-config-is-an-immutable-snapshot`'s snapshot, resolved exactly as `nvs run` resolves it and
    // for the same reason: a tree that does not resolve is a refusal to start.
    // The `[server]` block is `Boot`-class as a whole (`rule:http-server/the-server-block-is-boot-class`), so this
    // is the only time it is read.
    let mut sources = SourceMap::new();
    let (snapshot, origins) = match crate::config::boot_origins(config, path, &mut sources) {
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
    let handler = Rc::new({
        let compiler = Arc::clone(&compiler);
        let table = Rc::clone(&table);
        let draining = draining.clone();
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
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                supply,
            )
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

    // The reactor is what a parked coroutine is woken by, and every connection
    // parks; the resolver is installed for the whole run so that a served
    // program's own `spawn script` reaches the same compiler and the same cache
    // this handler does.
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
    // this is the thread whose turning it describes. The watchdog reads that
    // view and this core writes nothing further for it, on any path: what it
    // watches is the deadline table every accept loop below already keeps.
    // The handle deregisters on drop, so the watched set is the cores that are
    // running rather than the cores that were started — a finished core's
    // frozen deadline would otherwise read as a wedged one forever.
    let _watched = watched
        .zip(nvs_host::reactor::with_current(|reactor| {
            reactor.deadline_view()
        }))
        .map(|((cpu, watchdog), view)| watchdog.register(cpu, view));
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
    use super::{Listen, SocketAddr, addresses, bind_all, handles_for, sweep_orphans, workers_for};
    use std::collections::BTreeMap;
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    fn tcp(written: &str) -> Listen {
        Listen::Tcp(written.parse::<SocketAddr>().expect("a literal address"))
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
}
