//! `nvs serve`: one core, one listening socket, and every request running one
//! entry file as [ADR 0006]'s isolate.
//!
//! [`nvs_server::serve::serve_on_this_core`] is the loop and
//! [ADR 0138](../../../docs/adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md)
//! is what drives a connection on it; what this module owns is the three things
//! only the binary can supply — the socket the loop accepts on, the clock it
//! holds a connection to, and the handler that says which isolate a request is.
//!
//! # Decision: the entry file is named on the command line, and it is a table of one
//!
//! [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
//! § 2 is the server's governing rule — a request *selects* an entry point from
//! a set enumerated before it arrived and may never construct one — and § 4's
//! mount table is how that set is normally built. This command is *told* its
//! entry, so [`one_mount`] is that table with one row in it: mounted at `/`, with
//! the file's own directory as the mount root, and compiled **before the socket
//! is bound**. Every request then goes through
//! [`nvs_server::Table::select`]'s five steps, so what selects a program here is
//! the same code a deployment's fleet of mounts selects through — a set of one is
//! still a set, and § 2's rule is about the enumeration and not about its size.
//!
//! Expanding `[[server.mount]]` against the disk
//! ([`nvs_config::mount::expand`]) belongs to the deployment path that has no
//! `<file>` argument to be told; `[server] static` and `dispatch` are read off
//! the tree here as written, which is
//! [`nvs_server::Table::from_config`]'s fail-closed reading of them. A step 3
//! selection is answered by the mount's entry until the slice that sends a
//! file's bytes lands — the handler's own comment is why that is the closed
//! answer rather than a status.
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
//! one compiled unit for the entry, held for the life of the process and shared
//! by every request that runs it ([ADR 0006]'s "shares immutable compiled code",
//! which is [`crate::script`]'s cache and nothing else), plus whatever the accept
//! loop holds per connection in flight. Nothing accumulates per request answered.
//!
//! [ADR 0006]: ../../../docs/adr/0006-isolated-script-execution.md

use std::cell::Cell;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use nvs_config::mount::Mounted;
use nvs_config::server::{Listen, listen_on, waits_for};
use nvs_config::tree::Config;
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_host::{Isolate, NvsListener, Output};
use nvs_runtime::script::{Program, Resolver as _};
use nvs_runtime::{Ctx, OutputSink, TaskRoot, Value};
use nvs_server::{OnDisk, Reply, Table, What};

use crate::script::Compiler;

/// `nvs serve <file>` — resolve the tree, compile the entry, bind the socket and
/// run the accept loop until this process is stopped.
///
/// The order is the whole of § 2's rule: the configuration is refused before
/// anything is compiled, the entry is compiled before anything is bound, and the
/// socket exists only once there is something for it to answer with.
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
    let snapshot = match crate::config::boot_snapshot(config, path, &mut sources) {
        Ok(snapshot) => snapshot,
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
    // and the origins map is empty for the reason `nvs run`'s `queue_for` call
    // has an empty one: what it adds is the line a refusal points at.
    let origins = BTreeMap::new();
    let waits = match waits_for(&snapshot.config, &origins) {
        Ok(waits) => waits,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    let configured = match listen_on(&snapshot.config, &origins) {
        Ok(entries) => entries,
        Err(diagnostic) => return report(diagnostic, &sources),
    };
    let addr = match address(&configured, listen, port) {
        Ok(addr) => addr,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };

    // § 4's table, of one: the file named on the command line, at `/`, with the
    // directory it sits in as the mount root. Canonical on both sides because
    // that is what § 4 steps 3 and 4 compare a resolved remainder against, and
    // because the compile below is keyed by the same string every request then
    // resolves.
    let table = match one_mount(path, &snapshot.config) {
        Ok(table) => table,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    let entry = table.mounts()[0].entry.to_string_lossy().into_owned();

    // § 2, kept before the socket exists: the one path this server can execute
    // is compiled now, so a program that does not compile is a start that fails
    // rather than a request that does. The front end renders its own
    // diagnostics (`script`'s module doc), so the message here is the summary.
    let compiler = Rc::new(Compiler::default());
    if let Err(message) = compiler.resolve(&entry) {
        eprintln!("error: {message}");
        return ExitCode::FAILURE;
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

    // Every request goes through § 4's five steps and then runs what they chose
    // as ADR 0006's isolate — the same type `spawn script` runs, and deliberately
    // not a second isolation path (ADR 0097's crate doc). With one mount at `/`
    // and `dispatch = "entry"` that is step 5 every time and the resolve is a
    // cache hit on the unit compiled above, so what it costs per request is one
    // `Program` over shared code.
    let handler = Rc::new({
        let compiler = Rc::clone(&compiler);
        let table = Rc::clone(&table);
        move |request| {
            let Some(selected) = table.select(&request, &OnDisk) else {
                // § 4 step 1's third arrow. There is no mount for it and so
                // nothing to run: not a program's `404` but the table's.
                return Reply::not_found();
            };
            let file = match selected.what {
                What::Run(file) => file,
                // Step 3 chose a file to *send*, and the slice that sends bytes
                // has not landed. Running the mount's entry is exactly what
                // `static = false` selects for the same request, so a tree that
                // turned the switch on early gets the answer it would have had
                // with it off rather than a status nothing promised.
                What::Static(_) => selected.mount.entry.clone(),
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
            Reply::Run(Isolate::new(program, Value::null(), Output::Capture))
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
        move |_ctx| {
            // `ControlFlow::Continue` forever: a development server runs until
            // the process is stopped, and ADR 0097 § 5's drain is the slice that
            // gives this command a control socket to be asked by.
            let served = nvs_server::serve_on_this_core(&mut listener, &handler, waits, || {
                ControlFlow::Continue(())
            });
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

/// ADR 0097 § 4's table for this command: the file named on the command line,
/// mounted at `/`, with the directory it sits in as the mount root.
///
/// One row rather than [`nvs_config::mount::expand`]'s expansion of
/// `[[server.mount]]`, because this command was *told* which entry to serve and
/// § 2's rule is about the set being enumerated before a request arrives rather
/// than about how many rows it has. The module doc's § *Decision* is the whole of
/// why, and the deployment that has no `<file>` to be told is what calls `expand`.
///
/// Canonical on both sides: § 4 steps 3 and 4 compare a resolved remainder
/// against the mount root, and a `starts_with` between a canonical path and a
/// written one answers `false` for every file in the tree. `[server] static` and
/// `dispatch` come off the tree as written, which is
/// [`nvs_server::Table::from_config`]'s fail-closed reading.
///
/// # Errors
///
/// A path that cannot be canonicalized — absent, or unreadable — and one that is
/// a filesystem root and so has no directory to be a mount root. Both are a
/// sentence rather than a [`nvs_diagnostics::Diagnostic`], for [`address`]'s
/// reason: the value came from a command line and there is no span to point into.
fn one_mount(path: &Path, config: &Config) -> Result<Rc<Table>, String> {
    let entry = nvs_config::trust::canonical(path)
        .map_err(|why| format!("`{}` cannot be served: {why}", path.display()))?;
    let root = entry
        .parent()
        .ok_or_else(|| format!("`{}` is not a file in a directory", entry.display()))?
        .to_path_buf();
    Ok(Rc::new(Table::from_config(
        vec![Mounted {
            prefix: "/".to_string(),
            host: None,
            entry,
            root,
            origin: None,
            captures: Vec::new(),
        }],
        config,
    )))
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
