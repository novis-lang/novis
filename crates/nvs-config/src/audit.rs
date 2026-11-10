//! `rule:config/check-and-dump-audit-the-tree-offline`'s listing: every key in force, one per line,
//! in dotted-key order, beside the file each was written in or the word `default`.
//!
//! [`Audit`] is the borrowed view `nvs config dump --origin` hands over: the merged table, where
//! every leaf was written, what each override replaced, the secrets that are in force beside the
//! table rather than in it, and the defaults no file overrode.
//!
//! The written half is the **merged table**, not the typed configuration, so a key no reader has a
//! field for is still in force and still printed. That is what an audit needs, and it is the same
//! table `rule:config/every-matching-app-block-applies-least-specific-first` layers `[[app]]` blocks
//! over. The default half is [`crate::defaults::unwritten`] at [`defaults_of`]'s run: the values
//! `nvs serve` has for every key no file wrote.
//!
//! Cost, as `rule:programs/memory-priority` requires: one `String` per audit, held for as long as it
//! takes to print it, plus the flattened key list and the default rows it is built from. Nothing
//! is retained here and nothing is per request.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::defaults::{At, Unwritten};
use crate::mode;
use crate::resolve::{Origin, Override, Resolved};
use crate::secret::{REDACTED, Secret};
use crate::snapshot::value_at;

/// The directive that says who resolves an outbound destination, as [`leaves`] spells a dotted key.
/// The value that gives the address question away is `crate::http`'s own word, so the tree and this
/// listing cannot disagree about which one it is.
const RESOLVE_DIRECTIVE: &str = "http.client.proxy.resolve";

/// What the directive above is rendered beside when it carries that word: the rule whose table the
/// proxy is then enforcing, by id and not summarized. An audit is read by an operator who looks the
/// id up, and a sentence here would be a second wording of a rule that has one home.
const NARROWED_POLICY: &str = "rule:security/net-address-policy";

/// The tree an audit is of, borrowed — `rule:config/check-and-dump-audit-the-tree-offline`.
///
/// It is five borrows and not an owned copy because the caller already holds every part, four in a
/// [`Resolved`] and the defaults from [`defaults_of`], and rendering reads each part once.
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
    /// The defaults in force that no file wrote. Each is a row of its own, whose origin is
    /// `default`, or `default (<mode>)` where the mode chose the value.
    pub defaults: &'a [Unwritten],
}

impl<'a> Audit<'a> {
    /// The tree as the files on disk resolve it, with no default rows.
    #[must_use]
    pub fn of_files(resolved: &'a Resolved) -> Self {
        Self {
            table: &resolved.table,
            origins: &resolved.origins,
            overrides: &resolved.overrides,
            secrets: &resolved.secrets,
            defaults: &[],
        }
    }

    /// This audit with `defaults` listed beside the keys the files wrote — `nvs config dump`'s
    /// listing, with `defaults` from [`defaults_of`] over the same table.
    #[must_use]
    pub fn with_defaults(self, defaults: &'a [Unwritten]) -> Self {
        Self { defaults, ..self }
    }

    /// Every leaf of the table, plus one row per secret, in dotted-key order: the keys the files
    /// wrote, and no default.
    ///
    /// `nvs config check` counts this as the directives set, and [`render`](Audit::render) prints
    /// it with the default rows beside it. A secret is counted because it *is* set —
    /// `db.main.password` is a key with a value in force, and the `password_file` beside it is a
    /// second key rather than the same one spelled differently.
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

