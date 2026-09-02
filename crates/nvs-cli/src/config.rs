//! `nvs config` — [ADR 0103] § 9's offline audit of the configuration tree, and
//! the reader every caller in this binary resolves that tree through.
//!
//! § 9 puts `check` beside `dump` and `ctl config` for one stated reason: § 3's
//! later-wins precedence "is only safe while it is auditable", so the reporting
//! is part of that decision rather than tooling around it. [`check`] answers
//! whether a tree resolves at all and what it holds in summary; [`dump`] answers
//! what every key resolved to and where it was written, which is the obligation
//! § 3 attaches to letting an include override the file that pulled it in. Both
//! are offline and need no server, which is what lets a tree be validated in CI
//! before it is deployed.
//!
//! ## Why the audit does not apply § 6's ownership check
//!
//! [`LocalFiles`] reads without that check, and `nvs config check` is a second
//! caller that wants it that way — for a reason of its own rather than by
//! inheriting `nvs run`'s. § 6 asks whether a file is owned by **the account the
//! runtime runs as** and unwritable by anyone else. A machine auditing a tree
//! before deployment is not that account and usually not that host, so the check
//! run there answers a different question than the one it exists for: it refuses
//! trees the server would accept and passes trees the server will refuse. A
//! green `nvs config check` that means neither is worse than one that does not
//! claim to have looked.
//!
//! So the boundary is asserted where it can be answered — `nvs serve` and
//! `nvs ctl reload`, through `nvs_config::resolve::Disk`, on the host and as the
//! account that will serve — and `check` reports what is decidable offline:
//! syntax, unknown keys, a duplicate key within one file, an include cycle, a
//! missing include, and the precedence the tree flattens to.
//!
//! [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};

use crate::render_diagnostics;

/// The reader this binary resolves the configuration tree through: the real
/// filesystem, with ADR 0103 § 6's **ownership check not applied**.
///
/// `nvs_config::resolve::Files` exists for exactly this split — its own doc
/// says § 6's check belongs on the reader so that "a caller that has one and a
/// caller that does not are two implementations of one interface rather than a
/// flag threaded through the resolver". This is the caller that does not, and
/// it is one of two rather than a weakening of the boundary:
///
/// - § 6 defends a runtime that grants **configured capabilities to requests
///   nobody at the keyboard wrote**. `nvs serve` and `nvs ctl reload` are that
///   runtime and they use `Disk`, which checks.
/// - A `nvs run` has no such boundary to defend. The program is named on argv
///   and executed as the invoking account, and the configuration is `./nvs.toml`
///   in a working directory that same person chose. Whoever can write that file
///   is in a position to be writing the program too.
/// - And the check would refuse nearly every Windows checkout. § 6 names the
///   reason itself: Windows grants `Authenticated Users` modify rights by
///   default on a non-system drive's root and on everything inheriting from it,
///   so a repository on `D:` fails the check until an operator breaks that
///   inheritance. That is the right price for a served host and the wrong one
///   for `nvs run examples/hello.nvs`.
///
/// [`check`] reads through it for a third reason, which is this module's own
/// doc comment.
///
/// **This is not the whole answer, and the rest is Stage 4's.** Where a
/// capability check sits so that no member can route around it is the one ADR
/// slot this milestone reserved, and whether a CLI run may be granted anything
/// out of an unchecked file belongs in it. Until then nothing here grants
/// anything: `Core\Config` reads values and `[capabilities]` is enforced
/// nowhere, so the split above costs no right that is currently checked.
pub(crate) struct LocalFiles;

impl nvs_config::resolve::Files for LocalFiles {
    /// Canonicalization without the ownership check — see the type's own docs.
    /// The canonical path still comes from `nvs_config::trust::canonical`,
    /// because the resolver's cycle test compares files rather than spellings
    /// and a second canonicalizer is how a symlinked cycle gets through.
    fn trust(&self, path: &Path) -> Result<PathBuf, nvs_config::trust::Untrusted> {
        nvs_config::trust::canonical(path)
            .map_err(|err| nvs_config::trust::Untrusted::Unreadable(err.to_string()))
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        nvs_config::resolve::Disk.canonical(path)
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        nvs_config::resolve::Disk.read(path)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        nvs_config::resolve::Disk.read_bytes(path)
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        nvs_config::resolve::Disk.exposure(path)
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        nvs_config::resolve::Disk.list(dir)
    }

