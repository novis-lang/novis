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
//! [`Process::pending`] then remembers on the process's behalf, each key with
//! the value in force and the value written. It is also where a service manager
//! is told a reload is happening and then that it is over
//! ([`crate::service::State`]), for the same reason: one function, so one pair
//! of transitions however the reload was asked for. One lock covers the whole
//! of it, so two reloads never interleave two snapshots.
//!
//! **The server checks its own configuration files** —
//! `rule:config/the-config-is-an-immutable-snapshot`'s last paragraph. [`check`]
//! starts one thread that, every [`CHECK`], takes the stamp (`mtime` and size)
//! of every path the serving tree was read from or probed
//! ([`Snapshot::files`] and [`Snapshot::probed`]). A stamp that moved and then
//! holds for one more check is a saved file, and [`Process::noticed`] resolves
//! the tree again and publishes it through the same steps a pushed reload runs.
//! A tree equal to the one serving publishes nothing, so a file saved with the
//! same content does not start a new generation. A refusal is logged once per
//! rendered diagnostic, so a broken file is reported when it is saved and not
//! every two seconds after. A tree whose roots are the shipped defaults read no
//! file, and this check has nothing to stat.
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
//! holding the roots as written, the pending restart keys with their two values,
//! and the last refusal the check logged. A reload holds one snapshot's worth of
//! tree while it resolves and drops the previous one when the new one is
//! published; requests already running keep theirs, which is
//! `rule:config/the-config-is-an-immutable-snapshot`'s own promise and bounds
//! that hold at O(in-flight). The check is one thread per process, asleep
//! between passes, and one `stat` per configuration file per [`CHECK`]. No
//! request makes one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, SystemTime};

use nvs_config::resolve::Origin;
use nvs_config::snapshot::{Current, Snapshot, value_at};
use nvs_config::{Apply, DIRECTIVES};
use nvs_diagnostics::{Diagnostic, Renderer, SourceMap};
use nvs_render::{Level, Node, Record, Rendered, Scalar};
use nvs_runtime::LogWriter;
use nvs_server::control::{Controlled, Pending, Report};
use nvs_server::{Admission, Ceiling, Draining};

use crate::script::Compiler;
use crate::service::{Notify, State};

/// The process `nvs serve` installed, for the one caller that is handed
/// nothing: a service manager's control handler (`crate::dispatch`).
static PROCESS: OnceLock<Arc<Process>> = OnceLock::new();

/// Keeps `process` as this process's, for [`installed`].
///
/// Installed once — a second call keeps the first, because `nvs serve` runs
/// once per process.
pub(crate) fn install(process: Arc<Process>) {
    drop(PROCESS.set(process));
}

/// What [`install`] left, or `None` before the boot got that far.
#[cfg(windows)]
pub(crate) fn installed() -> Option<Arc<Process>> {
    PROCESS.get().cloned()
}

/// What `nvs serve` supplies to its control endpoint.
pub(crate) struct Process {
    /// The published tree — what the accept loop hands a request at its start,
    /// and what a reload replaces whole.
    current: Arc<Current>,
    /// `--config` as it was written, so a reload resolves the roots this boot
    /// resolved rather than whatever the working directory now holds.
    roots: Vec<PathBuf>,
    /// The entry file the command named, which is what selects the `[[app]]`
    /// blocks the snapshot folds — or `None` where it named none and the
    /// snapshot folds no block.
    entry: Option<PathBuf>,
    /// The fleet's one unit cache, for the count a reload reports and the
    /// re-key it owes.
    compiler: Arc<Compiler>,
    /// The valve every request is admitted through, which is where the in-flight
    /// count already lives, and whose ceiling a reload moves.
    admission: Arc<Admission>,
    /// This process's drain bit.
    draining: Draining,
    /// Whatever started this process, told `RELOADING=1` and `READY=1` around
    /// the reload below — the transitions `rule:packaging/the-generated-unit-is-hardened`'s
    /// `Type=notify` unit is owed, from the one function every spelling of a
    /// reload ends in.
    notify: Notify,
    /// Held for the whole of a reload, pushed or noticed, so the two publish
    /// one at a time.
    reloading: Mutex<()>,
    /// The `Boot` keys the last reload reported and left unapplied, each with
    /// its two values. Empty until one has happened, which is the honest
    /// answer: a process that has never reloaded has ignored nothing.
    pending: Mutex<Vec<Pending>>,
    /// The refusal [`Process::noticed`] last logged, rendered. A publish
    /// clears it.
    refused: Mutex<Option<String>>,
}

