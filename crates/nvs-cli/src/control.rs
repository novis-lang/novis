//! The running `nvs serve`, as the half
//! [`nvs_server::control::Controlled`] asks a process for.
//!
//! The control surface itself is `nvs-server`'s — which requests mean anything,
//! what each answer says, and the one thread that accepts them. What is *here*
//! is the four things only this binary holds: the roots the boot resolved and
//! can resolve again, the unit cache the fleet shares, the valve the accept loop
//! counts through, and this process's drain bit.
//!
//! **A reload is one function**, and it is
//! [`nvs_config::control::reload`]: this module resolves the tree exactly as the
//! boot did and hands the result to it, so the control socket, `systemctl
//! reload` and a service manager's `PARAMCHANGE` all end in the same publish and
//! the same `rule:config/a-reload-names-what-it-could-not-apply` report. Nothing
//! here decides what a `Boot` key does — [`nvs_config::Current::publish`] carries
//! the running value back over the incoming tree, and what it reports is what
//! [`Process::unapplied`] then remembers on the process's behalf.
//!
//! **A reload re-reads the tree; it never writes one.** The boot may have been
//! asked to create a default `nvs.toml` ([`crate::config::Init`]), and a control
//! operation that did the same thing would be a file appearing on disk because
//! somebody asked what the server was serving. So the resolve here is always
//! [`Init::Never`](crate::config::Init::Never).
//!
//! **The unit cache is re-keyed and not merely counted.** `[[extension]]` is the
//! configuration's whole contribution to
//! `rule:config/the-extension-set-is-in-every-unit-key`'s environment digest and
//! it is a reloadable directive, so a reload that changes it makes every unit
//! this process holds a unit compiled for a different environment.
//! [`crate::script::Compiler::rekey`] is what makes the `invalidated` count the
//! report carries a fact rather than a prediction: the units are dropped and the
//! compiler is keying on the published environment before the answer is written.
//!
//! Cost, as `rule:programs/memory-priority` requires: one of these per process,
//! holding the roots as written and the last report's ignored keys, which is a
//! list of `&'static str`. A reload holds one snapshot's worth of tree while it
//! resolves and drops the previous one when the new one is published; requests
//! already running keep theirs, which is
//! `rule:config/the-config-is-an-immutable-snapshot`'s own promise and bounds
//! that hold at O(in-flight).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nvs_config::snapshot::{Current, Snapshot};
use nvs_diagnostics::{Diagnostic, Renderer, SourceMap};
use nvs_server::control::{Controlled, Report};
use nvs_server::{Admission, Draining};

use crate::script::Compiler;

/// What `nvs serve` supplies to its control endpoint.
pub(crate) struct Process {
    /// The published tree — what the accept loop hands a request at its start,
    /// and what a reload replaces whole.
    current: Arc<Current>,
    /// `--config` as it was written, so a reload resolves the roots this boot
    /// resolved rather than whatever the working directory now holds.
    roots: Vec<PathBuf>,
    /// The entry file the command named, which is what selects the `[[app]]`
    /// blocks the snapshot folds.
    entry: PathBuf,
    /// The fleet's one unit cache, for the count a reload reports and the
    /// re-key it owes.
    compiler: Arc<Compiler>,
    /// The valve every request is admitted through, which is where the in-flight
    /// count already lives.
    admission: Arc<Admission>,
    /// This process's drain bit.
    draining: Draining,
    /// The `Boot` keys the last reload reported and left unapplied. Empty until
    /// one has happened, which is the honest answer: a process that has never
    /// reloaded has ignored nothing.
    unapplied: Mutex<Vec<&'static str>>,
}

impl Process {
    /// The process serving `current`, resolved from `roots` and `entry`.
    pub(crate) fn new(
        current: Arc<Current>,
        roots: Vec<PathBuf>,
        entry: PathBuf,
        compiler: Arc<Compiler>,
        admission: Arc<Admission>,
        draining: Draining,
    ) -> Self {
        Self {
            current,
            roots,
            entry,
            compiler,
            admission,
            draining,
            unapplied: Mutex::new(Vec::new()),
        }
    }
}

impl Controlled for Process {
    fn reload(&self) -> Result<Report, String> {
        let mut sources = SourceMap::new();
        let next = crate::config::boot_snapshot(
            &self.roots,
            &self.entry,
            &mut sources,
            crate::config::Init::Never,
        )
        .map_err(|refusal| rendered(&refusal, &sources))?;
        // The publish takes a snapshot by value and this one was built for it,
        // so the clone is the branch that never runs: a tree just resolved is
        // held by nobody else.
        let next = Arc::try_unwrap(next).unwrap_or_else(|held| Snapshot::clone(&held));
        let report = nvs_config::control::reload(&self.current, next, self.compiler.held())
            .map_err(|refusal| rendered(&refusal, &sources))?;
        *self
            .unapplied
            .lock()
            .expect("the unapplied list is only ever taken here") = report.ignored.clone();
        // After the publish, from the tree that is now serving: the report's
        // `invalidated` count was derived from the same comparison, so doing
        // this first would leave a window where the two disagree.
        self.compiler
            .rekey(nvs_config::cache::env_hash(&self.current.load().config));
        Ok(report)
    }

    fn snapshot(&self) -> Arc<Snapshot> {
        self.current.load()
    }

    fn unapplied(&self) -> Vec<&'static str> {
        self.unapplied
            .lock()
            .expect("the unapplied list is only ever taken here")
            .clone()
    }

    fn in_flight(&self) -> usize {
        self.admission.in_flight()
    }

    fn draining(&self) -> bool {
        self.draining.is_draining()
    }
}

/// A refusal as the text that crosses the [`Controlled`] seam.
///
/// Rendered rather than summarised, because a malformed `nvs.toml` is a
/// diagnostic with a span into a file and the operator reading the answer is the
/// one who just edited that line. Colour is off: the answer is an HTTP body,
/// and `nvs ctl` prints it to whatever the operator's terminal is.
fn rendered(refusal: &Diagnostic, sources: &SourceMap) -> String {
    let mut out = Vec::new();
    drop(
        Renderer::new()
            .with_color(false)
            .render(refusal, sources, &mut out),
    );
    String::from_utf8_lossy(&out).into_owned()
}