    fn exists(&self, path: &Path) -> bool {
        nvs_config::resolve::Disk.exists(path)
    }
}

/// The working directory, as the diagnostic a caller reports when it cannot be
/// read.
///
/// Both entry points below need it — ADR 0103 § 5 resolves a `--config` against
/// it and § 1 step 2 looks for `./nvs.toml` in it — and neither can proceed
/// without it, so the failure is one shape rather than two.
pub(crate) fn working_directory() -> Result<PathBuf, Diagnostic> {
    std::env::current_dir().map_err(|err| {
        Diagnostic::error(
            nvs_diagnostics::code::E_UNREADABLE_CONFIG,
            format!("the working directory could not be read: {err}"),
        )
    })
}

/// The snapshot this run's request reads — ADR 0103 § 1's roots, § 3's ordered
/// stream, ADR 0104 § 2's `[[app]]` fold for `entry`, and ADR 0078 § 1's
/// immutable result.
///
/// It replaces the hand-rolled one-key `nvs.toml` scanner that stood here for
/// ADR 0102 § 6's origin, which said in its own doc comment that a second key
/// added to it would be a second configuration format. This is the reader it
/// was waiting for, so the origin now arrives through `[[app]]` matching rather
/// than out of any block in the file.
///
/// `sources` is the caller's so that a refusal can be rendered with the line it
/// came from: a `nvs.toml` diagnostic carries a span into a file this map is
/// the only holder of.
pub(crate) fn boot_snapshot(
    config: &[PathBuf],
    entry: &Path,
    sources: &mut SourceMap,
) -> Result<Arc<nvs_config::Snapshot>, Diagnostic> {
    let files = LocalFiles;
    let cwd = working_directory()?;
    // ADR 0103 § 1: every `--config` in the order given, else `./nvs.toml`,
    // else the shipped defaults. `roots` owns all three steps, so this call is
    // the whole of the CLI's part in choosing what is read.
    let roots = nvs_config::resolve::roots(config, &cwd, &files);
    let resolved = nvs_config::resolve::resolve(&roots, sources, &files)?;
    nvs_config::Snapshot::build(&resolved, entry, &files)
}

/// `nvs config check [<file>...]` — resolve the tree and report what it holds,
/// exiting non-zero on any refusal.
///
/// The paths are ADR 0103 § 1 step 1's root list given positionally: naming one
/// disables step 2 exactly as `--config` does, so an audit of `/etc/nvs` on a
/// developer's machine never quietly merges the `./nvs.toml` beside the
/// checkout. With none named, § 1's own search runs — step 2's `./nvs.toml`,
/// else step 3's shipped defaults, which resolve and are reported as a tree of
/// no files rather than as a failure to find one.
///
/// It stops at the **first** refusal, because that is what `resolve` reports:
/// the tree is an ordered stream and a file that does not parse has no keys to
/// carry into the rest of it, so a second refusal found after the first would
/// be a guess about a tree that was never built.
///
/// The summary line is § 9's, and its four counts are the ones that make § 3's
/// precedence auditable: how many files the tree reached, how many keys are in
/// force, how many of those overrode an earlier assignment, and how many
/// advisories (§ 7's readable secret file today) were raised. `dump --origin`
/// is where each override is named; this line is what CI reads.
pub(crate) fn check(config: &[PathBuf], paths: &[PathBuf]) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(&named, &cwd, &files);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return ExitCode::FAILURE;
        }
    };

    // An advisory is printed in full and does not change the verdict — ADR 0103
    // § 7 says so, and `Resolved::warnings`' own doc says a refusal is never
    // here. The count on the summary line is so a green run still says one was
    // raised.
    if !resolved.warnings.is_empty() {
        let mut diags = Diagnostics::new();
        for warning in &resolved.warnings {
            diags.report(warning.clone());
        }
        render_diagnostics(&mut diags, &sources);
    }

    let set = listing(&resolved).len();
    println!(
        "ok: {} file{}, {} directive{} set, {} override{}, {} warning{}",
        resolved.files.len(),
        plural(resolved.files.len()),
        set,
        plural(set),
        resolved.overrides.len(),
        plural(resolved.overrides.len()),
        resolved.warnings.len(),
        plural(resolved.warnings.len()),
    );
    ExitCode::SUCCESS
}