    /// The listing as text, one row per line, each line ended: every key a file wrote and every
    /// default row, merged in dotted-key order.
    ///
    /// `origin` adds the origin column: the file a key was written in, or `default`, or
    /// `default (production)` and `default (development)` where the mode chose the value. It is a
    /// column rather than a suffix, so a tree assembled from five files reads down that column
    /// instead of along each line, and the value is padded to hold it straight.
    ///
    /// `--origin` **names the file and not the line**: an [`Origin`] carries the path and the
    /// `SourceId` because that is what a refusal needs, and a line would need a span per leaf, which
    /// `toml::Value` carries none of once the document is parsed. The file is what an operator edits,
    /// so the column is useful without it.
    #[must_use]
    pub fn render(&self, origin: bool) -> String {
        // Each key is printed with its segments quoted where they need it, so a table named with a
        // newline, `[db."main\n…"]`, cannot start a row of its own. A secret is not a leaf of the
        // table; its `_file` sibling is, and the two share every segment but the last.
        let spellings: BTreeMap<String, String> = spelled_leaves(self.table)
            .into_iter()
            .map(|leaf| (leaf.key, leaf.shown))
            .collect();
        let overridden: BTreeMap<&str, &Override> = self
            .overrides
            .iter()
            .map(|record| (record.key.as_str(), record))
            .collect();
        let mut rows: Vec<Row> = self
            .listing()
            .into_iter()
            .map(|(key, value)| {
                let shown = match spellings.get(&key) {
                    Some(shown) => shown.clone(),
                    None => spellings
                        .get(&format!("{key}_file"))
                        .and_then(|file| file.strip_suffix("_file"))
                        .map_or_else(
                            || key.split('.').map(segment).collect::<Vec<_>>().join("."),
                            str::to_owned,
                        ),
                };
                // A secret's origin is the file its *value* came from, not the file that named it —
                // the `password_file` row directly below it is where the naming file is already
                // reported, so printing that one twice would leave the path
                // `rule:config/a-secret-is-a-file-whose-content-is-the-value` makes the value
                // nowhere in the listing.
                let mut from = if let Some(secret) = self.secrets.get(&key) {
                    format!("    {}", secret.file.display())
                } else if let Some(written_in) = self.origins.get(key.as_str()) {
                    format!("    {}", written_in.path.display())
                } else {
                    String::new()
                };
                if let Some(record) = overridden.get(key.as_str()) {
                    let _ = write!(from, " (overrides {})", record.replaced.path.display());
                }
                Row {
                    key,
                    shown,
                    value,
                    from,
                }
            })
            .collect();
        // A default's key is spelled segment by segment like a written one, so a named block's
        // default rows quote its name exactly as its written rows do.
        rows.extend(self.defaults.iter().map(|default| {
            Row {
                key: default.key(),
                shown: default
                    .path
                    .iter()
                    .map(String::as_str)
                    .map(segment)
                    .collect::<Vec<_>>()
                    .join("."),
                value: spelled(&default.value),
                from: match &default.mode {
                    Some(chosen) => format!("    default ({chosen})"),
                    None => "    default".to_owned(),
                },
            }
        }));
        rows.sort_by(|left, right| left.key.cmp(&right.key));
        let width = rows.iter().map(|row| row.shown.len()).max().unwrap_or(0);
        let value_width = rows.iter().map(|row| row.value.len()).max().unwrap_or(0);

        let mut out = String::new();
        for Row {
            key,
            shown,
            value,
            from,
        } in &rows
        {
            let mut line = if origin {
                format!("{shown:width$} = {value:value_width$}{from}")
            } else {
                format!("{shown:width$} = {value}")
            };
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

/// One line of [`Audit::render`] before it is laid out.
struct Row {
    /// The dotted key the rows are sorted by.
    key: String,
    /// The key as the line prints it, each segment through [`segment`].
    shown: String,
    /// The value, through [`spelled`].
    value: String,
    /// The origin column with its leading gap, or empty where nothing names one.
    from: String,
}

/// The defaults `nvs config dump` lists beside the keys `table` writes:
/// [`crate::defaults::unwritten`] for server work, in the mode `table` starts in.
///
/// Server work, because an audit is of the tree a server is given. A command-line run of the same
/// tree has no `limits.cpu_time`, and no `limits.hard.memory` unless a file writes one. The mode is
/// `[mode] default` where the table writes one of the two modes, and `production` otherwise, as a
/// server that wrote nothing starts in.
#[must_use]
pub fn defaults_of(table: &toml::Table) -> Vec<Unwritten> {
    let started = value_at(table, mode::KEY)
        .and_then(toml::Value::as_str)
        .filter(|written| mode::rank(written).is_some())
        .unwrap_or(mode::PRODUCTION);
    crate::defaults::unwritten(&At {
        table,
        serving: true,
        mode: started,
        started,
    })
}

/// Every key in force in `table`, as a dotted key and the TOML spelling of its value, in dotted-key
/// order.
///
/// Counted rather than [`Resolved::origins`], whose keys include the containers a leaf hangs off.
#[must_use]
pub fn leaves(table: &toml::Table) -> Vec<(String, String)> {
    spelled_leaves(table)
        .into_iter()
        .map(|leaf| (leaf.key, leaf.value))
        .collect()
}

/// One row of the listing before it is laid out.
struct Leaf {
    /// The dotted key every lookup uses: [`Resolved::origins`] and the secrets.
    key: String,
    /// The same key as the listing prints it, each segment through [`segment`].
    shown: String,
    /// The value, through [`spelled`].
    value: String,
}

/// [`leaves`], with the printed spelling of each key beside the one lookups use.
fn spelled_leaves(table: &toml::Table) -> Vec<Leaf> {
    let mut out = Vec::new();
    flatten(&mut out, "", "", &toml::Value::Table(table.clone()));
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
fn flatten(out: &mut Vec<Leaf>, key: &str, shown: &str, value: &toml::Value) {
    let joined = |prefix: &str, next: &str| {
        if prefix.is_empty() {
            next.to_string()
        } else {
            format!("{prefix}.{next}")
        }
    };
    match value {
        toml::Value::Table(table) => {
            for (name, child) in table {
                flatten(
                    out,
                    &joined(key, name),
                    &joined(shown, &segment(name)),
                    child,
                );
            }
        }
        toml::Value::Array(items) if items.iter().any(toml::Value::is_table) => {
            for (index, child) in items.iter().enumerate() {
                let index = index.to_string();
                flatten(out, &joined(key, &index), &joined(shown, &index), child);
            }
        }
        leaf => out.push(Leaf {
            key: key.to_owned(),
            shown: shown.to_owned(),
            value: spelled(leaf),
        }),
    }
}

/// `value` as TOML spells it on **one line**, which is what keeps the listing one row per key.
///
/// `toml`'s own `Display` writes a string holding a newline as a multi-line string, so a value
/// could print a line of its own that reads as another directive — `limits.hard.memory = "99G"`
/// forged by an `origin` nobody checks. Every string here is a basic string instead, with each
/// character a terminal would act on or hide escaped.
fn spelled(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => quoted(text),
        toml::Value::Array(items) => {
            let items: Vec<String> = items.iter().map(spelled).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(table) => {
            let pairs: Vec<String> = table
                .iter()
                .map(|(key, child)| format!("{} = {}", segment(key), spelled(child)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        scalar => scalar.to_string(),
    }
}

/// One key segment, bare when every character may stand bare in TOML and quoted otherwise.
fn segment(key: &str) -> String {
    let bare = !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if bare { key.to_owned() } else { quoted(key) }
}

/// `text` as a TOML basic string, on one line, with every control character and every
/// directional or invisible formatting character written as an escape, so the row shows what the
/// file holds and not what a terminal makes of it.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() || hidden(ch) => {
                let _ = write!(out, "\\u{:04X}", u32::from(ch));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// A formatting character that changes how the text around it reads without showing itself: the
/// directional marks, embeddings, overrides and isolates, the line and paragraph separators, and
/// the zero-width characters.
fn hidden(ch: char) -> bool {
    matches!(
        ch,
        '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{2028}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}'
    )
}

#[cfg(test)]
mod tests {
    use super::{Audit, defaults_of, leaves};

    /// The default rows of a tree that writes `development`, one `log.level`, a `[db.main]` block
    /// and one `[[schedule]]` entry: a key the tree writes has no default row, a mode row names the
    /// mode that chose it, a named block and an entry get their own keys' defaults, and
    /// `[log] target` is listed although `Core\Config::get` never renders it.
    // covers: tools:cli/nvs-config-check-and-nvs-config-dump
    #[test]
    fn every_default_no_file_wrote_is_a_row_naming_what_chose_it() {
        let table: toml::Table = toml::from_str(
            r#"
            [mode]
            default = "development"
            [log]
            level = "Info"
            [db.main]
            driver = "sqlite"
            [[schedule]]
            script = "jobs/report.nvs"
            cron = "0 * * * *"
            "#,
        )
        .expect("the fixture is valid TOML");
        let defaults = defaults_of(&table);
        let found = |key: &str| defaults.iter().find(|row| row.key() == key);

        assert!(found("log.level").is_none(), "a written key is no default");
        assert!(
            found("mode.default").is_none(),
            "a written key is no default"
        );
        let format = found("log.format").expect("`log.format` has a default row");
        assert_eq!(format.value, toml::Value::String("text".to_owned()));
        assert_eq!(format.mode.as_deref(), Some("development"));
        let inline = found("debug.inline").expect("`debug.inline` has a default row");
        assert_eq!(inline.value, toml::Value::Boolean(true));
        let target = found("log.target").expect("`log.target` has a default row");
        assert_eq!(target.value, toml::Value::String("stderr".to_owned()));
        assert_eq!(target.mode, None);
        assert!(found("db.main.pool.max").is_some(), "{defaults:?}");
        assert!(found("schedule.0.timezone").is_some(), "{defaults:?}");
        assert!(
            found("ldap.main.timeout").is_none(),
            "no `[ldap]` block is written: {defaults:?}"
        );

        let origins = std::collections::BTreeMap::new();
        let secrets = std::collections::BTreeMap::new();
        let audit = Audit {
            table: &table,
            origins: &origins,
            overrides: &[],
            secrets: &secrets,
            defaults: &[],
        }
        .with_defaults(&defaults);
        let listing = audit.render(true);
        let row = |key: &str| {
            listing
                .lines()
                .find(|line| line.starts_with(&format!("{key} ")))
                .unwrap_or_else(|| panic!("no `{key}` row:\n{listing}"))
                .trim_end()
                .to_owned()
        };
        let format = row("log.format");
        assert!(
            format.contains("= \"text\" ") && format.ends_with("    default (development)"),
            "{listing}"
        );
        assert!(row("log.target").ends_with("default"), "{listing}");
        assert!(!row("log.level").contains("default"), "{listing}");
        assert_eq!(
            listing.lines().count(),
            audit.listing().len() + defaults.len(),
            "one line per written key and per default: {listing}"
        );
        assert!(
            audit
                .render(false)
                .lines()
                .all(|line| !line.contains("default (")),
            "the origin column is printed only with `--origin`"
        );
    }

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

    /// `rule:config/check-and-dump-audit-the-tree-offline`'s listing is one line per key, even when a
    /// value or a table name holds a newline written to look like another directive. Each is spelled
    /// on its row with the newline, the quotes and the right-to-left override escaped.
    // covers: tools:cli/nvs-config-check-and-nvs-config-dump
    #[test]
    fn a_value_or_a_table_name_holding_a_newline_cannot_forge_a_row() {
        let table: toml::Table = toml::from_str(concat!(
            "[[app]]\n",
            "root = \".\"\n",
            "origin = \"https://example.test\\nlimits.hard.memory = \\\"99G\\\"\\u202e\\t\"\n",
            "[db.\"main\\nlimits.hard.memory = \\\"99G\\\"\"]\n",
            "hosts = [\"a\\nb\", \"c\"]\n",
        ))
        .expect("the fixture is valid TOML");
        let origins = std::collections::BTreeMap::new();
        let secrets = std::collections::BTreeMap::new();
        let audit = Audit {
            table: &table,
            origins: &origins,
            overrides: &[],
            secrets: &secrets,
            defaults: &[],
        };

        let listing = audit.render(false);
        let forged_table = r#"db."main\nlimits.hard.memory = \"99G\"".hosts"#;
        let width = forged_table.len();
        let expected = [
            (
                "app.0.origin",
                concat!(
                    r#""https://example.test\nlimits.hard.memory = \"99G\""#,
                    "\\u",
                    "202E",
                    r#"\t""#
                ),
            ),
            ("app.0.root", r#"".""#),
            (forged_table, r#"["a\nb", "c"]"#),
        ]
        .map(|(key, value)| format!("{key:width$} = {value}\n"))
        .concat();
        assert_eq!(listing, expected);
        assert_eq!(listing.lines().count(), audit.listing().len());
        assert!(
            !listing.lines().any(|line| line.starts_with("limits")),
            "no row reads as a limit: {listing}"
        );
    }
}
