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
//! A Unix-domain entry is bound where `rule:http-server/a-unix-socket-listener`
//! admits one, with `[server] socket_mode` on it, and on Windows — which has no
//! listener for one — the whole set is refused **once**, in [`addresses`],
//! before a socket exists. A refusal taken where a listener is bound would
//! report one deployment mistake once per core and leave the process
//! half-listening while it did. Past the bind the family is inside the value
//! ([`Socket`], `nvs_host::NvsConnection`) and no accept loop asks about it.
//!
//! **What it spends**, per `rule:programs/memory-priority`:
//! one compiled unit per mounted entry, held for the life of the process and
//! shared by every request that runs it (`rule:security/isolate-shares-nothing`'s "shares immutable compiled code",
//! which is [`crate::script`]'s cache and nothing else), plus whatever the accept
//! loop holds per connection in flight. Nothing accumulates per request answered.
//! One thread for the control endpoint where `[control] socket` names one, plus
//! one answer's bytes while it is answering — one at a time for the whole
//! process, since operations serialize there.
//! One more thread again for the process, not one per core, and one watched
//! entry per
//! *running* core: [`nvs_host::Watchdog`], which every worker registers itself
//! with once it holds a reactor and deregisters from when it ends. A stall
//! report costs a core nothing at all — it is read off the deadline each accept
//! loop already publishes. `rule:errors/on-limit`'s CPU ceiling costs it two
//! stores per request, one when a request's isolate starts and one when it ends
//! ([`nvs_host::Isolate::watched_by`]), and one word of the watched entry to
//! hold them in; nothing is written at a safepoint, and nothing accumulates per
//! request answered.
//!
//! # Decision: the snapshot this command publishes is what a served request reads
//!
//! What makes a ceiling live is the configuration reaching the request, not the
//! publication above. This command holds the tree in an
//! [`nvs_config::Current`] and hands `nvs_server`'s accept loop the holder, and
//! that loop writes a clone of what it holds onto the connection's own context
//! at the start of every request — `rule:config/the-config-is-an-immutable-snapshot`'s
//! one clone, taken when a request starts rather than when its peer dialled,
//! because one connection carries any number of requests and a tree read once
//! per socket would answer under whatever stood when that peer arrived.
//! `Ctx::set_config` refreshes the limits off the snapshot it is handed, so
//! `Ctx::cpu_limit` is the `[limits]` ceiling the tree wrote and
//! `nvs_runtime::capability` answers with the grants a mounted entry asks for.
//!
//! **The holder is what makes a reload a reload.** `rule:config/one-local-control-socket`'s
//! endpoint publishes into this one ([`crate::control`]), so an operator's
//! `nvs ctl reload` is serving from the next request rather than from the next
//! start; a request already running keeps the snapshot it took, and the boot's
//! own `Boot`-class reads above are never asked again. The `Core` each worker
//! is handed carries the boot snapshot beside the holder, for the per-core work
//! — the schedule roster and the queue's bounds — that is fixed at boot for
//! `rule:http-server/the-server-block-is-boot-class`'s reason.
//!
//! # Decision: the control endpoint is bound before any listener
//!
//! [`bind_sockets`] is the whole of the order and owns why. The short of it:
//! `rule:config/one-local-control-socket` holds the endpoint's directory to the
//! trust boundary every configuration file gets, and a start that had already
//! bound its listeners when it discovered the directory was writable by another
//! account would be a server answering requests on the way to refusing to
//! start.
//!

use std::cell::Cell;
use std::net::{SocketAddr, ToSocketAddrs as _};
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
use nvs_runtime::script::Program;
use nvs_runtime::{Ctx, Inbound, OutputSink, TaskRoot, Value};
use nvs_server::control::{Address, Endpoint};
use nvs_server::{
    Admission, Arrived, Ceiling, Cors, Incoming, OnDisk, Origin, Reply, Request, Resolved, Secure,
    Serving, Table, Trusted, What,
};

