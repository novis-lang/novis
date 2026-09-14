//! `rule:config/check-and-dump-audit-the-tree-offline`'s listing and
//! `rule:config/ctl-config-reports-the-live-snapshot`'s: every key in force, one per line, in
//! dotted-key order, beside the file each was written in.
//!
//! **One renderer, two trees.** `nvs config dump --origin` reads the files on disk; `nvs ctl config`
//! reads the snapshot a running process is serving. The second rule has an operator diff the two
//! against each other, so a difference between them means a reload that has not happened, a file
//! that changed since the last one, or a directory-mode change that will refuse the next one — and
//! it means those things only while **one** function renders both. [`Audit`] is the borrowed view
//! each side hands over: the merged table, where every leaf was written, what each override
//! replaced, the secrets that are in force beside the table rather than in it, and the `Boot` keys a
//! reload reported and left unapplied.
//!
//! The listing is the **merged table**, not the typed configuration, so a key no reader has a field
//! for is still in force and still printed. That is what an audit needs, and it is the same table
//! `rule:config/every-matching-app-block-applies-least-specific-first` layers `[[app]]` blocks over.
//!
//! Cost, as `rule:programs/memory-priority` requires: one `String` per audit, held for as long as it
//! takes to print it or write it to a socket, plus the flattened key list it is built from. Nothing
//! is retained here and nothing is per request.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::resolve::{Origin, Override, Resolved};
use crate::secret::Secret;
use crate::snapshot::Snapshot;

/// What `rule:config/check-and-dump-audit-the-tree-offline` renders a secret's value as. Never the
/// content, and never a fixed-width mask that would say how long it is.
const REDACTED: &str = "<secret>";

/// The directive that says who resolves an outbound destination, as [`leaves`] spells a dotted key.
/// The value that gives the address question away is `crate::http`'s own word, so the tree and this
/// listing cannot disagree about which one it is.
const RESOLVE_DIRECTIVE: &str = "http.client.proxy.resolve";

/// What the directive above is rendered beside when it carries that word: the rule whose table the
/// proxy is then enforcing, by id and not summarized. An audit is read by an operator who looks the
/// id up, and a sentence here would be a second wording of a rule that has one home.
const NARROWED_POLICY: &str = "rule:security/net-address-policy";

/// What a row carries when the running value is not what the tree on disk now says.
///
/// `rule:config/ctl-config-reports-the-live-snapshot` asks for those keys by name, and this is where
/// they go: on the row that holds the value still in force, so the listing is still one line per key
/// and a diff against `nvs config dump --origin` lands on exactly the key that differs.
const UNAPPLIED: &str = "(changed on disk; needs a restart)";

/// The tree an audit is of, borrowed — `rule:config/check-and-dump-audit-the-tree-offline`.
///
/// It is four borrows and not an owned copy because both callers already hold every part: an offline
/// audit holds a [`Resolved`], a live one holds the [`Snapshot`] it is serving, and rendering reads
/// each part once.
#[derive(Clone, Copy, Debug)]
pub struct Audit<'a> {
    /// The merged table, whose every leaf is a row.
    pub table: &'a toml::Table,
    /// Where each of those leaves was written, by dotted key.
    pub origins: &'a BTreeMap<String, Origin>,
    /// Every override, in the order they happened — `rule:config/later-wins-and-every-override-is-recorded`.
    pub overrides: &'a [Override],
    /// The secrets in force, by the key each is the value of. Each is a row of its own, rendered
    /// [`REDACTED`] and naming the file its *value* came from.
    pub secrets: &'a BTreeMap<String, Secret>,
    /// The `Boot` keys whose changed value the last reload reported and left unapplied. Empty for an
    /// offline audit, which has no reload behind it to have reported anything.
    pub unapplied: &'a [&'static str],
}

impl<'a> Audit<'a> {
    /// The tree as the files on disk resolve it — `nvs config dump`'s half.
    #[must_use]
    pub fn of_files(resolved: &'a Resolved) -> Self {
        Self {
            table: &resolved.table,
            origins: &resolved.origins,
            overrides: &resolved.overrides,
            secrets: &resolved.secrets,
            unapplied: &[],
        }
    }