/// `nvs config dump [--origin] [--toml] [<file>...]` — every key in force, one
/// per line, in dotted-key order.
///
/// The roots are read exactly as [`check`] reads them. This is § 9's other
/// offline half and § 3's own condition: later-wins is acceptable *because*
/// every override is recoverable, and `--origin` is where it is recovered in
/// full rather than summarized.
///
/// Three things about what is printed are decisions rather than formatting:
///
/// - **The dump is the merged table, not the typed configuration.** It is
///   therefore complete — a key no reader has a field for is still in force and
///   still printed — which is what an audit needs, and it is the same table
///   [ADR 0104] § 2 layers `[[app]]` blocks over.
/// - **A secret is a row of its own, rendered `<secret>` and naming its file.**
///   ADR 0103 § 7's value is read at boot and carried beside the table rather
///   than in it, so `db.main.password` is not a leaf of the merged table at all
///   — and § 9's listing would be missing a key that is in force if the dump
///   printed only what it walks. It is added back here, with the secret file in
///   the origin column, because that path is what an operator acts on: the
///   redaction is what the row says, and the file is what makes the row useful.
///   `--toml` prints the table and therefore carries no content, which is a
///   property of where the value is kept rather than of a redaction this dump
///   remembers to apply.
/// - **`--origin` names the file and not the line.** An [`Origin`] carries the
///   path and the `SourceId`, because that is what a refusal needs; a line would
///   need a span per leaf, and `toml::Value` carries none once the document is
///   parsed. The file is what an operator acts on — it is the one they edit —
///   so the column is useful without it, and the line is a later slice rather
///   than a hole in this one.
///
/// [ADR 0104]: ../../../docs/adr/0104-an-application-is-an-entry-file-path.md
/// [`Origin`]: nvs_config::resolve::Origin
pub(crate) fn dump(config: &[PathBuf], paths: &[PathBuf], origin: bool, as_toml: bool) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(&named, &cwd, &files);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return ExitCode::FAILURE;
        }
    };

    // `--toml` is one canonical file for diffing two environments, so it is the
    // table serialized and nothing else: no origins, no alignment, no advisory
    // on standard output. Anything added to that stream is something the diff
    // reports as a change.
    if as_toml {
        return match toml::to_string(&resolved.table) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: the resolved configuration could not be written as TOML: {err}");
                ExitCode::FAILURE
            }
        };
    }

    let leaves = listing(&resolved);
    let width = leaves.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
    // The origin is a column rather than a suffix, so a tree assembled from
    // five files reads down that column instead of along each line.
    let value_width = leaves
        .iter()
        .map(|(_, value)| value.len())
        .max()
        .unwrap_or(0);
    let overridden: std::collections::BTreeMap<&str, &nvs_config::resolve::Override> = resolved
        .overrides
        .iter()
        .map(|record| (record.key.as_str(), record))
        .collect();

    for (key, value) in &leaves {
        let mut line = if origin {
            format!("{key:width$} = {value:value_width$}")
        } else {
            format!("{key:width$} = {value}")
        };
        if origin {
            // A secret's origin is the file its *value* came from, not the file
            // that named it — the `password_file` row directly below it is where
            // the naming file is already reported, so printing that one twice
            // would leave the path § 7 makes the value nowhere in the listing.
            if let Some(secret) = resolved.secrets.get(key) {
                line.push_str(&format!("    {}", secret.file.display()));
            } else if let Some(written_in) = resolved.origins.get(key) {
                line.push_str(&format!("    {}", written_in.path.display()));
            }
            if let Some(record) = overridden.get(key.as_str()) {
                line.push_str(&format!(" (overrides {})", record.replaced.path.display()));
            }
        }
        println!("{line}");
    }
    ExitCode::SUCCESS
}

/// What ADR 0103 § 9 renders a secret's value as. Never the content, and never
/// a fixed-width mask that would say how long it is.
const REDACTED: &str = "<secret>";