/// A tree resolved from the files as they stand now, with what a refusal of
/// it is rendered against.
struct Tree {
    next: Arc<Snapshot>,
    origins: BTreeMap<String, Origin>,
    sources: SourceMap,
}

impl Process {
    /// The process serving `current`, resolved from `roots` and `entry`.
    pub(crate) fn new(
        current: Arc<Current>,
        roots: Vec<PathBuf>,
        entry: Option<PathBuf>,
        compiler: Arc<Compiler>,
        admission: Arc<Admission>,
        draining: Draining,
        notify: Notify,
    ) -> Self {
        Self {
            current,
            roots,
            entry,
            compiler,
            admission,
            draining,
            notify,
            reloading: Mutex::new(()),
            pending: Mutex::new(Vec::new()),
            refused: Mutex::new(None),
        }
    }

    /// The tree the roots resolve to now.
    ///
    /// # Errors
    ///
    /// The tree does not resolve, rendered.
    fn resolved(&self) -> Result<Tree, String> {
        let mut sources = SourceMap::new();
        let (next, origins) = crate::config::boot_origins(
            &self.roots,
            self.entry.as_deref(),
            &mut sources,
            crate::config::Init::Never,
        )
        .map_err(|refusal| rendered(&refusal, &sources))?;
        Ok(Tree {
            next,
            origins,
            sources,
        })
    }

