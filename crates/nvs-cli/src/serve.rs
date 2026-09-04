//! `nvs serve`: one core, one listening socket, and every request running one
//! entry file as [ADR 0006]'s isolate.
//!
//! [`nvs_server::serve::serve_on_this_core`] is the loop and
//! [ADR 0138](../../../docs/adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md)
//! is what drives a connection on it; what this module owns is the three things
//! only the binary can supply — the socket the loop accepts on, the clock it
//! holds a connection to, and the handler that says which isolate a request is.
//!
//! # Decision: the configuration's mounts are the table, and a bare file is a table of one
//!
//! [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
//! § 2 is the server's governing rule — a request *selects* an entry point from
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
//! always decides is *which* configuration this is: ADR 0104 § 2 layers the
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
//! # Decision: one socket, and the flag is the last word
//!
//! `[server] listen` is a flat array (§ 5) and this loop is *one core*, so it
//! binds the first entry and says on standard error what it left. Binding all of
//! them is [`nvs_host::NvsListener::from_std`]'s fan-out, which is the slice that
//! gives this command a core count. `--listen` and `--port` override the file, on
//! § 5's own sentence; they conflict with each other, because two spellings of
//! one address is a question guessing an answer to would be worse than refusing.
//!
//! A Unix-domain entry classifies (`nvs_config::server::Listen::Unix`) and is
//! then refused here, because [`nvs_host::NvsListener`] accepts on TCP alone
//! today. That refusal moves the day there is a listener for one; the
//! classification does not.
//!
//! **What it spends**, per [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md):
//! one compiled unit per mounted entry, held for the life of the process and
//! shared by every request that runs it ([ADR 0006]'s "shares immutable compiled code",
//! which is [`crate::script`]'s cache and nothing else), plus whatever the accept
//! loop holds per connection in flight. Nothing accumulates per request answered.
//!
//! [ADR 0006]: ../../../docs/adr/0006-isolated-script-execution.md