/// § 9's listing: every leaf of the merged table, plus one row per secret, in
/// dotted-key order.
///
/// [`check`] counts it and [`dump`] prints it, so the summary line and the
/// listing cannot disagree about how many directives are set. A secret is
/// counted because it *is* set — `db.main.password` is a key with a value in
/// force, and the `password_file` beside it is a second key rather than the same
/// one spelled differently.
fn listing(resolved: &nvs_config::resolve::Resolved) -> Vec<(String, String)> {
    let mut out = leaves(&resolved.table);
    out.extend(
        resolved
            .secrets
            .keys()
            .map(|key| (key.clone(), REDACTED.to_owned())),
    );
    out.sort_by(|(left, _), (right, _)| left.cmp(right));
    out
}

/// Every key in force in `table`, as a dotted key and the TOML spelling of its
/// value, in dotted-key order.
///
/// [`listing`] is what both readers go through, and this is the half of it that
/// walks the table — counted rather than `Resolved::origins`, whose keys include
/// the containers a leaf hangs off.
fn leaves(table: &toml::Table) -> Vec<(String, String)> {
    let mut out = Vec::new();
    flatten(&mut out, String::new(), &toml::Value::Table(table.clone()));
    out
}

/// Every leaf of `value` as a dotted key and its TOML spelling, appended to
/// `out` in the order the table holds them — which is sorted, `toml::Table`
/// being a `BTreeMap` unless a feature says otherwise.
///
/// An array of tables descends by index, so the second `[[app]]` block's `root`
/// is `app.1.root`: the same spelling `Resolved::origins` uses, which is what
/// lets the origin column be a lookup rather than a second walk. An array of
/// scalars is a leaf, because [ADR 0103] § 4 makes a value array *replace* —
/// there is no per-element origin to report.
///
/// [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md
fn flatten(out: &mut Vec<(String, String)>, prefix: String, value: &toml::Value) {
    let joined = |key: &str| {
        if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        }
    };
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                flatten(out, joined(key), child);
            }
        }
        toml::Value::Array(items) if items.iter().any(toml::Value::is_table) => {
            for (index, child) in items.iter().enumerate() {
                flatten(out, joined(&index.to_string()), child);
            }
        }
        leaf => out.push((prefix, leaf.to_string())),
    }
}

/// The root list an audit reads: every `--config` in the order given, then
/// every file named positionally.
///
/// Two spellings for one of ADR 0103 § 1's steps, because the audit and the
/// runtime are asking about the same files from different directions. `--config`
/// is the runtime's, so `nvs config check --config /etc/nvs/nvs.toml` audits
/// exactly the argv the server will be given — § 9's own example is that
/// spelling. The positional list is the shorthand a person types at a prompt.
/// Naming both reads both, flag first, because that is the order a copied
/// server argv wants to keep.
pub(crate) fn named_roots(config: &[PathBuf], paths: &[PathBuf]) -> Vec<PathBuf> {
    config.iter().chain(paths).cloned().collect()
}

/// The `s` on a count, so the one-file case does not read as a template.
fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

#[cfg(test)]
mod tests {
    use super::leaves;

    /// The two array shapes ADR 0103 § 4 distinguishes, flattened the way
    /// `Resolved::origins` spells them: an array of tables descends by index so
    /// its leaves can be looked up there, and an array of scalars is one leaf
    /// because a value array replaces whole and has no per-element origin.
    #[test]
    fn an_array_of_tables_descends_by_index_and_a_value_array_is_one_leaf() {
        let table: toml::Table = toml::from_str(
            r#"
            [[app]]
            root = "/srv/one"
            [[app]]
            root = "/srv/two"
            [capabilities.fs]
            read = ["/srv", "/tmp"]
            "#,
        )
        .expect("the fixture is valid TOML");

        let leaves = leaves(&table);
        let rendered: Vec<(&str, &str)> = leaves
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        assert_eq!(
            rendered,
            vec![
                ("app.0.root", "\"/srv/one\""),
                ("app.1.root", "\"/srv/two\""),
                ("capabilities.fs.read", "[\"/srv\", \"/tmp\"]"),
            ]
        );
    }
}