    /// The reload itself, from a resolved tree, which is everything except
    /// saying that one is happening.
    ///
    /// # Errors
    ///
    /// The publish refused the tree. This process is still serving the tree it
    /// had, which is what makes the `READY=1` its caller sends next true.
    fn published(&self, tree: Tree) -> Result<Report, String> {
        let Tree {
            next,
            origins,
            sources,
        } = tree;
        // Asked of the incoming tree before the publish, so a `[limits]` memory
        // setting the admission arithmetic cannot read refuses the reload the
        // way it refuses a boot, and leaves the running tree serving.
        nvs_config::server::capacity_for(&next.config, &origins)
            .map_err(|refusal| rendered(&refusal, &sources))?;
        // The publish takes a snapshot by value and this one was built for it,
        // so the clone is the branch that never runs: a tree just resolved is
        // held by nobody else.
        let next = Arc::try_unwrap(next).unwrap_or_else(|held| Snapshot::clone(&held));
        // The written value of every restart key, taken before the publish
        // carries the running value back over it.
        let written: Vec<(&'static str, String)> = DIRECTIVES
            .iter()
            .filter(|row| row.apply == Apply::Boot)
            .map(|row| (row.key, shown(value_at(&next.table, row.key))))
            .collect();
        let report = nvs_config::control::reload(&self.current, next, self.compiler.held())
            .map_err(|refusal| rendered(&refusal, &sources))?;
        // After the publish, from the tree that is now serving: the report's
        // `invalidated` count was derived from the same comparison, so doing
        // this first would leave a window where the two disagree.
        let serving = self.current.load();
        let pending: Vec<Pending> = report
            .ignored
            .iter()
            .map(|key| Pending {
                key,
                running: shown(value_at(&serving.table, key)),
                written: written
                    .iter()
                    .find(|(row, _)| row == key)
                    .map_or_else(|| shown(None), |(_, value)| value.clone()),
            })
            .collect();
        // Logged once for each written value: a key already pending with the
        // same written value was logged by the reload that first saw it.
        let seen = std::mem::replace(&mut *lock(&self.pending), pending.clone());
        let config = nvs_config::Request::new(Arc::clone(&serving));
        let mut writer = LogWriter::resolve(Some(&config));
        for fresh in pending.iter().filter(|entry| !seen.contains(entry)) {
            drop(writer.write(&restart_pending(fresh)));
        }
        *lock(&self.refused) = None;
        self.compiler
            .rekey(nvs_config::cache::env_hash(&serving.config));
        self.compiler.reconfigure(&serving.config);
        // `rule:http-server/admission-is-arithmetic-not-a-number` over the tree
        // now serving: `limits.memory` reloads, and `[server] max_in_flight` is
        // whatever the publish carried. The tree passed the same arithmetic
        // above, so the error arm keeps the ceiling it had and never runs.
        if let Ok(capacity) = nvs_config::server::capacity_for(&serving.config, &origins) {
            let ceiling = Ceiling::of(&capacity);
            if let Some(note) = ceiling.clamp_note() {
                eprintln!("note: {note}");
            }
            self.admission.resize(&ceiling);
        }
        Ok(report)
    }

    /// The configuration check's reload: `changed` are the paths whose stamps
    /// moved and then held.
    ///
    /// A tree equal to the one serving publishes nothing, and clears the
    /// pending restart keys: every one of them is back at its running value.
    /// A refusal is logged
    /// unless it is the same rendered diagnostic this check logged last, which
    /// is a file saved again with the same fault.
    fn noticed(&self, changed: &[PathBuf]) {
        let _one = lock(&self.reloading);
        if self.draining.is_draining() {
            return;
        }
        let outcome = match self.resolved() {
            // Every restart key the files hold is the running value again,
            // so nothing is pending any more.
            Ok(tree) if same_tree(&tree.next, &self.current.load()) => {
                lock(&self.pending).clear();
                return;
            }
            Ok(tree) => {
                self.notify.state(State::Reloading);
                let outcome = self.published(tree);
                self.notify.state(State::Ready);
                outcome
            }
            Err(refusal) => Err(refusal),
        };
        if let Err(refusal) = &outcome {
            let mut last = lock(&self.refused);
            if last.as_deref() == Some(refusal.as_str()) {
                return;
            }
            *last = Some(refusal.clone());
        }
        self.logged(&outcome, changed);
    }

    /// Every path the serving tree was read from or probed, with its stamp now.
    fn stamps(&self) -> Vec<(PathBuf, Stamp)> {
        let serving = self.current.load();
        serving
            .files
            .iter()
            .chain(&serving.probed)
            .map(|path| (path.clone(), stamp(path)))
            .collect()
    }

    /// Writes what the reload did where `[log] target` says —
    /// `rule:config/one-local-control-socket`'s "every reload is written to
    /// `Core\Log` with its outcome".
    ///
    /// **Written from the tree that is now serving**, which is the new one after
    /// a publish and the old one after a refusal: either way it is the tree this
    /// process is running under by the time the line is written, so an operator
    /// who moved `[log] target` in the reload that just landed finds the record
    /// at the new target rather than at the one it replaced.
    ///
    /// The directives behind that sink are read by
    /// [`nvs_runtime::LogWriter`] and not here, so this thread is not a second
    /// reader of them — its doc comment owns why, and owns where a record goes
    /// when the tree names no target at all.
    ///
    /// The write's own failure is swallowed, per `rule:errors/engine-floor`: a
    /// full disk under the log target is not a reason to fail the reload that
    /// already happened, and the answer the operator is holding says what it did
    /// regardless.
    fn logged(&self, outcome: &Result<Report, String>, changed: &[PathBuf]) {
        let config = nvs_config::Request::new(self.current.load());
        drop(LogWriter::resolve(Some(&config)).write(&record(outcome, changed)));
    }
}

impl Controlled for Process {
    fn reload(&self) -> Result<Report, String> {
        let _one = lock(&self.reloading);
        self.notify.state(State::Reloading);
        let outcome = self.resolved().and_then(|tree| self.published(tree));
        // `READY=1` whatever that outcome was, and `State::Reloading` owns why:
        // a refused reload leaves this process serving the tree it already had,
        // so a manager left in `reloading` over one would be reporting a state
        // this process is not in.
        self.notify.state(State::Ready);
        // After the state, and from the tree now in force: a record written
        // before the publish settled could name a target the reload replaced.
        self.logged(&outcome, &[]);
        outcome
    }

    fn snapshot(&self) -> Arc<Snapshot> {
        self.current.load()
    }

    fn unapplied(&self) -> Vec<&'static str> {
        lock(&self.pending).iter().map(|entry| entry.key).collect()
    }

    fn pending(&self) -> Vec<Pending> {
        lock(&self.pending).clone()
    }

    fn in_flight(&self) -> usize {
        self.admission.in_flight()
    }

    fn draining(&self) -> bool {
        self.draining.is_draining()
    }
}

/// How often the configuration check takes every stamp. Fixed and not a
/// directive: no operator needs to tune how quickly a saved file is noticed.
pub(crate) const CHECK: Duration = Duration::from_secs(2);