    /// The tree a running process is actually serving — `nvs ctl config`'s half.
    ///
    /// `unapplied` is the caller's because the snapshot does not hold it: publishing carries every
    /// running `Boot` value back over the incoming tree, so by the time a snapshot exists the change
    /// that was refused is nowhere in it. The reload's own [`Report`](crate::control::Report) is
    /// where it lives, and the process that took that reload is what remembers it.
    #[must_use]
    pub fn of_snapshot(snapshot: &'a Snapshot, unapplied: &'a [&'static str]) -> Self {
        Self {
            table: &snapshot.table,
            origins: &snapshot.origins,
            overrides: &snapshot.overrides,
            secrets: &snapshot.secrets,
            unapplied,
        }
    }

    /// Every leaf of the table, plus one row per secret, in dotted-key order.
    ///
    /// `nvs config check` counts this and [`render`](Audit::render) prints it, so the summary line
    /// and the listing cannot disagree about how many directives are set. A secret is counted
    /// because it *is* set — `db.main.password` is a key with a value in force, and the
    /// `password_file` beside it is a second key rather than the same one spelled differently.
    #[must_use]
    pub fn listing(&self) -> Vec<(String, String)> {
        let mut out = leaves(self.table);
        out.extend(
            self.secrets
                .keys()
                .map(|key| (key.clone(), REDACTED.to_owned())),
        );
        out.sort_by(|(left, _), (right, _)| left.cmp(right));
        out
    }

    /// The listing as text, one row per line, each line ended.
    ///
    /// `origin` adds the file column. It is a column rather than a suffix, so a tree assembled from
    /// five files reads down that column instead of along each line, and the value is padded to hold
    /// it straight.
    ///
    /// `--origin` **names the file and not the line**: an [`Origin`] carries the path and the
    /// `SourceId` because that is what a refusal needs, and a line would need a span per leaf, which
    /// `toml::Value` carries none of once the document is parsed. The file is what an operator edits,
    /// so the column is useful without it.
    #[must_use]
    pub fn render(&self, origin: bool) -> String {
        let rows = self.listing();
        let width = rows.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
        let value_width = rows.iter().map(|(_, value)| value.len()).max().unwrap_or(0);
        let overridden: BTreeMap<&str, &Override> = self
            .overrides
            .iter()
            .map(|record| (record.key.as_str(), record))
            .collect();

        let mut out = String::new();
        for (key, value) in &rows {
            let mut line = if origin {
                format!("{key:width$} = {value:value_width$}")
            } else {
                format!("{key:width$} = {value}")
            };
            if origin {
                // A secret's origin is the file its *value* came from, not the file that named it —
                // the `password_file` row directly below it is where the naming file is already
                // reported, so printing that one twice would leave the path
                // `rule:config/a-secret-is-a-file-whose-content-is-the-value` makes the value
                // nowhere in the listing.
                if let Some(secret) = self.secrets.get(key) {
                    let _ = write!(line, "    {}", secret.file.display());
                } else if let Some(written_in) = self.origins.get(key.as_str()) {
                    let _ = write!(line, "    {}", written_in.path.display());
                }
                if let Some(record) = overridden.get(key.as_str()) {
                    let _ = write!(line, " (overrides {})", record.replaced.path.display());
                }
                if self.unapplied.iter().any(|boot| *boot == key) {
                    line.push_str(&format!(" {UNAPPLIED}"));
                }
            }
            // The offline half of
            // `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`:
            // the one word that hands the address question to the proxy reads as an ordinary
            // directive set to an ordinary string, so the rule it narrows is rendered beside it. An
            // audit of a tree with no server running is exactly where the boot's own `Warn` record
            // is not there to say so.
            if key == RESOLVE_DIRECTIVE
                && value.trim_matches('"') == crate::http::RESOLVE_AT_THE_PROXY
            {
                let _ = write!(line, "    {NARROWED_POLICY}");
            }
            line.push('\n');
            out.push_str(&line);
        }
        out
    }
}

/// Every key in force in `table`, as a dotted key and the TOML spelling of its value, in dotted-key
/// order.
///
/// Counted rather than [`Resolved::origins`], whose keys include the containers a leaf hangs off.
#[must_use]
pub fn leaves(table: &toml::Table) -> Vec<(String, String)> {
    let mut out = Vec::new();
    flatten(&mut out, String::new(), &toml::Value::Table(table.clone()));
    out
}

/// Every leaf of `value` as a dotted key and its TOML spelling, appended to `out` in the order the
/// table holds them — which is sorted, `toml::Table` being a `BTreeMap` unless a feature says
/// otherwise.
///
/// An array of tables descends by index, so the second `[[app]]` block's `root` is `app.1.root`: the
/// same spelling [`Resolved::origins`] uses, which is what lets the origin column be a lookup rather
/// than a second walk. An array of scalars is a leaf, because
/// `rule:config/a-value-array-replaces-and-a-table-appends` makes a value array *replace* — there is
/// no per-element origin to report.
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

#[cfg(test)]
mod tests {
    use super::{Audit, leaves};

    /// The two array shapes `rule:config/a-value-array-replaces-and-a-table-appends` distinguishes,
    /// flattened the way `Resolved::origins` spells them: an array of tables descends by index so
    /// its leaves can be looked up there, and an array of scalars is one leaf because a value array
    /// replaces whole and has no per-element origin.
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

        let rows = leaves(&table);
        let rendered: Vec<(&str, &str)> = rows
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

    /// `rule:config/ctl-config-reports-the-live-snapshot`: a `Boot` key the last reload could not
    /// apply is named on its own row, beside the value still in force — so the listing stays one
    /// line per key and a diff against `nvs config dump --origin` lands on the key that differs.
    #[test]
    fn an_unapplied_boot_key_is_named_on_the_row_that_still_holds_the_running_value() {
        let table: toml::Table = "[limits]\nmemory = \"128M\"\n"
            .parse()
            .expect("the fixture is valid TOML");
        let origins = std::collections::BTreeMap::new();
        let secrets = std::collections::BTreeMap::new();

        let plain = Audit {
            table: &table,
            origins: &origins,
            overrides: &[],
            secrets: &secrets,
            unapplied: &[],
        };
        assert_eq!(plain.render(true), "limits.memory = \"128M\"\n");

        let after = Audit {
            unapplied: &["limits.memory"],
            ..plain
        };
        assert_eq!(
            after.render(true),
            format!(
                "limits.memory = \"128M\" {UNAPPLIED}\n",
                UNAPPLIED = super::UNAPPLIED
            ),
            "the running value is the one reported, and the row says the tree on disk disagrees",
        );
        assert_eq!(
            after.render(false),
            plain.render(false),
            "and without the origin column there is no column for it to be in",
        );
    }
}