use crate::script::Compiler;
use crate::service::{Notify, State};

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
    // Held beside the accept loop's copy because the control endpoint's
    // `status` is this same count: the valve is where a request is admitted, so
    // asking it is reading the number rather than keeping a second one
    // (`crate::control`).
    let admission = Arc::new(Admission::new(&ceiling));
    // `rule:config/the-config-is-an-immutable-snapshot`'s tree, in the holder a
    // reload publishes into. The accept loop takes its clone out of this at the
    // start of every request, so a tree published on the control endpoint is
    // serving from the next request rather than from the next start; a request
    // already running keeps the one it took. It crosses to the loop rather than
    // through [`Isolate`] because `nvs-host` names no configuration crate at
    // all, and this is the argument every connection — and so every request —
    // is already served under.
    let current = Arc::new(nvs_config::Current::new(Arc::clone(&snapshot)));
    let serving = Serving::live(
        Arc::clone(&admission),
        Arc::new(Secure::of(snapshot.config.http.as_ref())),
        Arc::new(trusted),
        Arc::new(Cors::of(snapshot.config.http.as_ref())),
        Arc::clone(&current),
    );
    // `rule:config/one-local-control-socket`'s address, read here with the rest
    // of the `Boot`-class block so that a value naming something a network
    // could reach refuses the start before anything is compiled. A tree that
    // wrote no `[control]` block resolves to `Disabled`, which is no control
    // surface at all rather than a default one.
    let controlled = match nvs_server::control::Address::of(&snapshot.config) {
        Ok(address) => address,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    let wanted = match addresses(&configured, listen, port) {
        Ok(wanted) => wanted,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    // Read beside the set it applies to and before anything is bound, because
    // it is the whole of who may connect to a socket path
    // (`nvs_config::server::socket_mode_for`) and a value this boot could not
    // read is one it must not guess at. Resolved even where the set holds no
    // Unix entry, so a tree carrying the key is refused for a bad one on every
    // platform rather than on the ones that would have used it.
    let socket_mode = match nvs_config::server::socket_mode_for(&snapshot.config, &origins) {
        Ok(mode) => mode,
        Err(diagnostic) => return report(diagnostic, &sources),
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
    //
    // A set holding a Unix-domain entry does not match, and that is the rule
    // rather than an accident of the predicate: a connection over one *is*
    // trusted for the forwarded headers
    // (`rule:http-server/a-unix-socket-listener`), so such a deployment has
    // not forgotten the directive — it is using the transport the directive
    // exists to make unnecessary.
    let started_in = snapshot
        .config
        .mode
        .as_ref()
        .and_then(|mode| mode.default.as_deref())
        .unwrap_or(nvs_config::mode::PRODUCTION);
    if nobody_trusted
        && snapshot.config.server.is_some()
        && started_in == nvs_config::mode::PRODUCTION
        && wanted
            .iter()
            .all(|entry| matches!(entry, Listen::Tcp(addr) if addr.ip().is_loopback()))
    {
        eprintln!(
            "warning: [server] trusted_proxies is empty and every address this server binds ({}) \
             is loopback, so no forwarded header is read and the proxy's own address is what \
             `Core\\Request::clientIp()` will answer; write the proxy's address or network there",
            wanted.iter().map(written_as).collect::<Vec<_>>().join(", ")
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
    let mut mounts = if deployed {
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
    // `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s fallback,
    // folded into the rows once and here, so that every reader below — the boot
    // check and the door alike — has one field to read and cannot disagree
    // about which of the two keys applied.
    fall_back_to(&mut mounts, snapshot.origin.as_deref());
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
        if let Err(message) = compiled_under(&compiler, mounted) {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    }

    // Every socket this process opens, in the one order [`bind_sockets`] owns:
    // the control endpoint, then every address the set named — all of it before
    // any worker exists. That second half is the rule's: a bind taken by the
    // core that reached the entry would report one wrong address once per core,
    // and would leave the process listening on whichever entries it got to
    // first.
    let (controlling, bound) = match bind_sockets(
        &controlled,
        &wanted,
        socket_mode,
        nvs_server::control::boundary,
    ) {
        Ok(both) => both,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    // What stops this process, armed before anything is serving: from here a
    // `SIGTERM`, a Ctrl-C or a console closing is
    // `rule:concurrency/a-drain-closes-a-connection-cleanly`'s drain rather
    // than a kill, and every accept loop below reads the same bit. A start that
    // could not arm it is refused, because a server nothing can stop gracefully
    // is one an operator has to end mid-request (`crate::stop`).
    if let Err(error) = crate::stop::on_termination() {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    // Whatever started this process, taken from the environment it was started
    // in and kept as the process's: the boot reports `READY=1` below, the
    // reload its pair, and the terminating signal's thread — which is handed
    // nothing — reports `STOPPING=1` through `Notify::process`
    // (`crate::service`). A process nothing supervises reports to nobody, which
    // is what an operator running this by hand has.
    let told = Notify::from_env();
    told.install();
    // The endpoint's own thread, started before anything accepts so that the
    // first thing an operator can ask this process is answerable. It is a
    // thread and not a task on a core (`rule:concurrency/one-scheduler`), it
    // answers one client at a time, and it runs no Novis code; what it is
    // allowed to ask of this process is `crate::control::Process` and nothing
    // wider. Detached, because it has no ending of its own — the process's
    // drain is what stops it, and joining it here would be joining a thread
    // parked in an accept.
    if let Some(endpoint) = controlling {
        println!("control endpoint on {}", endpoint.name().display());
        let host = crate::control::Process::new(
            Arc::clone(&current),
            config.to_vec(),
            path.to_path_buf(),
            Arc::clone(&compiler),
            Arc::clone(&admission),
            nvs_server::Draining::process(),
            told.clone(),
        );
        if let Err(error) = std::thread::Builder::new()
            .name("nvs-control".to_owned())
            .spawn(move || drop(nvs_server::control::serve(&endpoint, &host)))
        {
            eprintln!("error: could not start the control endpoint's thread: {error}");
            return ExitCode::FAILURE;
        }
    }
    // `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`'s
    // scrape endpoint, bound here rather than by the core that ends up serving
    // it, for [`bind_sockets`]' own reason: an address the platform refuses is a
    // start that fails, not one core quietly missing a listener. A tree naming
    // no Prometheus exporter binds nothing at all, which is [`scrape_socket`]'s
    // `None` and the cheapest reading of `exporter = false`.
    let mut scrapes = match scrape_socket(&current.load().config) {
        Ok(socket) => socket,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(socket) = &scrapes {
        println!("scrapes answered on {}", socket.named());
    }
    listening(&bound, path, &told);

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
    // The process tier before any core that shares it exists, which is what
    // `rule:concurrency/the-process-tier-is-one-store-per-process` says about
    // when the map is created. Both fan-outs below are past this line, so
    // neither the pinned workers nor the single-threaded fallback is the one that
    // builds it.
    nvs_stdlib::arm_process_tier();
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
            scrapes: scrapes.take(),
            compiler,
            mounts,
            snapshot,
            current,
            waits,
            serving,
            ticks: true,
            // Unpinned, and a `CpuId` is only ever handed out by `cpus()`, so
            // there is no core to report a stall against and nothing to watch.
            watched: None,
        };
        // Nothing is watched here, so there is no stall detector to gate the
        // manager's ping on and it says only that the process is up — which is
        // all a host with no core to name a stall against can honestly say.
        // Withholding it instead would have the manager stop a server that is
        // serving (`crate::service::Heartbeat`).
        let _ping = crate::service::Heartbeat::start(&told, || true);
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
    // What answers the `WatchdogSec=` line `nvs service unit` renders, gated on
    // that same detector so the ping is evidence the fleet is turning rather
    // than evidence this thread is. Held in this frame like the watchdog it
    // reads: dropping it past the join stops the pings where the last core
    // stopped, and a process that is already reporting `STOPPING=1` owes its
    // manager no further beat (`crate::service::Heartbeat`).
    let _ping = {
        let watchdog = Arc::clone(&watchdog);
        crate::service::Heartbeat::start(&told, move || watchdog.turning())
    };
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
            current: Arc::clone(&current),
            waits,
            serving: serving.clone(),
            // One listener and so one core answering scrapes, on the same
            // worker the ticker went to. Every core's series still reach it —
            // `nvs_server::metrics::every_core` is what a scrape gathers
            // through, and the listener's core is not the only one it reads.
            scrapes: if index == 0 { scrapes.take() } else { None },
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
    listeners: Vec<Socket>,
    /// `[metrics] listen`, on exactly one worker: a scrape gathers every core's
    /// series through `nvs_server::metrics::every_core`, so a second listener
    /// would be a second address answering the same numbers.
    ///
    /// The socket itself and not a handle on it, because there is nothing to
    /// share it with. `None` on every other core, and on every core of a
    /// process whose `[metrics]` names no Prometheus exporter.
    scrapes: Option<Socket>,
    /// The fleet's one compiled-unit cache, so a source compiles once for the
    /// process rather than once per core.
    compiler: Arc<Compiler>,
    /// § 4's mounts, which every core turns into its own [`Table`].
    mounts: Vec<Mounted>,
    /// The tree this process booted on, read by every core and written by none.
    snapshot: Arc<nvs_config::Snapshot>,
    /// `rule:config/the-config-is-an-immutable-snapshot`'s holder, which is where
    /// the ticker on this core reads a fire's configuration out of: [`Scheduled`]
    /// takes its clone at each fire exactly as the accept loop takes one at each
    /// request. Beside the boot snapshot above rather than instead of it — what
    /// this core *builds* is `Boot`-class and is the tree this process started
    /// on, and what a fire *runs under* is whatever a reload has published since.
    current: Arc<nvs_config::Current>,
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
        scrapes,
        compiler,
        mounts,
        snapshot,
        current,
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
    // The one state the probe reports, and what ends this worker: the accept
    // loop below reads it on every pass, so a stop asked for on any thread of
    // this process stops every core.
    //
    // The *process's* bit, because this command is the process: a terminating
    // signal writes it (`crate::stop`), an application reads the same one
    // through `Core\Server::isDraining()`, which has no handle to have been
    // given (`nvs_runtime::drain`), and § 5's probe answers `503` off it.
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
        // The holder, not a rate read out of it here: `[trace] sample` reloads
        // (`rule:config/reloadability-is-its-own-field`), so a fraction resolved
        // while this closure was being built would be the one an operator can no
        // longer change. What a request draws against is read below, per request,
        // exactly as [`Scheduled`] reads a fire's tree per fire.
        let current = Arc::clone(&current);
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
            // carrier existed. The rate is `rule:observability/sampling-is-head-based`'s,
            // off the tree standing right now and not the one this core booted
            // on, and it decides only a trace this request roots.
            nvs_server::trace::take(
                &mut inbound,
                nvs_config::export::head_sample(&current.load().config),
            );
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
            // And the other half of the row step 1 chose: the origin this
            // mount resolved, which is what the program's absolute links are
            // built from. [`at_mount_origin`] owns why it goes onto the
            // isolate and not onto the carrier beside the prefix above.
            let isolate = at_mount_origin(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                mount,
            );
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
    // `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s lease, and
    // this binary is the one place it can be opened: `nvs-server` names no
    // `nvs-stdlib`, so the store a fleet entry is held in is reachable from here
    // and nowhere else. A tree with no `scope = "fleet"` entry opens nothing —
    // the connection is per process and lives as long as the server, so it is
    // paid for by the roster that needs it and by nothing else — and a store
    // that will not answer leaves § 3's fallback holding: every fleet entry
    // unarmed and named while an operator is still reading the start.
    let fleet = ticks
        && snapshot
            .config
            .schedule
            .iter()
            .any(|entry| entry.scope.as_deref().map(str::trim) == Some("fleet"));
    // An `Rc` rather than the value: § 3's renewal runs on each fire's own task, so the store is
    // reached from frames that outlive both this one and the tick's.
    let lease: Option<Rc<dyn nvs_server::Leases>> = if fleet {
        fleet_lease(&snapshot.config).map(|held| Rc::new(held) as Rc<dyn nvs_server::Leases>)
    } else {
        None
    };
    let mut armed = if ticks {
        nvs_server::arm(
            &snapshot.config.schedule,
            &Zoned::now(),
            lease.as_deref(),
            |note| {
                eprintln!("note: {note}");
            },
        )
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
        let fires = Rc::new(Scheduled { current });
        let ticking = draining.clone();
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            // `Zoned::now` and not a fixed instant: § 6's missed interval is
            // skipped rather than replayed, which is the ticker asking the clock
            // for every fire and never counting from the last one.
            let ticked = nvs_server::tick_on_this_core(
                &mut armed,
                &fires,
                lease.as_ref(),
                Zoned::now,
                || {
                    // The same drain the accept loop reads, so a stop ends the
                    // roster too rather than leaving this core turning for a
                    // ticker nobody can reach. An interval already being waited
                    // out is waited out first: `sleep` answers the instant its
                    // caller asked for and a wake does not cut it short, so the
                    // bound on a stop here is one entry's interval.
                    if ticking.is_draining() {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                },
            );
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
        let named = handle.named();
        let mut listener = match handle.accepting() {
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
                // `ControlFlow::Continue` forever, because this command has no
                // count of connections to serve and no ending of its own: both
                // of `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s
                // deployments run until the process is asked to stop, and what
                // asks is the drain the loop reads for itself — a terminating
                // signal today (`crate::stop`), the control socket's own stop
                // when it lands. The seam stays a seam for the caller that
                // ends a loop on its own terms, which this one is not.
                let served = nvs_server::serve_on_this_core(
                    &mut *listener,
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
                    eprintln!("error: the accept loop on {named} stopped: {error}");
                    stopped.set(true);
                }
            }
        });
    }

    // The exporter's own accept loop, on the one core the boot handed the
    // listener to. It is a task beside the accept loops above and not a thread
    // of its own (`rule:concurrency/one-scheduler`), it answers one collector at
    // a time, and it runs no Novis code; what it reads is every core's series
    // and never this one's alone.
    // A build without the feature reaches here with nothing: `scrape_socket`
    // refused the tree that would have bound a socket, so the `None` this arm is
    // left with is the only value it can hold.
    #[cfg(not(feature = "exporter"))]
    let _ = scrapes;
    #[cfg(feature = "exporter")]
    if let Some(handle) = scrapes {
        let named = handle.named();
        let mut listener = match handle.accepting() {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("error: this core could not take the scrape socket: {error}");
                return false;
            }
        };
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let stopped = Rc::clone(&stopped);
            let draining = draining.clone();
            move |_ctx| {
                let served = nvs_server::serve_scrapes_on_this_core(
                    &mut *listener,
                    waits,
                    &draining,
                    |note| eprintln!("note: {note}"),
                    || ControlFlow::Continue(()),
                );
                if let Err(error) = served {
                    eprintln!("error: the scrape loop on {named} stopped: {error}");
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

/// `nvs_server::Leases` over the shared tier — § 3's lease, which this binary is
/// the only crate that can supply and `nvs_server::schedule`'s module doc §
/// *Where a `fleet` entry's lease comes from* is the whole reason for.
///
/// It holds a token because a lease is held by **this process**: a renewal is
/// checked against what took the key, so a host that restarted never extends the
/// lease its predecessor was holding, and a second host never extends one it
/// merely found.
struct FleetLease {
    /// One connection, borrowed for the length of one command.
    ///
    /// A `RefCell` because the ticker asks through a `&self` and a command
    /// writes to a socket. The borrow can be held across a park — the exchange
    /// hands the core back — so a second asker finds it taken rather than
    /// waiting, and `try_borrow_mut` is what makes that a refusal instead of a
    /// panic. There is one other asker: the renewal a fire runs on its own task.
    store: std::cell::RefCell<nvs_stdlib::Lease>,
    /// What this process writes under a lease's key, and what its own renewals
    /// are checked against. The pid alone is not enough — two hosts have the
    /// same pids — so the instant this server started is in it.
    token: Vec<u8>,
}

impl nvs_server::Leases for FleetLease {
    fn take(&self, key: &str, ttl: std::time::Duration) -> bool {
        self.asked(key, "taken", |store, token| store.take(key, token, ttl))
    }

    fn renew(&self, key: &str, ttl: std::time::Duration) -> bool {
        self.asked(key, "renewed", |store, token| store.renew(key, token, ttl))
    }
}

impl FleetLease {
    /// One question to the store, with every way of not getting an answer
    /// collapsed into `false`.
    ///
    /// § 3 says a partition may leave an interval unrun, so not reaching the
    /// store is "this host does not hold the lease" and never a guess in the
    /// other direction. Each way of not reaching it is still a line an operator
    /// sees, because a schedule whose failures are silent is what
    /// `rule:config/scheduled-work-is-a-config-block`'s boot refusals exist to
    /// prevent.
    fn asked(
        &self,
        key: &str,
        verb: &str,
        ask: impl FnOnce(&mut nvs_stdlib::Lease, &[u8]) -> Result<bool, String>,
    ) -> bool {
        let Ok(mut store) = self.store.try_borrow_mut() else {
            // The connection is mid-command on another task. Waiting for it
            // would be the ticker blocking on a fire, which is the one thing a
            // scheduler must not do (`rule:concurrency/one-scheduler`).
            eprintln!(
                "note: the lease for `{key}` was not {verb}: this host's one lease connection is \
                 already answering another fire"
            );
            return false;
        };
        match ask(&mut store, &self.token) {
            Ok(answer) => answer,
            Err(why) => {
                eprintln!("warning: the lease for `{key}` was not {verb}: {why}");
                false
            }
        }
    }
}

/// The lease `arm` and the ticker are handed, or [`None`] when there is no
/// shared store configured or the one there is will not answer.
///
/// `[cache.shared]`'s two keys as the operator wrote them: `nvs_stdlib::Lease`
/// owns how a URL and a timeout are read, because it is the same pair
/// `Core\Cache::shared()` reads and a second reading of them here would be a
/// second dialect of one block.
fn fleet_lease(config: &nvs_config::Config) -> Option<FleetLease> {
    let shared = config.cache.as_ref()?.shared.as_ref()?;
    let url = shared.url.as_deref()?;
    match nvs_stdlib::Lease::open(url, shared.timeout.as_deref()) {
        Ok(store) => Some(FleetLease {
            store: std::cell::RefCell::new(store),
            token: format!("{}:{}", std::process::id(), Zoned::now()).into_bytes(),
        }),
        Err(why) => {
            eprintln!(
                "note: no `scope = \"fleet\"` entry is armed on this host, because its lease \
                 cannot be held: {why}"
            );
            None
        }
    }
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
/// The configuration is the one thing carried on it. A fire's context is built
/// by the ticker, in a crate that names no configuration at all, so this is the
/// only side that can put a tree on it — and `rule:config/a-scheduled-run-is-a-root-isolate`
/// wants the deployment's: `[limits]` for the run's budget and `[capabilities]`
/// for its grants, which is also what the resolve below is asked under.
///
/// The resolver is not carried. It is installed for the whole run
/// (`nvs_runtime::script::scoped` below), so a fire reaches the same compiler and
/// the same compiled-unit cache a request does, and a scheduled script that is
/// also a mounted entry is a cache hit rather than a second compile.
///
/// # Known gaps
///
/// The roster this serves is armed once, off the boot tree, so a reload that
/// adds, removes or re-times a `[[schedule]]` entry reaches the *configuration*
/// a fire runs under and not the set of entries that fire. Every `[[schedule]]`
/// key is `System`-class (`rule:config/three-changeability-classes`), so the
/// reload applies it and nothing re-arms — the operator is told a change landed
/// that this ticker will not honour until a restart.
struct Scheduled {
    /// The tree a fire is run under, taken from the holder per fire and never
    /// held across one.
    current: Arc<nvs_config::Current>,
}

impl nvs_server::Fires for Scheduled {
    fn isolate(&self, entry: &nvs_server::Armed, ctx: &mut Ctx) -> Option<Isolate> {
        // The fire's context arrives holding no tree, and this is the line that
        // gives it one. Read out of the holder per fire for the reason the accept
        // loop reads it per request (`rule:config/the-config-is-an-immutable-snapshot`):
        // an entry firing nightly runs under what a reload published, and a fire
        // already in flight keeps the clone it took. It is before the resolve
        // because `script.spawn` is the first thing that resolve asks for, and a
        // context holding no configuration grants nothing.
        ctx.set_config(self.current.load());
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

/// `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s fallback: a row
/// that wrote no `origin` of its own takes `[app] origin`, and a row that wrote
/// one keeps it.
///
/// **Folded into the rows rather than read beside them**, because a fallback
/// consulted at each reader is a fallback two readers can apply differently —
/// and the two here are a boot refusal and a served link, which is the pair
/// that must not disagree. From here down, `Mounted::origin` is *the* origin
/// this mount resolved, whichever key wrote it.
///
/// The `[app]` half is the boot snapshot's, which is the one this command
/// resolved for the entry it was told to serve — the same value `nvs run`
/// installs for that file. A mount whose entry matches a different `[[app]]`
/// block than the served one does not get that block's origin yet, which is
/// this command's per-application gap and not this key's.
fn fall_back_to(mounts: &mut [Mounted], app: Option<&str>) {
    for mount in mounts {
        if mount.origin.is_none() {
            mount.origin = app.map(str::to_owned);
        }
    }
}

/// One mount's entry compiled, and
/// `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s boot check asked
/// of what came back: a unit that builds an absolute link under a mount that
/// resolved no origin refuses the start.
///
/// **Per resolved mount, and here rather than at the expansion**, because the
/// question needs the compiled unit and § 3 needs it per row: one entry serves
/// every tenant a `scan` glob enumerated, and each of those rows resolves its
/// own origin, so the same unit is a refusal under one and a start under
/// another. `nvs_config::mount`'s expander owns the substitution that decides
/// which, and this is the first point at which both halves are in hand.
///
/// **The failure is at deploy time rather than in a sent message**, which is
/// the whole of what the check buys: the alternative is a program that throws
/// out of `Core\Router::urlAbsolute` on whichever request first builds a link,
/// long after an operator stopped reading the start. It re-runs on reload for
/// the same reason a mount is expanded again there.
///
/// # Errors
///
/// The front end's own summary for an entry that does not compile — it renders
/// its diagnostics itself (`crate::script`) — and a sentence naming the mount,
/// the entry and the two keys that resolve an origin for it.
fn compiled_under(compiler: &Compiler, mount: &Mounted) -> Result<(), String> {
    let entry = mount.entry.to_string_lossy().into_owned();
    let (_program, routes) = compiler.compiled(&entry)?;
    if routes.absolute_links() && mount.origin.is_none() {
        return Err(format!(
            "the mount at `{}` serves `{entry}`, which builds an absolute link, and no origin \
             resolves for it. `rule:routing/an-origin-is-per-mount-and-checked-at-boot`: give \
             this mount's `[[server.mount]]` block an `origin`, or `[[app]] origin` for every \
             mount that has none of its own",
            mount.prefix
        ));
    }
    Ok(())
}

/// `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s origin, from the
/// row § 4 step 1 chose onto the isolate that is about to answer the request.
///
/// **Nothing here resolves anything.** `Mounted::origin` is already
/// substituted — `{1}` is gone by the time a mount exists — so a request that
/// selected a host mount gets that tenant's origin and no other, which is the
/// whole of why the origin is per mount rather than per process.
///
/// It goes onto the isolate rather than onto the carrier beside
/// `nvs_server::mount::carry`'s prefix because it is not a fact about the
/// request: `nvs_host::Isolate::at_origin` owns that reading, and
/// `nvs_runtime::Ctx::set_origin` is where the child's context takes it.
///
/// **A mount that resolved no origin leaves the isolate as it was**, which is
/// the fail-closed direction: `Core\Router::urlAbsolute` throws in a program
/// with no origin rather than linking to an authority somebody guessed, and
/// § 3's boot check is what turns that throw into a start that fails instead.
fn at_mount_origin(isolate: Isolate, mount: &Mounted) -> Isolate {
    match &mount.origin {
        Some(origin) => isolate.at_origin(origin),
        None => isolate,
    }
}

/// One resolved `listen` entry, as the operator wrote it.
///
/// A function rather than a `Display` on [`Listen`]: what an address and a
/// socket path have in common is that they are both what somebody typed, and
/// that is a fact about this command's messages rather than about the type.
fn written_as(entry: &Listen) -> String {
    match entry {
        Listen::Tcp(addr) => addr.to_string(),
        Listen::Unix(path) => path.display().to_string(),
    }
}

/// Every socket this server binds: the file's entries in the order written,
/// then whichever flag had the last word.
///
/// A [`Listen`] out as well as in, one step resolved: the flags have had their
/// say, the duplicates are gone, and a platform with no Unix-domain listener
/// has refused the entries it cannot bind — so what comes back is a set
/// [`bind_all`] can take one at a time with nothing left to decide.
///
/// `--listen` names one address and replaces the whole set, because that is
/// what overriding an array with a single spelling means. `--port` is the last
/// word for its own key alone and keeps each entry's host.
///
/// # Errors
///
/// A `--listen` that is not a literal address, and — on a platform with no
/// Unix-domain listener — every Unix entry the set holds. **One refusal names
/// every entry it applies to**, because the set is read here, once, before any
/// listener is bound. The refusal is a sentence rather than a
/// [`nvs_diagnostics::Diagnostic`] when it came from a flag, because a command
/// line has no file and no span to point into.
fn addresses(
    configured: &[Listen],
    listen: Option<&str>,
    port: Option<u16>,
) -> Result<Vec<Listen>, String> {
    if let Some(written) = listen {
        return written
            .parse::<SocketAddr>()
            .map(|addr| vec![Listen::Tcp(addr)])
            .map_err(|_| {
                format!(
                    "`--listen {written}` is not an address and a port; write `127.0.0.1:8000` or \
                     `[::1]:8000`"
                )
            });
    }
    let mut bound: Vec<Listen> = Vec::with_capacity(configured.len());
    #[cfg(windows)]
    let mut unsupported = Vec::new();
    for entry in configured {
        let resolved = match (entry, port) {
            (Listen::Tcp(addr), None) => Listen::Tcp(*addr),
            // The flag is the last word for its own key alone: a port written
            // here keeps the host the file chose, so `--port` over a `0.0.0.0:80`
            // does not quietly narrow the deployment to loopback.
            (Listen::Tcp(addr), Some(port)) => Listen::Tcp(SocketAddr::new(addr.ip(), port)),
            // And over a Unix entry it names a whole address, since there is no
            // host in one to keep. `127.0.0.1` because that is § 5's own default
            // and the one a development machine means.
            (Listen::Unix(_), Some(port)) => {
                Listen::Tcp(SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port)))
            }
            #[cfg(unix)]
            (Listen::Unix(path), None) => Listen::Unix(path.clone()),
            #[cfg(windows)]
            (Listen::Unix(path), None) => {
                unsupported.push(format!("`{}`", path.display()));
                continue;
            }
        };
        // Two entries that resolve to one socket are one socket: the platform
        // has no second one to give, and `--port` over a mixed set is how two
        // entries collapse into one. Port `0` is never a duplicate — it asks
        // for another free port, and the platform answers a different one.
        let ephemeral = matches!(resolved, Listen::Tcp(addr) if addr.port() == 0);
        if ephemeral || !bound.contains(&resolved) {
            bound.push(resolved);
        }
    }
    #[cfg(windows)]
    if !unsupported.is_empty() {
        return Err(format!(
            "`[server] listen` asks for the Unix-domain socket{} {}, and this platform has no \
             listener for one; write `--listen 127.0.0.1:8000` or a `host:port` entry",
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

/// The boot's last words: one line per socket for whoever is reading the
/// terminal, then `READY=1` for whatever started this process.
///
/// The path is printed as it was written rather than the canonical one the
/// table holds, because an operator reads these against the command they typed.
/// Each socket names *itself* ([`Socket::named`]) rather than what was asked
/// for, so an entry written with port `0` prints the port the platform chose.
///
/// **`READY=1` belongs here and last**, because what that state claims is
/// exactly what the lines above report: every socket in the set is bound, and
/// every mounted entry was compiled before any of them was. A `Type=notify`
/// unit whose process says it any earlier is one `systemctl start` returns from
/// while the port still refuses (`crate::service`).
fn listening(bound: &[Socket], path: &Path, told: &Notify) {
    for listener in bound {
        println!("listening on {} — {}", listener.named(), path.display());
    }
    told.state(State::Ready);
}

/// Every socket the process opens, in the order it opens them: the control
/// endpoint `controlled` names, then every address [`addresses`] gave.
///
/// **The order is the point of this function existing at all.**
/// `rule:config/one-local-control-socket` puts the endpoint's directory under
/// the same trust boundary a configuration file gets, and a boot that bound its
/// listeners first would have a server answering requests out of a tree it is
/// about to refuse to be controlled over. So the endpoint is created — and its
/// directory refused — while nothing is listening, and a refusal leaves a
/// process that never accepted anything rather than one that has to be stopped.
///
/// `guard` is the directory check rather than a call to
/// [`nvs_server::control::boundary`] for the reason
/// [`nvs_config::control::bind`] gives: production passes that function, and a
/// test passes a verdict so the *order* is assertable on a platform whose
/// filesystem cannot be put in the refused state.
///
/// # Errors
///
/// The refusal as one line — the endpoint's directory, the endpoint itself, or
/// the first address the platform would not give.
fn bind_sockets(
    controlled: &Address,
    wanted: &[Listen],
    mode: u32,
    guard: impl FnOnce(&Path) -> Result<(), nvs_config::trust::Untrusted>,
) -> Result<(Option<Endpoint>, Vec<Socket>), String> {
    let controlling = match controlled {
        Address::Disabled => None,
        Address::Local(name) => {
            Some(nvs_server::control::bind(name, guard).map_err(|refusal| {
                format!(
                    "`[control] socket` names `{}`, and this process may not create it: {}",
                    name.display(),
                    refusal.message()
                )
            })?)
        }
    };
    Ok((controlling, bind_all(wanted, mode)?))
}

/// One socket the boot bound, before any core has taken a handle on it.
///
/// A `std` listener and not one of `nvs_host`'s: [`bind_sockets`]' order is the
/// point of it existing, and that order runs before any core does, so each core
/// takes its own handle on the descriptor afterwards ([`Self::try_clone`]) and
/// turns it into a parking listener there ([`Self::accepting`]).
///
/// [`Listen`]'s two arms one step further on, and the last place the family is
/// a question this file asks: past [`Self::accepting`] a core holds a
/// `dyn nvs_server::Listening` and every connection is an
/// `nvs_host::NvsConnection`.
#[derive(Debug)]
enum Socket {
    /// A bound port.
    Tcp(std::net::TcpListener),
    /// A bound Unix-domain path, created with `[server] socket_mode` and not
    /// removed by this process's exit — `nvs_host::NvsUnixListener::bind` owns
    /// why unlinking a socket path is not something the process that bound it
    /// can do safely.
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixListener),
}

impl Socket {
    /// What this socket is, for a line an operator reads.
    ///
    /// Asked of the socket rather than of the entry that named it, so a `:0`
    /// prints the port the platform actually chose. A socket that will not
    /// answer says so instead of this function inventing an address.
    fn named(&self) -> String {
        match self {
            Self::Tcp(socket) => socket.local_addr().map_or_else(
                |_| "an address this process no longer holds".to_owned(),
                |addr| format!("http://{addr}"),
            ),
            #[cfg(unix)]
            Self::Unix(socket) => socket
                .local_addr()
                .ok()
                .and_then(|addr| addr.as_pathname().map(std::path::Path::to_path_buf))
                .map_or_else(
                    || "an unnamed Unix-domain socket".to_owned(),
                    |path| format!("unix:{}", path.display()),
                ),
        }
    }

    /// Another handle on this same socket.
    ///
    /// # Errors
    ///
    /// The platform's, for a descriptor it will not duplicate.
    fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            Self::Tcp(socket) => socket.try_clone().map(Self::Tcp),
            #[cfg(unix)]
            Self::Unix(socket) => socket.try_clone().map(Self::Unix),
        }
    }

    /// The address a bound port is on.
    ///
    /// # Panics
    ///
    /// For a Unix-domain socket, which has no such thing: the cases that ask
    /// this are the ones that then connect to a port.
    #[cfg(test)]
    fn address(&self) -> SocketAddr {
        match self {
            Self::Tcp(socket) => socket
                .local_addr()
                .expect("a bound socket knows its own address"),
            #[cfg(unix)]
            Self::Unix(_) => panic!("a Unix-domain socket has no address and a port"),
        }
    }

    /// This core's parking listener over the descriptor, ready to accept.
    ///
    /// Boxed because one `[server] listen` set may hold both families and a
    /// core accepts on every entry of it: what varies is behind one pointer
    /// rather than in the type of the loop, which is
    /// [`nvs_server::Listening`]'s own reason for being object-safe.
    ///
    /// # Errors
    ///
    /// The platform refused the mode change to non-blocking.
    fn accepting(self) -> std::io::Result<Box<dyn nvs_server::Listening>> {
        match self {
            Self::Tcp(socket) => Ok(Box::new(NvsListener::from_std(socket)?)),
            #[cfg(unix)]
            Self::Unix(socket) => Ok(Box::new(nvs_host::NvsUnixListener::from_std(socket)?)),
        }
    }
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
/// The platform's, for the first entry it refuses — an address already in use
/// or not one of this host's, a socket path that already exists — with that
/// entry named, because a refusal an operator has to guess the subject of is
/// one they read the configuration twice for.
fn bind_all(wanted: &[Listen], mode: u32) -> Result<Vec<Socket>, String> {
    wanted.iter().map(|entry| bind_one(entry, mode)).collect()
}

/// One entry bound, with `mode` on it if it is a socket path.
///
/// # Errors
///
/// [`bind_all`]'s, for this one entry.
#[cfg(unix)]
fn bind_one(entry: &Listen, mode: u32) -> Result<Socket, String> {
    use std::os::unix::fs::PermissionsExt as _;

    let path = match entry {
        Listen::Tcp(addr) => {
            return std::net::TcpListener::bind(addr)
                .map(Socket::Tcp)
                .map_err(|error| format!("could not listen on {addr}: {error}"));
        }
        Listen::Unix(path) => path,
    };
    // **The mask first, and a `set_permissions` only after that.** A socket
    // created under an ordinary `022` is world-connectable, and on this
    // transport `rule:http-server/a-unix-socket-listener` makes that
    // world-*trusted* — so a mode put on afterwards alone would leave a window
    // in which any local account can connect, and the connection would sit in
    // the backlog until the accept loops this boot has not started yet reach
    // it. Narrowing the mask first means the socket never exists wider than
    // `mode`; the call below only ever adds back what the mask denied.
    //
    // The mask is the whole process's, and this does not rest on the boot
    // being single-threaded: it is *narrower* for the length of the bind, so
    // anything else creating a file inside that window gets one more private
    // than it asked for and never a wider one.
    let mask = libc::mode_t::try_from(0o777 & !mode).unwrap_or(0o077);
    let previous = masked_to(mask);
    let bound = std::os::unix::net::UnixListener::bind(path);
    masked_to(previous);
    let listener = bound.map_err(|error| {
        format!(
            "could not listen on `{}`: {error}; a path that already exists is not reused, so \
             remove a socket no server holds before starting this one",
            path.display()
        )
    })?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|error| {
        format!(
            "`{}` was bound and `[server] socket_mode` could not be put on it: {error}",
            path.display()
        )
    })?;
    Ok(Socket::Unix(listener))
}

/// Puts `mask` on this process's file-creation mask, answering the one it
/// replaced.
///
/// The one call in this file that is not safe Rust, and the only one that can
/// bound the mode a `bind` *creates* rather than correcting it afterwards —
/// which is the window [`bind_one`] explains. It touches nothing but this
/// process's own mask and cannot fail.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "`umask` is the only call that bounds the mode a socket is created with, and it \
              reads and writes one word of this process's own state"
)]
fn masked_to(mask: libc::mode_t) -> libc::mode_t {
    unsafe { libc::umask(mask) }
}

/// One entry bound. `mode` names a Unix-domain socket's permissions, and this
/// platform has no such socket to put them on — [`addresses`] has already
/// refused the whole set over any entry that asked for one, so the only entry
/// that reaches here is an address.
///
/// # Errors
///
/// [`bind_all`]'s, for this one entry.
#[cfg(windows)]
fn bind_one(entry: &Listen, _mode: u32) -> Result<Socket, String> {
    match entry {
        Listen::Tcp(addr) => std::net::TcpListener::bind(addr)
            .map(Socket::Tcp)
            .map_err(|error| format!("could not listen on {addr}: {error}")),
        Listen::Unix(path) => Err(format!(
            "`{}` is a Unix-domain socket, and this platform has no listener for one",
            path.display()
        )),
    }
}

/// One row of handles per worker: every socket the boot bound, duplicated once
/// for each core that will accept on it.
///
/// A duplicate is the same socket — [`Socket::try_clone`] — so
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
fn handles_for(bound: &[Socket], workers: usize) -> std::io::Result<Vec<Vec<Socket>>> {
    (0..workers)
        .map(|_| {
            bound
                .iter()
                .map(Socket::try_clone)
                .collect::<std::io::Result<Vec<_>>>()
        })
        .collect()
}

/// The exporter's listener, where `[metrics]` asks for one.
///
/// `None` covers every way of not asking — no `[metrics]` block, `exporter =
/// false`, and an `otlp` exporter, which pushes and is never scraped — and is
/// the reading `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`
/// wants: a deployment that configured no scrape endpoint opens no port.
///
/// **A `prometheus` exporter with no `listen` is refused rather than defaulted.**
/// The address a scrape arrives at decides who on the network can read this
/// process's series, and picking one on an operator's behalf is
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s unsafe
/// default: the tree would open a port nobody wrote down. `nvs.toml`'s own
/// commented line is the one to uncomment.
///
/// **A build without the `exporter` feature refuses either protocol here**, which
/// is [`exporter_not_built`]'s sentence: a tree that configured an exporter has
/// asked this process for something it has no code for, and answering by serving
/// nothing would leave an operator watching a collector that never fills. It is
/// the one refusal this function makes before reading `listen` at all.
///
/// # Errors
///
/// The refusal as one line: an exporter this build cannot run, an exporter with
/// nowhere to answer, an address that does not resolve, or the platform's own on
/// the bind.
fn scrape_socket(config: &nvs_config::Config) -> Result<Option<Socket>, String> {
    let Some(metering) = nvs_config::Metering::of(config) else {
        return Ok(None);
    };
    if let Some(refusal) = exporter_not_built(metering.exporter, cfg!(feature = "exporter")) {
        return Err(refusal);
    }
    if metering.exporter != nvs_config::Exporter::Prometheus {
        return Ok(None);
    }
    let written = config
        .metrics
        .as_ref()
        .and_then(|metrics| metrics.listen.as_deref())
        .ok_or_else(|| {
            "`[metrics] exporter` is `prometheus` and `[metrics] listen` names no address, so a \
             scrape would have nowhere to arrive: write one, or `exporter = false`"
                .to_owned()
        })?;
    let address = written
        .to_socket_addrs()
        .map_err(|error| {
            format!("`[metrics] listen` names `{written}`, which does not resolve: {error}")
        })?
        .next()
        .ok_or_else(|| {
            format!("`[metrics] listen` names `{written}`, which resolves to no address at all")
        })?;
    // The same bind every `[server] listen` entry takes, so a refusal reads the
    // same way whichever socket it was. The mode is the Unix-domain one and
    // this entry is always a port: `rule:config/no-network-control-surface`
    // keeps an operator surface local, and a scrape endpoint is read by a
    // collector on the network rather than by an operator on this host.
    Ok(Some(
        bind_all(&[Listen::Tcp(address)], 0o660)?.swap_remove(0),
    ))
}

/// What a build carrying no exporter owes a tree that configured one, or `None`.
///
/// `built` is `cfg!(feature = "exporter")` at the one call site, and a parameter
/// rather than a `#[cfg]` in this body so that the sentence an operator reads is
/// written once and asserted in the same build everything else here is tested in.
///
/// `None` is every build that has the feature, and — because
/// `nvs_config::Metering::of` already answers `None` for all three ways of writing
/// `exporter = false` — every tree that asked for nothing. So the refusal is owed
/// exactly where an operator wrote a protocol that this binary cannot speak, which
/// is what `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`
/// asks a featureless build to say rather than start quietly without it.
fn exporter_not_built(asked: nvs_config::Exporter, built: bool) -> Option<String> {
    if built {
        return None;
    }
    let protocol = match asked {
        nvs_config::Exporter::Prometheus => "prometheus",
        nvs_config::Exporter::Otlp => "otlp",
    };
    Some(format!(
        "`[metrics] exporter` is `{protocol}` and this `nvs` was built without the `exporter` \
         feature, so it can neither serve a scrape nor push to a collector: run a build that has \
         the feature, or write `exporter = false`"
    ))
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
        Address, Compiler, Ctx, Inbound, Isolate, Listen, Mounted, Notify, Output, OutputSink,
        Socket, SocketAddr, TaskRoot, Value, addresses, at_mount_origin, bind_all, bind_sockets,
        compiled_under, exporter_not_built, fall_back_to, handles_for, listening, one_mount,
        scrape_socket, sweep_orphans, workers_for,
    };
    use std::cell::Cell;
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::num::NonZeroUsize;
    use std::ops::ControlFlow;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

    /// A `scope = "fleet"` entry is armed exactly when this host has a lease to
    /// hold it with, which is the whole of `nvs_server::schedule`'s § 3 seam
    /// seen from the one crate that can close it.
    ///
    /// The same tree is armed twice — once with the lease this command builds
    /// from `[cache.shared]`, once with the [`None`] that was the only answer
    /// before it existed — because the feature is the *difference*: an entry
    /// that arms either way would prove nothing about the lease, and the note
    /// the second one writes is § 3's refusal to fire on each host's own clock.
    ///
    /// The lease is then asked the question the ticker asks, against the store
    /// itself: a key is taken once and refused the second time. Skipped where
    /// `tests/db/compose.yaml`'s `redis` is not running, since a set-if-absent
    /// is the server's semantics and nothing here stands in for it.
    #[test]
    fn serve_arms_a_fleet_entry_when_the_shared_tier_can_take_a_lease() {
        let config = config_of(
            "[cache.shared]\nurl = 'redis://127.0.0.1:16379'\n\n[[schedule]]\nname = \
             'fleet-case'\ncron = '0 3 * * *'\nscript = 'jobs/report.nvs'\nscope = 'fleet'\n",
        );
        let Some(lease) = super::fleet_lease(&config) else {
            eprintln!(
                "serve_arms_a_fleet_entry_when_the_shared_tier_can_take_a_lease asserted \
                 nothing: `docker compose -f tests/db/compose.yaml up -d --wait redis` starts \
                 the store it is written against"
            );
            return;
        };

        let mut notes: Vec<String> = Vec::new();
        let armed = nvs_server::arm(
            &config.schedule,
            &super::Zoned::now(),
            Some(&lease as &dyn nvs_server::Leases),
            |note| notes.push(note.to_owned()),
        );
        assert_eq!(
            armed.len(),
            1,
            "a fleet entry is armed when this host can hold its lease: {notes:?}"
        );
        assert!(
            notes.is_empty(),
            "an armed entry is not also refused: {notes:?}"
        );

        let unarmed = nvs_server::arm(&config.schedule, &super::Zoned::now(), None, |note| {
            notes.push(note.to_owned());
        });
        assert!(
            unarmed.is_empty(),
            "with no lease to hold, the same entry is left unarmed"
        );
        assert_eq!(notes.len(), 1, "and it is named once: {notes:?}");

        let key = format!("serve-arms-{}", std::process::id());
        let held = std::time::Duration::from_secs(1);
        assert!(
            nvs_server::Leases::take(&lease, &key, held),
            "the lease this command built reaches the store and takes a key"
        );
        assert!(
            !nvs_server::Leases::take(&lease, &key, held),
            "and the store, not this process, is what refuses the second asker"
        );
        assert!(
            nvs_server::Leases::renew(&lease, &key, held),
            "and the renewal a fire runs on its own task holds the key this process took"
        );
    }

    /// A unit for a fire to be handed, so that a case about the configuration a
    /// fire runs under does not also need a compiler. It answers every path,
    /// which is what makes it usable as the control below: whether a fire got an
    /// isolate is then entirely a question of which side of `script.spawn` the
    /// entry's path fell on, since the door is asked before any resolver is
    /// (`nvs_runtime::script::resolve`).
    #[derive(Debug)]
    struct Compiles;

    impl nvs_runtime::script::Resolver for Compiles {
        fn resolve(&self, _path: &str) -> Result<nvs_runtime::script::Program, String> {
            Ok(Box::new(|_ctx, _args| Value::null()))
        }
    }

    /// A deployment whose one entry fires `script` under a `[limits] memory`
    /// ceiling, granting `script.spawn` over `over` and nothing else.
    fn firing(script: &std::path::Path, over: Option<&std::path::Path>) -> String {
        let grant = match over {
            Some(root) => format!("[capabilities]\nscript.spawn = ['{}']\n\n", root.display()),
            None => String::new(),
        };
        format!(
            "[limits]\nmemory = '512M'\n\n{grant}[[schedule]]\nname = 'nightly'\ncron = '0 3 * * \
             *'\nscript = '{}'\nscope = 'host'\n",
            script.display()
        )
    }

    /// The written tree as a snapshot, with both halves filled: a directive is
    /// read off the table a boot kept and the typed tree is the view of it
    /// ([`nvs_config::Snapshot::table`]), so a case leaving one of them empty
    /// asserts against a deployment that configured nothing.
    fn tree_of(written: &str) -> nvs_config::Snapshot {
        nvs_config::Snapshot {
            config: config_of(written),
            table: written.parse().expect("the case writes valid TOML"),
            ..Default::default()
        }
    }

    /// One tree in the holder a reload publishes into, which is what a core hands
    /// its ticker.
    fn holding(written: &str) -> Arc<nvs_config::Current> {
        Arc::new(nvs_config::Current::new(Arc::new(tree_of(written))))
    }

    /// A directory of this case's own, with one script in it for an entry to
    /// name.
    fn scheduled_script(case: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("nvs-serve-{case}-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("the platform temporary root is writable");
        let script = root.join("nightly.nvs");
        std::fs::write(&script, b"<?php\n").expect("the case writes its own script");
        (root, script)
    }

    /// `rule:config/a-scheduled-run-is-a-root-isolate`'s two halves, which the
    /// ticker cannot supply and this crate can: a fire's budget is the
    /// deployment's `[limits]`, and its grants are the deployment's
    /// `[capabilities]`.
    ///
    /// The grant half is asserted as a **difference** rather than as a `Some`,
    /// because the same resolver answers both runs: the tree that wrote the grant
    /// gets an isolate and the tree that wrote none is refused at the door. A
    /// fire whose context held no configuration at all would look exactly like
    /// the second one, which is what this closes.
    #[test]
    fn a_scheduled_fire_runs_under_the_deployments_configuration() {
        let (root, script) = scheduled_script("fire-configured");
        let granted = firing(&script, Some(&root));
        let armed = nvs_server::arm(
            &config_of(&granted).schedule,
            &super::Zoned::now(),
            None,
            |note| panic!("a `scope = \"host\"` entry needs no lease and was not armed: {note}"),
        );
        assert_eq!(
            armed.len(),
            1,
            "the case wrote one entry for the ticker to hold"
        );

        let fires = super::Scheduled {
            current: holding(&granted),
        };
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert_eq!(
            ctx.memory_limit(),
            0,
            "the context the ticker builds a fire on holds no tree, so it starts uncapped"
        );
        let built = nvs_runtime::script::scoped(&Compiles, || {
            nvs_server::Fires::isolate(&fires, &armed[0], &mut ctx)
        });
        assert!(
            built.is_some(),
            "the fire's context did not carry the `script.spawn` grant its deployment wrote, so \
             the resolve was refused at the door"
        );
        // The written ceiling less the slice `rule:errors/on-limit` carves out of
        // it for the tier-1 handler, which is what ordinary execution is held to
        // and is bounded by a quarter of the ceiling
        // (`nvs_runtime::Ctx::refresh_limits`). Asserted as that band rather than
        // as one number, so the reserve's own size stays the runtime's to choose.
        let ceiling = 512 * 1024 * 1024;
        let held = ctx.memory_limit();
        assert!(
            held > 0 && held <= ceiling && ceiling - held <= ceiling / 4,
            "a fire is charged against the deployment's `[limits] memory = '512M'`, and this one \
             was held to {held} byte(s)"
        );

        let ungranted = super::Scheduled {
            current: holding(&firing(&script, None)),
        };
        let mut ctx = Ctx::new(OutputSink::Sink);
        let refused = nvs_runtime::script::scoped(&Compiles, || {
            nvs_server::Fires::isolate(&ungranted, &armed[0], &mut ctx)
        });
        assert!(
            refused.is_none(),
            "the same entry under a deployment granting no `script.spawn` still reached a resolver"
        );
    }

    /// The tree a fire runs under is the published one, which is
    /// `rule:config/an-edit-reaches-the-next-request-without-a-restart` read for
    /// a fire: the ticker holds its [`super::Scheduled`] for as long as the
    /// process runs, so a snapshot cloned into it at boot would outlive every
    /// reload.
    ///
    /// What a reload does **not** reach is the roster itself, which is armed once
    /// off the boot tree — [`super::Scheduled`]'s own `# Known gaps` owns that,
    /// and it is why the entry here is the same one on both sides of the publish.
    #[test]
    fn a_fire_reads_the_published_tree_and_not_the_one_its_ticker_was_armed_on() {
        let (root, script) = scheduled_script("fire-reloaded");
        let booted = firing(&script, None);
        let armed = nvs_server::arm(
            &config_of(&booted).schedule,
            &super::Zoned::now(),
            None,
            |note| panic!("a `scope = \"host\"` entry needs no lease and was not armed: {note}"),
        );
        let fires = super::Scheduled {
            current: holding(&booted),
        };

        let mut ctx = Ctx::new(OutputSink::Sink);
        let before = nvs_runtime::script::scoped(&Compiles, || {
            nvs_server::Fires::isolate(&fires, &armed[0], &mut ctx)
        });
        assert!(
            before.is_none(),
            "the boot tree granted no `script.spawn`, so this fire had nothing to run"
        );

        fires
            .current
            .publish(tree_of(&firing(&script, Some(&root))))
            .expect("the case publishes a tree that changes no `Boot` key");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let after = nvs_runtime::script::scoped(&Compiles, || {
            nvs_server::Fires::isolate(&fires, &armed[0], &mut ctx)
        });
        assert!(
            after.is_some(),
            "the next fire after a reload ran under the tree the boot resolved rather than the \
             one the reload published"
        );
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
            vec![tcp("127.0.0.1:9001")]
        );
        assert!(addresses(&configured, Some("localhost:9001"), None).is_err());
        assert_eq!(
            addresses(&configured, None, Some(9001)).expect("an address per entry"),
            vec![tcp("0.0.0.0:9001"), tcp("127.0.0.1:9001")]
        );
        assert_eq!(
            addresses(&configured, None, None).expect("the file's own set"),
            vec![tcp("0.0.0.0:80"), tcp("127.0.0.1:8000")]
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
            vec![tcp("0.0.0.0:80"), tcp("127.0.0.1:8000"), tcp("[::1]:8100"),]
        );

        let ephemeral = vec![tcp("127.0.0.1:0"), tcp("127.0.0.1:0"), tcp("127.0.0.1:0")];
        let wanted = addresses(&ephemeral, None, None).expect("three loopback entries");
        assert_eq!(wanted.len(), 3);
        let listeners = bind_all(&wanted, 0o660).expect("three free ports on the loopback");
        let mut answered = Vec::new();
        for listener in &listeners {
            let addr = listener.address();
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
        let bound = bind_all(
            &addresses(&configured, None, None).expect("two loopback entries"),
            0o660,
        )
        .expect("two free ports on the loopback");
        let listening: Vec<SocketAddr> = bound.iter().map(Socket::address).collect();

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
                    row.iter().map(Socket::address).collect::<Vec<_>>()
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

    /// `rule:http-server/a-unix-socket-listener` from both of the ends it
    /// has, which is one per platform: the entry is bound with `[server]
    /// socket_mode` on it where the rule admits a Unix socket, and the whole
    /// set is refused once where it does not.
    ///
    /// One case and not two, because the rule is one sentence with a platform
    /// in it and a case per leg would let the other leg's half rot unnoticed.
    /// The refusal half asserts *once* over two entries rather than a message
    /// per entry, since taking it in [`addresses`] is what makes a deployment
    /// mistake one report before any socket exists. The bind half asserts the
    /// mode on the socket the platform actually created and then connects to
    /// it, which is what a mode silently left at the umask's answer and a path
    /// that was created but never listened on would each fail. The flag is
    /// asserted on both legs, because a configuration a proxy should prefer
    /// must not fail to start with nothing to try.
    #[test]
    fn a_unix_socket_listen_entry_binds_with_its_mode_on_unix_and_is_refused_once_elsewhere() {
        #[cfg(windows)]
        {
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
                refusal.matches("no listener for one").count(),
                1,
                "one deployment mistake is one refusal: {refusal}"
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let dir = scratch_directory("unix-listener");
            let configured = vec![
                Listen::Unix(dir.join("nvs.sock")),
                Listen::Unix(dir.join("nvs-admin.sock")),
            ];
            let wanted = addresses(&configured, None, None).expect("two socket paths");
            assert_eq!(
                wanted, configured,
                "a socket path is admitted rather than refused on this platform"
            );
            // Not `0660`, which is the default: a mode the case wrote is the
            // only one that can tell the directive being read apart from the
            // directive's absence happening to agree with it.
            let bound = bind_all(&wanted, 0o640).expect("two paths this account may create");
            for (socket, entry) in bound.iter().zip(&configured) {
                let Listen::Unix(path) = entry else {
                    unreachable!("the resolved set is socket paths alone")
                };
                assert_eq!(
                    std::fs::metadata(path)
                        .expect("a bound socket is a name on disk")
                        .permissions()
                        .mode()
                        & 0o777,
                    0o640,
                    "`{}` carries `[server] socket_mode` and not the umask's answer",
                    path.display()
                );
                assert_eq!(socket.named(), format!("unix:{}", path.display()));
                std::os::unix::net::UnixStream::connect(path).unwrap_or_else(|error| {
                    panic!("nothing is listening on `{}`: {error}", path.display())
                });
            }
        }
        assert_eq!(
            addresses(
                &[Listen::Unix(PathBuf::from("/run/nvs.sock"))],
                None,
                Some(8080)
            )
            .expect("the flag is the last word"),
            vec![tcp("127.0.0.1:8080")]
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
    ///
    /// **Skipped in the debug profile**, for the reason `benches/abi-probe`'s
    /// guards are: a cost margin holds only on an idle machine, and
    /// `tools/verify.py` runs this binary beside every other test binary in
    /// the workspace. The driver runs it under `--release` once its sweep has
    /// finished and the box is idle — `tools/loop.py`'s release checks.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "a cost margin needs an idle machine; the driver's release slot is one"
    )]
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

    /// The endpoint one case names, in the platform's own namespace: a socket
    /// under [`scratch_directory`] on Unix, a pipe name on Windows.
    fn control_endpoint(case: &str) -> PathBuf {
        #[cfg(unix)]
        {
            scratch_directory(&format!("control-{case}")).join("control.sock")
        }
        #[cfg(windows)]
        {
            PathBuf::from(format!(
                r"\\.\pipe\nvs-serve-control-{}-{case}",
                std::process::id()
            ))
        }
    }

    /// An empty directory of this case's own, beside the test binary.
    ///
    /// Beside the binary and not under `std::env::temp_dir()`, for the reason
    /// `nvs_config::control`'s own cases give:
    /// `rule:config/ownership-is-the-trust-boundary` runs on the parent too,
    /// and a world-writable `/tmp` above a tight directory of ours fails it.
    /// `target/` is owned by this account and writable by neither its group
    /// nor the world, which is the bar an operator's `/run/nvs` clears — and
    /// the bar a socket whose own mode decides who may connect is asserted
    /// against.
    #[cfg(unix)]
    fn scratch_directory(case: &str) -> PathBuf {
        let beside = std::env::current_exe().expect("the test binary knows its own path");
        let dir = beside
            .parent()
            .expect("a test binary sits in a directory")
            .join(format!("nvs-serve-{}-{case}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
        dir
    }

    /// `rule:config/one-local-control-socket`'s address, as the tree a boot
    /// reads it out of: `[control] socket` naming `at` and nothing else written
    /// anywhere.
    fn a_tree_naming(at: &Path) -> Address {
        let config = nvs_config::tree::Config {
            control: Some(nvs_config::tree::Control {
                socket: Some(nvs_config::tree::Setting::Text(
                    at.to_string_lossy().into_owned(),
                )),
            }),
            ..nvs_config::tree::Config::default()
        };
        Address::of(&config).expect("a path names a local endpoint")
    }

    /// An address on the loopback nothing is listening on, taken from the
    /// platform and given straight back so the boot under test is what binds it.
    fn a_free_address() -> SocketAddr {
        std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .expect("the loopback has a free port")
            .local_addr()
            .expect("a bound listener knows its own address")
    }

    /// § 3's order, asserted from inside the trust check: the guard runs while
    /// the address `[server] listen` named is still free, so the endpoint an
    /// operator drives this process over exists before anything could have
    /// accepted a request.
    ///
    /// The listener is bound **after** the guard by the same call, so the flag
    /// could only be false if the two steps were the other way round.
    #[test]
    fn serve_binds_the_control_socket_the_tree_names_before_any_listener_accepts() {
        let name = control_endpoint("binds");
        let wanted = a_free_address();
        let free = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&free);
        let (controlling, bound) = bind_sockets(
            &a_tree_naming(&name),
            &[Listen::Tcp(wanted)],
            0o660,
            move |_| {
                seen.store(
                    std::net::TcpListener::bind(wanted).is_ok(),
                    Ordering::Relaxed,
                );
                Ok(())
            },
        )
        .expect("a control endpoint and every listener the set named");

        assert!(
            free.load(Ordering::Relaxed),
            "`{wanted}` was already bound when the control endpoint was created, so a listener \
             was accepting before the endpoint existed"
        );
        let endpoint = controlling.expect("the tree named a control socket");
        assert_eq!(endpoint.name(), name);
        assert_eq!(bound.len(), 1);
        assert_eq!(
            bound[0].address(),
            wanted,
            "the listener the set named is bound as well as the endpoint"
        );
        nvs_config::control::connect(&name)
            .expect("a client reaches the endpoint that was created");
    }

    /// § 3's directory rule: the boot is refused, and it is refused with
    /// nothing created and nothing listening.
    ///
    /// The verdict stands in for the filesystem state — `nvs_config::control`'s
    /// own cases take the real check, and a directory another account can write
    /// is not a state every platform's test can be put in.
    #[test]
    fn serve_refuses_to_boot_when_the_control_sockets_directory_is_writable_by_another_account() {
        let name = control_endpoint("untrusted");
        let wanted = a_free_address();
        let asked = name.clone();
        let refusal = bind_sockets(
            &a_tree_naming(&name),
            &[Listen::Tcp(wanted)],
            0o660,
            move |at| {
                assert_eq!(
                    at, asked,
                    "the guard is asked about the endpoint the tree named"
                );
                Err(nvs_config::trust::Untrusted::Unreadable(format!(
                    "`{}` is writable by another account",
                    at.display()
                )))
            },
        )
        .expect_err("a directory another account can write refuses the start");

        assert!(
            refusal.contains("writable by another account"),
            "the refusal names the directory check that took it: {refusal}"
        );
        assert!(
            nvs_config::control::connect(&name).is_err(),
            "the refusal left a reachable control endpoint behind"
        );
        assert!(
            std::net::TcpListener::bind(wanted).is_ok(),
            "`{wanted}` was bound by a start that refused, so this process was serving requests \
             on the way to failing"
        );
    }

    /// How long the client below waits on a server that may already have
    /// stopped: a loaded machine is not the failure this case is looking for,
    /// and a run that stopped answering has to fail rather than hang.
    const CLIENT_PATIENCE: Duration = Duration::from_secs(20);

    /// What the connection below is given to say anything more, shortened for
    /// this case from the ten seconds a deployment gets.
    ///
    /// **It is what bounds the close**, because a drain does not yet cut an
    /// idle wait short: a connection sees the drain when its own wait ends, and
    /// `nvs_server::socket`'s `receive` owns that gap for both readers. Both
    /// waits are set because a connection between one response and the next
    /// request is waiting for a request head, so `header` is the one in force
    /// and `keepalive` is here to say that neither is what this case turns on.
    /// What it asserts is unaffected either way: the close is the connection's
    /// own, and it arrives without a reset.
    const KEPT_ALIVE_FOR: Duration = Duration::from_millis(250);

    /// One response head off the wire, read a byte at a time so that nothing
    /// after it is taken with it — what the case asserts next is that the
    /// server writes nothing more.
    fn head_from(socket: &mut std::net::TcpStream) -> String {
        let mut head = Vec::new();
        let mut byte = [0_u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            let got = socket
                .read(&mut byte)
                .expect("the response could not be read");
            assert_eq!(got, 1, "the connection ended before its response head did");
            head.push(byte[0]);
        }
        String::from_utf8(head).expect("a response head is text")
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly` end to end, from
    /// the seam the operating system's handler calls: the delivery begins the
    /// process's drain, the accept loop parked in `accept` on another thread is
    /// told rather than finding out at its next connection, and the connection
    /// already handed over ends in a clean close.
    ///
    /// **Nothing else could have stopped this loop.** Its `keep_serving` seam
    /// answers `Continue` for ever, so the return asserted below is the drain's
    /// and only the drain's — and the drain is the **process's** bit, which
    /// every other case in this binary avoids on purpose
    /// ([`crate::worker`]'s cases say why). Here it is the subject: a signal
    /// handler is handed no server, so the bit it writes has to be the one a
    /// worker takes for itself.
    ///
    /// The close is asserted as *what the client's socket saw*, since that is
    /// the whole of the rule: end of stream after a response that was finished,
    /// never a reset, and nothing written after the head that was already sent.
    #[test]
    fn a_terminating_signal_drains_serve_and_every_connection_closes_cleanly() {
        let _in_turn = ONE_STOP_AT_A_TIME
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::stop::on_termination().expect("this process's terminating signals arm");
        let mut listener =
            nvs_host::NvsListener::bind(a_free_address()).expect("the loopback refused a listener");
        let addr = listener
            .local_addr()
            .expect("a bound listener knows its own address");

        // The client is a thread because this one is about to be the server. It
        // asks for one response, keeps the connection — no `Connection: close`,
        // so what closes it is the server's own decision and not the request's
        // — and only then asks the process to stop.
        let client = std::thread::spawn(move || {
            let mut socket =
                std::net::TcpStream::connect(addr).expect("the loopback refused a socket");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the request could not be written");
            let answered = head_from(&mut socket);
            // The delivery itself, made from a thread that is not a core's —
            // which is where a signal handler's call comes from.
            crate::stop::deliver();
            let mut after = Vec::new();
            let closed = socket.read_to_end(&mut after);
            (answered, after, closed)
        });

        let serving = nvs_server::Serving::new(
            Arc::new(nvs_server::Admission::new(&nvs_server::Ceiling::of(
                &nvs_config::server::Capacity {
                    configured: u64::MAX,
                    per_request: None,
                    budget: None,
                },
            ))),
            Arc::new(nvs_server::Secure::of(None)),
            Arc::new(nvs_server::Trusted::of(&[]).0),
            Arc::new(nvs_server::Cors::of(None)),
            Arc::default(),
        );
        let handler = Rc::new(
            |_request: nvs_server::Request<nvs_server::Incoming>, _origin: nvs_server::Origin| {
                nvs_server::Reply::healthy()
            },
        );
        let waits = nvs_config::server::Waits {
            header: KEPT_ALIVE_FOR,
            keepalive: KEPT_ALIVE_FOR,
            ..nvs_config::server::Waits::default()
        };
        let draining = nvs_server::Draining::process();
        let returned = Rc::new(Cell::new(false));
        let mut sched = nvs_host::Scheduler::new();
        let installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let draining = draining.clone();
            let returned = Rc::clone(&returned);
            move |_ctx| {
                nvs_server::serve_on_this_core(
                    &mut listener,
                    &handler,
                    waits,
                    &serving,
                    &draining,
                    |_note| {},
                    || ControlFlow::Continue(()),
                )
                .expect("the accept loop failed");
                returned.set(true);
            }
        });
        // The same loop `serve_on_worker` turns, and for its reason: the accept
        // loop is a parked task for as long as it is serving, so a report with
        // anything still parked is a turn to take rather than an end.
        loop {
            match nvs_host::run_until_idle(&mut sched) {
                Ok(report) if report.parked > 0 => {}
                Ok(_) => break,
                Err(error) => panic!("the scheduler stopped: {error}"),
            }
        }
        drop(installed);

        let (answered, after, closed) = client.join().expect("the client thread panicked");
        assert!(
            answered.starts_with("HTTP/1.1 200 OK\r\n"),
            "the request made before the stop was not answered: {answered}"
        );
        assert!(
            draining.is_draining(),
            "the delivery did not begin this process's drain"
        );
        assert!(
            returned.get(),
            "the accept loop was still accepting after the drain began"
        );
        let after = String::from_utf8_lossy(&after).into_owned();
        assert_eq!(
            after, "",
            "the drained server wrote something after the response it had already finished"
        );
        closed.expect("the drained connection ended in a reset rather than a clean close");
    }

    /// The two cases that stop this process take it in turn.
    ///
    /// The drain is one bit for the whole binary and the service manager is one
    /// installed sink beside it, so two cases delivering a stop at once would
    /// each be reading the other's report.
    static ONE_STOP_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A tree of this case's own on disk, and the [`crate::control::Process`]
    /// serving it — the reload driven below is the real one, which re-resolves
    /// these files exactly as the boot that wrote them would.
    ///
    /// The drain is `Draining::detached` because this server's stopping is not
    /// this process's, and this process's is what the case stops afterwards.
    fn a_server_over(case: &str, told: &Notify) -> crate::control::Process {
        let beside = std::env::current_exe().expect("the test binary knows its own path");
        let dir = beside
            .parent()
            .expect("a test binary sits in a directory")
            .join(format!("nvs-serve-{}-{case}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
        let root = dir.join("nvs.toml");
        std::fs::write(&root, "[limits]\nmemory = \"64M\"\n").expect("a tree of this case's own");
        let entry = dir.join("app.nvs");
        std::fs::write(&entry, "fn main(): void {}\n")
            .expect("an entry file for the tree to be about");
        let mut sources = nvs_diagnostics::SourceMap::new();
        let snapshot = crate::config::boot_snapshot(
            std::slice::from_ref(&root),
            &entry,
            &mut sources,
            crate::config::Init::Never,
        )
        .expect("the tree this case wrote resolves");
        let current = Arc::new(nvs_config::snapshot::Current::new(snapshot));
        let capacity = nvs_config::server::capacity_for(&current.load().config, &BTreeMap::new())
            .expect("a tree that named no ceiling has this machine's");
        crate::control::Process::new(
            Arc::clone(&current),
            vec![root],
            entry,
            Arc::new(Compiler::default()),
            Arc::new(nvs_server::Admission::new(&nvs_server::Ceiling::of(
                &capacity,
            ))),
            nvs_server::Draining::detached(),
            told.clone(),
        )
    }

    /// The four states a `Type=notify` unit is owed, in the order a served life
    /// sends them and each taken from the call the process itself makes:
    /// [`listening`] is the boot's last act, `Controlled::reload` is where every
    /// spelling of a reload ends, and [`crate::stop::deliver`] is what a
    /// terminating signal ends in.
    ///
    /// **The manager is a recording sink rather than a datagram socket**,
    /// because a case may not assume it is running under systemd — and on
    /// Windows there is no socket of that kind to bind at all. What is asserted
    /// is still the protocol's own lines, so the seam is the only thing
    /// standing in.
    ///
    /// The reload is the real one over a real tree, so the pair around it is
    /// the pair an operator's `systemctl reload` produces rather than two calls
    /// a case made in the right order.
    #[test]
    fn sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping() {
        let _in_turn = ONE_STOP_AT_A_TIME
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (told, sent) = crate::service::recording();
        told.install();

        let listener =
            std::net::TcpListener::bind(a_free_address()).expect("the loopback refused a listener");
        listening(&[Socket::Tcp(listener)], Path::new("app.nvs"), &told);

        let process = a_server_over("sd-notify", &told);
        nvs_server::control::Controlled::reload(&process)
            .expect("the tree this case wrote reloads");

        // The stop a terminating signal's thread ends in, over a drain of this
        // case's own: the process's bit is begun once for the life of a binary
        // and the case that owns it is the one above, while the manager told
        // here is the process's either way.
        crate::stop::deliver_to(&nvs_server::Draining::detached());

        let sent = sent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(
            sent.as_slice(),
            ["READY=1", "RELOADING=1", "READY=1", "STOPPING=1"],
            "the states this process reported are not the ones a `Type=notify` unit is owed, in \
             the order a served life sends them"
        );
    }

    /// An entry declaring one route and linking to it with `member`, in a
    /// scratch directory of its own so that [`one_mount`] resolves a row off a
    /// real file rather than a struct literal pointing at nothing.
    ///
    /// The link is a *route's*, because that is the only kind there is: a name
    /// is resolved against the unit's own table while it compiles, so the entry
    /// has to declare the route it links to.
    fn an_entry_linking_with(member: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nvs-serve-origin-{}-{member}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to write the entry in");
        let path = dir.join("app.nvs");
        std::fs::write(
            &path,
            format!(
                r#"<?nvs
class Docs {{
    #[Core\Route(path: "/here", method: Core\Http\Method::Get, name: "Docs::here")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function here(): string {{ return "here"; }}
}}

echo Core\Router::{member}("Docs::here", []);
"#
            ),
        )
        .expect("the entry is writable");
        path
    }

    /// What the entry above writes when the door hands its isolate the row
    /// `mount` is — [`super::serve_on_worker`]'s handler with `hyper` and the
    /// accept loop left out, which is the unit off the compiler, the carrier
    /// the door built, and the mount's two crossings onto it.
    ///
    /// Both crossings, because the link is where they meet:
    /// `nvs_server::mount::carry` puts the prefix on the carrier and
    /// [`at_mount_origin`] puts the origin on the isolate, and a served
    /// absolute link is the origin, the prefix and the path in that order.
    fn served_under(compiler: &Compiler, mount: &Mounted) -> (bool, String) {
        let (program, _routes) = compiler
            .compiled(&mount.entry.to_string_lossy())
            .expect("the entry this case wrote compiles");
        let mut inbound = Inbound::new("GET", "/here", "");
        nvs_server::mount::carry(mount, &mut inbound);
        let isolate = at_mount_origin(
            Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
            mount,
        );
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        let done = isolate
            .run(&mut ctx)
            .expect("a null argument crosses into an isolate");
        (
            done.ok,
            String::from_utf8(done.output).expect("a link is text"),
        )
    }

    /// `rule:routing/an-origin-is-per-mount-and-checked-at-boot` at the door:
    /// the mount a request selected is what decides the origin its program
    /// links from, and the process serving it has none of its own.
    ///
    /// **One entry, three mounts**, because the origin is a property of the row
    /// rather than of the unit: the same compiled program is run under each,
    /// so a build that read an origin off the snapshot, off the request or off
    /// the parent context answers all three the same way and fails here.
    ///
    /// The three are the ways the join can be wrong. A tenant mount is the
    /// whole answer — origin, prefix, path — and is the one a host-mounted
    /// deployment depends on. The root mount is the same request under the
    /// prefix that strips nothing, where a prefix written as `/` would double
    /// the separator. A mount with no origin is the fail-closed direction:
    /// `Core\Router::urlAbsolute` throws rather than linking to an authority
    /// somebody guessed, and nothing is written at all.
    #[test]
    fn a_served_request_receives_its_mounts_resolved_origin() {
        let entry = an_entry_linking_with("urlAbsolute");
        let compiler = Compiler::default();
        let mut mount =
            one_mount(&entry).expect("the entry this case wrote is a file in a directory");
        mount.prefix = "/tenant".to_string();
        mount.origin = Some("https://tenant.example.test/".to_string());
        assert_eq!(
            served_under(&compiler, &mount),
            (true, "https://tenant.example.test/tenant/here".to_owned()),
            "a request served through a mount did not link from that mount's origin"
        );

        mount.prefix = "/".to_string();
        assert_eq!(
            served_under(&compiler, &mount),
            (true, "https://tenant.example.test/here".to_owned()),
            "the mount at the root stripped nothing, so nothing belongs between its origin and \
             the path"
        );

        mount.origin = None;
        let (ok, wrote) = served_under(&compiler, &mount);
        assert!(
            !ok && wrote.is_empty(),
            "a mount that resolved no origin answered `{wrote}` instead of refusing the link"
        );
    }

    /// `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s last
    /// paragraph: the throw the case above pins is a *deploy-time* failure, so
    /// the start refuses rather than the request.
    ///
    /// **Three mounts over two entries**, which is the whole predicate: the
    /// check fires on a unit that links absolutely with no origin, is silent
    /// the moment that mount resolves one, and is silent for a unit that links
    /// only relatively — which is the half that decides whether this is a
    /// check or a rule against `#[Route]`. The refusal names the mount, since
    /// one scanned glob is many rows and an operator has to know which.
    ///
    /// The same entry under two mounts is also the reason this is asked per
    /// resolved row rather than per unit: one compiled program is a refusal
    /// under the first mount here and a start under the second.
    #[test]
    fn a_mount_whose_unit_calls_url_absolute_and_resolves_no_origin_refuses_the_boot() {
        let compiler = Compiler::default();
        let absolute = an_entry_linking_with("urlAbsolute");
        let mut mount =
            one_mount(&absolute).expect("the entry this case wrote is a file in a directory");
        mount.prefix = "/tenant".to_string();

        let refusal = compiled_under(&compiler, &mount)
            .expect_err("a unit that links absolutely under a mount with no origin starts");
        assert!(
            refusal.contains("/tenant") && refusal.contains("origin"),
            "the refusal names neither the mount it applies to nor what resolves an origin for \
             it: {refusal}"
        );

        mount.origin = Some("https://tenant.example.test".to_string());
        assert!(
            compiled_under(&compiler, &mount).is_ok(),
            "the mount resolved an origin, so there is nothing left for the check to refuse"
        );

        let relative = an_entry_linking_with("url");
        let mut linking_relatively =
            one_mount(&relative).expect("the entry this case wrote is a file in a directory");
        linking_relatively.prefix = "/tenant".to_string();
        assert!(
            compiled_under(&compiler, &linking_relatively).is_ok(),
            "a unit that builds no absolute link needs no origin, and this start was refused one"
        );
    }

    /// `[app] origin` is the fallback, and it is folded into the row before
    /// anything reads one — so a tree that names its origin there boots, and
    /// its served links carry it.
    ///
    /// **The mount's own key still wins**, which is the half a fold can get
    /// wrong: a fallback applied over a row that wrote its own origin puts one
    /// tenant's authority in another's links, which is the failure
    /// `rule:routing/an-origin-is-per-mount-and-checked-at-boot` exists to
    /// stop.
    #[test]
    fn an_app_origin_is_the_fallback_for_a_mount_that_wrote_none() {
        let entry = an_entry_linking_with("urlAbsolute");
        let compiler = Compiler::default();
        let mut mounts = vec![
            one_mount(&entry).expect("the entry this case wrote is a file in a directory"),
            one_mount(&entry).expect("the entry this case wrote is a file in a directory"),
        ];
        mounts[0].prefix = "/tenant".to_string();
        mounts[1].prefix = "/other".to_string();
        mounts[1].origin = Some("https://other.example.test".to_string());

        fall_back_to(&mut mounts, Some("https://app.example.test"));

        assert_eq!(
            mounts[0].origin.as_deref(),
            Some("https://app.example.test"),
            "a mount that wrote no origin did not take `[app] origin`"
        );
        assert_eq!(
            mounts[1].origin.as_deref(),
            Some("https://other.example.test"),
            "the fallback overwrote an origin the mount wrote for itself"
        );
        assert!(
            compiled_under(&compiler, &mounts[0]).is_ok(),
            "the tree resolved an origin for this mount, so the boot check has nothing to refuse"
        );
        assert_eq!(
            served_under(&compiler, &mounts[0]),
            (true, "https://app.example.test/tenant/here".to_owned()),
            "a request served through a mount that took the fallback did not link from it"
        );
    }

    /// `[metrics] listen` is bound at boot, and a collector connecting to it is
    /// answered with this process's series in the text exposition format.
    ///
    /// The socket half of stage 10 end to end: the address the tree wrote is
    /// resolved and bound by [`scrape_socket`], and what answers on it is the
    /// loop [`serve_on_worker`] spawns. The counted request is made on *this*
    /// thread, which is the core the loop runs on, because what this case is
    /// about is the endpoint — `nvs_server::metrics`'s own case is the one that
    /// proves a scrape crosses cores.
    ///
    /// `Connection: close` is what lets the body be read to the end without a
    /// framing parser here: the case is about what the server wrote, and a
    /// keep-alive connection would leave the client waiting out `keepalive` for
    /// bytes it already has.
    #[cfg(feature = "exporter")]
    #[test]
    fn serve_answers_a_scrape_at_the_metrics_listen_address() {
        let wanted = a_free_address();
        let config = config_of(&format!(
            "[metrics]\nexporter = \"prometheus\"\nlisten = \"{wanted}\"\n"
        ));
        let socket = scrape_socket(&config)
            .expect("a written address was refused")
            .expect("a `prometheus` exporter binds a scrape endpoint");
        let bound = socket.address();
        assert_eq!(bound, wanted, "the endpoint is the address the tree named");

        let client = std::thread::spawn(move || {
            let mut socket =
                std::net::TcpStream::connect(bound).expect("the endpoint refused a socket");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the request could not be written");
            let mut answered = String::new();
            socket
                .read_to_string(&mut answered)
                .expect("the response could not be read");
            answered
        });

        nvs_server::metrics::meter_this_core(&config);
        nvs_server::metrics::count_request(
            "GET",
            200,
            Some("metrics.probe"),
            Duration::from_millis(5),
        );

        let mut listener = socket
            .accepting()
            .expect("the platform refused a parking listener");
        let waits = nvs_config::server::Waits {
            header: KEPT_ALIVE_FOR,
            keepalive: KEPT_ALIVE_FOR,
            ..nvs_config::server::Waits::default()
        };
        // Detached, so that a case elsewhere in this binary delivering the
        // process's own stop does not end this loop before it has accepted.
        let draining = nvs_server::Draining::detached();
        let mut sched = nvs_host::Scheduler::new();
        let installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            nvs_server::serve_scrapes_on_this_core(
                &mut *listener,
                waits,
                &draining,
                |_note| {},
                // One scrape is the whole case, and the loop has no ending of
                // its own: a deployment's is the drain.
                || ControlFlow::Break(()),
            )
            .expect("the scrape loop failed");
        });
        loop {
            match nvs_host::run_until_idle(&mut sched) {
                Ok(report) if report.parked > 0 => {}
                Ok(_) => break,
                Err(error) => panic!("the scheduler stopped: {error}"),
            }
        }
        drop(installed);

        let answered = client.join().expect("the client thread panicked");
        assert!(
            answered.starts_with("HTTP/1.1 200 OK\r\n"),
            "the scrape was not answered: {answered}"
        );
        assert!(
            answered.contains("content-type: text/plain; version=0.0.4; charset=utf-8\r\n"),
            "the body was not sent as the text exposition format: {answered}"
        );
        assert!(
            answered.contains("# TYPE nvs_requests_total counter\r\n")
                || answered.contains("# TYPE nvs_requests_total counter\n"),
            "the exposition carried no declared family: {answered}"
        );
        assert!(
            answered.contains(
                r#"nvs_requests_total{method="GET",route="metrics.probe",status="200"} 1"#
            ),
            "the scrape did not carry this core's own count: {answered}"
        );
    }

    /// A tree whose `[metrics]` asks for nothing opens no port and builds no
    /// registry, which is the cheapest reading of `exporter = false` and the one
    /// `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`
    /// asks for.
    ///
    /// The three trees are the three ways of not asking, and they are one case
    /// because saying them differently must not mean different things: a block
    /// that bounds a registry it never requested is still a tree that requested
    /// nothing.
    #[test]
    fn a_tree_whose_metrics_exporter_is_false_binds_nothing_and_builds_no_registry() {
        for written in [
            "[server]\nworkers = 1\n",
            "[metrics]\nexporter = false\n",
            "[metrics]\nmax_series = 64\n",
        ] {
            let config = config_of(written);
            assert!(
                scrape_socket(&config)
                    .expect("a tree asking for no exporter is not a refusal")
                    .is_none(),
                "a socket was bound for `{written}`"
            );
            assert!(
                nvs_server::Registry::of(&config).is_none(),
                "a registry was built for `{written}`"
            );
        }
    }

    /// A `prometheus` exporter with no address to answer at is a boot refusal,
    /// not a port this process picks for itself.
    ///
    /// The contrast that gives the case above its meaning: what binds nothing is
    /// a tree that asked for nothing, and a tree that asked for an exporter and
    /// forgot where gets told so rather than getting a default.
    ///
    /// A build without the `exporter` feature refuses this tree one step earlier
    /// and for a different reason, which is [`a_build_without_the_exporter_feature_refuses_every_protocol_but_false`]'s.
    #[cfg(feature = "exporter")]
    #[test]
    fn a_prometheus_exporter_with_no_listen_address_is_refused_at_boot() {
        let refusal = scrape_socket(&config_of("[metrics]\nexporter = \"prometheus\"\n"))
            .expect_err("an exporter with nowhere to answer was accepted");
        assert!(
            refusal.contains("`[metrics] listen`"),
            "the refusal did not name the key that is missing: {refusal}"
        );
    }

    /// A build carrying no exporter refuses either protocol at boot and names the
    /// feature, and a tree that asked for nothing is still no refusal at all.
    ///
    /// The half of
    /// `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` that
    /// the gate alone cannot show: what a featureless `nvs` *says*. It is asserted
    /// through [`exporter_not_built`] with `built` written out rather than through
    /// `scrape_socket`, because the build this suite runs in is the one that has
    /// the feature — a case behind `#[cfg(not(feature = ...))]` would assert the
    /// sentence in the only build nobody tests.
    #[test]
    fn a_build_without_the_exporter_feature_refuses_every_protocol_but_false() {
        for asked in [nvs_config::Exporter::Prometheus, nvs_config::Exporter::Otlp] {
            let refusal = exporter_not_built(asked, false)
                .expect("a build with no exporter accepted a tree that configured one");
            assert!(
                refusal.contains("`exporter` feature"),
                "the refusal did not name the feature that is missing: {refusal}"
            );
            assert!(
                refusal.contains("`exporter = false`"),
                "the refusal did not say what to write instead: {refusal}"
            );
            assert!(
                exporter_not_built(asked, true).is_none(),
                "a build carrying the exporter refused `{asked:?}`"
            );
        }
        assert!(
            scrape_socket(&config_of("[metrics]\nexporter = false\n"))
                .expect("a tree asking for no exporter is not a refusal")
                .is_none(),
            "`exporter = false` reached the feature's refusal"
        );
    }
}