/// A path's modification time and size, or `None` where it is absent.
type Stamp = Option<(SystemTime, u64)>;

/// The stamp of `path` now.
fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Starts the configuration check: one thread that, every [`CHECK`], takes
/// the stamp of every path the serving tree read or probed, and hands a change
/// that held for one more check to [`Process::noticed`].
///
/// The thread is detached, like the control endpoint's: it has no end of its
/// own, and the process ending is what stops it. A pass that panics is caught,
/// and the next pass runs as usual. A thread that cannot be started is
/// reported once, and then a saved file waits for `nvs ctl reload`.
pub(crate) fn check(process: &Arc<Process>) {
    let process = Arc::clone(process);
    // Taken here, before any listener exists, and not by the thread: a thread
    // that first runs after a file was saved would take the saved stamps as
    // the boot's, and never notice that save.
    let mut seen = process.stamps();
    let spawned = std::thread::Builder::new()
        .name("nvs-config-check".to_owned())
        .spawn(move || {
            let mut moved: Option<Vec<(PathBuf, Stamp)>> = None;
            loop {
                std::thread::sleep(CHECK);
                let now = process.stamps();
                if now == seen {
                    moved = None;
                    continue;
                }
                // A file still being written moves again before the next pass,
                // so only a stamp that held for a whole pass is read.
                if moved.as_ref() != Some(&now) {
                    moved = Some(now);
                    continue;
                }
                let changed: Vec<PathBuf> = now
                    .iter()
                    .filter(|entry| !seen.contains(entry))
                    .map(|(path, _)| path.clone())
                    .collect();
                moved = None;
                // The stamps taken before the tree is read again: a save that
                // lands while it is read moves one of them, and the next pass
                // reads the tree once more.
                seen = now;
                drop(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || {
                        process.noticed(&changed);
                    },
                )));
            }
        });
    if let Err(error) = spawned {
        eprintln!("warning: the configuration files will not be checked for changes: {error}");
    }
}

/// Whether `next` is the tree `serving` already is: the same keys and values,
/// each written in the same file, from the same files. A publish of it would
/// change nothing but the generation, which empties the process cache.
fn same_tree(next: &Snapshot, serving: &Snapshot) -> bool {
    next.table == serving.table
        && next.roster == serving.roster
        && next.blocks == serving.blocks
        && next.files == serving.files
        && next.probed == serving.probed
        && next.secrets == serving.secrets
        && next.origins.len() == serving.origins.len()
        && next
            .origins
            .iter()
            .zip(&serving.origins)
            .all(|((key, origin), (was, then))| key == was && origin.path == then.path)
}

/// A value as TOML, or `not written`.
fn shown(value: Option<&toml::Value>) -> String {
    value.map_or_else(|| "not written".to_string(), ToString::to_string)
}

/// `mutex`, taken whether or not a holder panicked: every value behind these
/// locks is whole after each statement that writes it.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One restart key the files changed, as the record the reload that first saw
/// its written value writes: `Warn`, with the key and both values in fields.
fn restart_pending(entry: &Pending) -> Record {
    let mut record = Record::at(Level::Warn);
    record.envelope.message = Some(Rendered::new("configuration restart pending"));
    record.envelope.fields = vec![
        ("key".to_string(), text(entry.key)),
        ("running".to_string(), text(&entry.running)),
        ("written".to_string(), text(&entry.written)),
    ];
    record
}