use std::cell::Cell;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use nvs_config::mount::Mounted;
use nvs_config::server::{Listen, capacity_for, listen_on, waits_for};
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_host::{Isolate, NvsListener, Output};
use nvs_runtime::script::{Program, Resolver as _};
use nvs_runtime::{Ctx, Inbound, OutputSink, TaskRoot, Value};
use nvs_server::{
    Admission, Arrived, Ceiling, Incoming, OnDisk, Origin, Reply, Request, Resolved, Secure,
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
    // ADR 0078 § 1's snapshot, resolved exactly as `nvs run` resolves it and
    // for the same reason: a tree that does not resolve is a refusal to start.
    // The `[server]` block is `Boot`-class as a whole (ADR 0097 § 5), so this
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
    // ADR 0106 § 13: the ceiling is the smaller of what the file asked for and
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
    // ADR 0074 § 1's header set, resolved once beside the valve: with nothing
    // written under `[http.headers]` it is the whole of what every response this
    // server writes carries beside its body, and `nvs_server::secure` owns the
    // three details § 1 calls decisions rather than transcription.
    // ADR 0097 § 6: who may assert a client address or a scheme, resolved once
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
    let serving = Serving::new(
        Arc::new(Admission::new(&ceiling)),
        Arc::new(Secure::of(snapshot.config.http.as_ref())),
        Arc::new(trusted),
    );
    let addr = match address(&configured, listen, port) {
        Ok(addr) => addr,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };

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
    let table = Rc::new(Table::from_config(mounts, &snapshot.config));

    // § 2, kept before the socket exists: **every** path this server can execute
    // is compiled now, so a program that does not compile is a start that fails
    // rather than a request that does. That is the rule the enumeration exists
    // for, and it costs one compile per mounted entry at boot rather than one
    // per entry per request. The front end renders its own diagnostics
    // (`script`'s module doc), so the message here is the summary.
    let compiler = Rc::new(Compiler::default());
    for mounted in table.mounts() {
        if let Err(message) = compiler.resolve(&mounted.entry.to_string_lossy()) {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    }

    let mut listener = match NvsListener::bind(addr) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("error: could not listen on {addr}: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The path as it was written, not the canonical one the table holds: an
    // operator reads this line against the command they typed.
    println!("listening on http://{addr} — {}", path.display());
    if configured.len() > 1 && listen.is_none() {
        eprintln!(
            "note: `[server] listen` names {} addresses and this core binds the first; \
             one listener per core is the fan-out",
            configured.len()
        );
    }

    // Every request goes through § 4's five steps, and what they chose is either
    // a file to send — `nvs_server::statics`, the same policy a configured
    // deployment serves under — or a file to run as ADR 0006's isolate, the same
    // type `spawn script` runs and deliberately not a second isolation path
    // (ADR 0097's crate doc). With one mount at `/` and `dispatch = "entry"`
    // that is step 5 every time and the resolve is a cache hit on the unit
    // compiled above, so what it costs per request is one `Program` over shared
    // code.
    // The one state the probe reports, and the accept loop below is what writes
    // it: this command never asks the loop to stop yet, so it reads `false` for
    // the whole of a run and § 5's `503` half arrives with the control socket
    // that can ask (ADR 0078 § 6).
    // The *process's* bit, because this command is the process: an application
    // reads the same one through `Core\Server::isDraining()`, which has no
    // handle to have been given (`nvs_runtime::drain`).
    let draining = nvs_server::Draining::process();
    let handler = Rc::new({
        let compiler = Rc::clone(&compiler);
        let table = Rc::clone(&table);
        let draining = draining.clone();
        move |request: Request<Incoming>, origin: Origin| {
            // Ahead of the table, because a verb outside `Core\Http\Method`'s
            // eight names no application on this server rather than none at
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
            // § 4 step 2's remainder, taken before the branch below moves the
            // rest of the selection: it is the path the application is written
            // against, and the prefix it was deployed under is not its business.
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
            let program: Program = match compiler.resolve(&file.to_string_lossy()) {
                Ok(program) => program,
                // Reachable now that step 4 can name a file the boot compile
                // never saw: under `dispatch = "path"` a `.nvs` under the mount
                // root compiles on the request that first asks for it. It is a
                // failing program rather than a panic because a handler answers
                // with a reply and not with a `Result`: ADR 0006's failure is a
                // value, and the accept loop turns one into this request's
                // `500`.
                Err(message) => Box::new(move |ctx: &mut Ctx, _args| {
                    ctx.set_pending(message);
                    Value::null()
                }),
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
            // reading: ADR 0097 § 6's walk ran on the connection, before the
            // ceiling and before this closure, because the answer decides
            // policy on responses no handler ever sees. Both of its answers
            // land here together — `Inbound::set_peer` owns why they are one
            // call — and `Core\Request::clientIp()` and `::scheme()` are what
            // read them back.
            inbound.set_peer(origin.client(), origin.scheme());
            // Split only here: everything above reads the request whole, and
            // the body is the one part of it that does not go where the rest
            // does.
            let (head, incoming) = request.into_parts();
            // And the body, which crosses as `nvs_runtime::RequestBody` and not
            // as bytes — ADR 0105 § 5, and that trait's own docs are the
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
    // accepts is a child of it (ADR 0072 § 1), and `serve_on_this_core` refuses
    // to run anywhere else. Its own context writes nothing — a connection's
    // bytes are its request's isolate's, captured and handed back as data (ADR
    // 0088 § 3) — so `OutputSink::Sink` is what it holds rather than stdout.
    let mut sched = nvs_host::Scheduler::new();
    let stopped = Rc::new(Cell::new(false));
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
        let stopped = Rc::clone(&stopped);
        let draining = draining.clone();
        move |_ctx| {
            // `ControlFlow::Continue` forever: a development server runs until
            // the process is stopped, and ADR 0097 § 5's drain is the slice that
            // gives this command a control socket to be asked by.
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
                eprintln!("error: the accept loop stopped: {error}");
                stopped.set(true);
            }
        }
    });

    // The reactor is what a parked coroutine is woken by, and every connection
    // parks; the resolver is installed for the whole run so that a served
    // program's own `spawn script` reaches the same compiler and the same cache
    // this handler does.
    let reactor = match nvs_host::Reactor::new() {
        Ok(reactor) => reactor,
        Err(error) => {
            eprintln!("error: could not start the reactor: {error}");
            return ExitCode::FAILURE;
        }
    };
    let installed = nvs_host::reactor::install(reactor);
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
            match nvs_host::run_until_idle(&mut sched) {
                Ok(report) if report.parked > 0 => {}
                Ok(_) => return Ok(()),
                Err(error) => return Err(error),
            }
        }
    });
    drop(installed);
    if let Err(error) = ran {
        eprintln!("error: the scheduler stopped: {error}");
        return ExitCode::FAILURE;
    }
    if stopped.get() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// ADR 0097 § 4's one row for a tree that mounts nothing: the file named on the
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
/// sentence rather than a [`nvs_diagnostics::Diagnostic`], for [`address`]'s
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

/// The one address this core binds: the file's first entry, then whichever flag
/// had the last word.
///
/// # Errors
///
/// A `--listen` that is not a literal address, and a Unix-domain entry there is
/// no listener for yet — the module doc owns both. The refusal is a sentence
/// rather than a [`nvs_diagnostics::Diagnostic`] when it came from a flag,
/// because a command line has no file and no span to point into.
fn address(
    configured: &[Listen],
    listen: Option<&str>,
    port: Option<u16>,
) -> Result<SocketAddr, String> {
    if let Some(written) = listen {
        return written.parse::<SocketAddr>().map_err(|_| {
            format!(
                "`--listen {written}` is not an address and a port; write `127.0.0.1:8000` or \
                 `[::1]:8000`"
            )
        });
    }
    // `listen_on` never answers with an empty list — an empty array is its own
    // refusal — so the first entry is the configured one.
    let first = configured
        .first()
        .ok_or_else(|| "`[server] listen` named no address".to_owned())?;
    match (first, port) {
        (Listen::Tcp(addr), None) => Ok(*addr),
        // The flag is the last word for its own key alone: a port written here
        // keeps the host the file chose, so `--port` over a `0.0.0.0:80` does
        // not quietly narrow the deployment to loopback.
        (Listen::Tcp(addr), Some(port)) => Ok(SocketAddr::new(addr.ip(), port)),
        // And over a Unix entry it names a whole address, since there is no
        // host in one to keep. `127.0.0.1` because that is § 5's own default
        // and the one a development machine means.
        (Listen::Unix(_), Some(port)) => {
            Ok(SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port)))
        }
        (Listen::Unix(path), None) => Err(format!(
            "`[server] listen` asks for the Unix-domain socket `{}`, and this server accepts on \
             TCP alone today; write `--listen 127.0.0.1:8000` or a `host:port` entry",
            path.display()
        )),
    }
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
    use super::{Listen, SocketAddr, address};
    use std::path::PathBuf;

    fn tcp(written: &str) -> Listen {
        Listen::Tcp(written.parse::<SocketAddr>().expect("a literal address"))
    }

    /// ADR 0097 § 5's own sentence: the flag overrides the file. Asserted
    /// against a configured entry that is nothing like it, so a reading that
    /// merged the two rather than replacing would fail here.
    #[test]
    fn a_listen_flag_is_the_last_word_over_the_configured_address() {
        let configured = vec![tcp("0.0.0.0:80")];
        assert_eq!(
            address(&configured, Some("127.0.0.1:9001"), None).expect("a literal address"),
            "127.0.0.1:9001".parse::<SocketAddr>().expect("parses")
        );
        assert!(address(&configured, Some("localhost:9001"), None).is_err());
    }

    /// `--port` is the last word for the port and for nothing else, which is
    /// the half a flag replacing the whole address would get wrong: a
    /// deployment listening on every interface still is one after it.
    #[test]
    fn a_port_flag_keeps_the_host_the_file_chose() {
        let bound = address(&[tcp("0.0.0.0:80")], None, Some(9001)).expect("an address");
        assert_eq!(bound, "0.0.0.0:9001".parse::<SocketAddr>().expect("parses"));
        assert_eq!(
            address(&[tcp("127.0.0.1:8000")], None, None).expect("an address"),
            "127.0.0.1:8000".parse::<SocketAddr>().expect("parses")
        );
    }

    /// The refusal that moves the day there is a Unix listener, and the flag
    /// that gets past it in the meantime — both here, because a configuration a
    /// proxy should prefer must not simply fail to start with nothing to try.
    #[test]
    fn a_unix_entry_is_refused_until_there_is_a_listener_for_one() {
        let configured = vec![Listen::Unix(PathBuf::from("/run/nvs.sock"))];
        let refusal = address(&configured, None, None).expect_err("a socket was bound");
        assert!(refusal.contains("/run/nvs.sock"), "unhelpful: {refusal}");
        assert_eq!(
            address(&configured, None, Some(8080)).expect("the flag is the last word"),
            "127.0.0.1:8080".parse::<SocketAddr>().expect("parses")
        );
    }
}