/// The reload's outcome as one record — what [`Process::logged`] writes.
///
/// **The message is a fixed line either way and the outcome is in the fields.**
/// A deployment then greps one string for every reload this process performed
/// and reads the answer beside it, instead of matching a sentence that varies
/// with what changed; and the fields are already the shape
/// `rule:errors/log-fields` keeps a compile-time schema possible for.
///
/// `ignored` names its keys one at a time rather than counting them, which is
/// `rule:config/a-reload-names-what-it-could-not-apply` in the log as well as in
/// the answer: a deployment that silently ignores a changed listen address
/// believes it applied a change it did not.
///
/// A refusal's rendered diagnostic is a field for the same reason — it is many
/// lines with a span in it, and a message is a line.
///
/// A reload the configuration check started also names the files whose stamps
/// moved, in a `changed` field. A pushed reload has none.
fn record(outcome: &Result<Report, String>, changed: &[PathBuf]) -> Record {
    match outcome {
        Ok(report) => {
            let mut record = Record::at(Level::Info);
            record.envelope.message = Some(Rendered::new("configuration reloaded"));
            if !changed.is_empty() {
                record.envelope.fields.push((
                    "changed".to_string(),
                    Node::Sequence(
                        changed
                            .iter()
                            .map(|path| text(&path.display().to_string()))
                            .collect(),
                    ),
                ));
            }
            record.envelope.fields.extend([
                (
                    "applied".to_string(),
                    names(report.applied.iter().map(String::as_str)),
                ),
                ("ignored".to_string(), names(report.ignored.iter().copied())),
                (
                    "invalidated".to_string(),
                    Node::Scalar(Scalar::Uint(
                        u64::try_from(report.invalidated).unwrap_or(u64::MAX),
                    )),
                ),
            ]);
            record
        }
        Err(refusal) => {
            let mut record = Record::at(Level::Error);
            record.envelope.message = Some(Rendered::new(
                "configuration reload refused; the running configuration is unchanged",
            ));
            record.envelope.fields = vec![("refusal".to_string(), text(refusal))];
            record
        }
    }
}

/// One string as a record node, with `rule:tooling/terminal-output-is-a-sink`'s
/// substitution applied by [`Rendered`] and the value's own length beside it.
fn text(value: &str) -> Node {
    Node::Scalar(Scalar::Str {
        text: Rendered::new(value),
        bytes: value.len(),
    })
}

/// A list of directive names as a node — empty where the reload changed or
/// ignored nothing, which is the honest answer and not an absent field.
fn names<'a>(keys: impl Iterator<Item = &'a str>) -> Node {
    Node::Sequence(keys.map(text).collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:config/one-local-control-socket`'s "every reload is written to
    /// `Core\Log` with its outcome": a reload that landed is one `Info` record
    /// naming the three answers its report carries, each in a field.
    #[test]
    fn a_reload_that_landed_is_one_info_record_naming_what_it_did() {
        let written = record(
            &Ok(Report {
                applied: vec!["server.workers".to_string()],
                ignored: vec!["server.listen"],
                invalidated: 12,
            }),
            &[],
        );
        assert_eq!(written.envelope.level, Level::Info);
        let named: Vec<&str> = written
            .envelope
            .fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(named, ["applied", "ignored", "invalidated"]);
        assert_eq!(
            written.envelope.fields[1].1,
            names(["server.listen"].into_iter()),
            "a `Boot` key left unapplied was counted instead of named"
        );
    }

    /// A refused reload says so at `Error` and carries the diagnostic in a
    /// field, so the message is the same greppable line as the one above.
    #[test]
    fn a_refused_reload_is_one_error_record_carrying_the_diagnostic_in_a_field() {
        let refusal = "E0601: the tree does not deserialize\n  --> nvs.toml:4:1";
        let written = record(&Err(refusal.to_string()), &[]);
        assert_eq!(written.envelope.level, Level::Error);
        assert_eq!(
            written.envelope.fields,
            vec![("refusal".to_string(), text(refusal))]
        );
    }

    /// A reload the configuration check started names the files whose stamps
    /// moved, before the three answers a pushed reload's record carries.
    #[test]
    fn a_noticed_reload_names_the_files_that_changed() {
        let written = record(
            &Ok(Report {
                applied: vec!["limits.memory".to_string()],
                ignored: Vec::new(),
                invalidated: 0,
            }),
            &[PathBuf::from("/etc/nvs/nvs.toml")],
        );
        let named: Vec<&str> = written
            .envelope
            .fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(named, ["changed", "applied", "ignored", "invalidated"]);
    }

    /// A pending restart key is one `Warn` record with the key and both
    /// values, each in a field.
    #[test]
    fn a_pending_restart_key_is_one_warn_record_with_both_values() {
        let written = restart_pending(&Pending {
            key: "server.workers",
            running: "4".to_string(),
            written: "2".to_string(),
        });
        assert_eq!(written.envelope.level, Level::Warn);
        assert_eq!(
            written.envelope.fields,
            vec![
                ("key".to_string(), text("server.workers")),
                ("running".to_string(), text("4")),
                ("written".to_string(), text("2")),
            ]
        );
    }
}
