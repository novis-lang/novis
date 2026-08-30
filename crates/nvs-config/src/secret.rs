//! [ADR 0103] § 7: a secret arrives as a file whose content is the value.
//!
//! A directive the configuration marks secret gains a `_file` sibling, and **exactly one of the
//! pair may be set** — both is a refusal, which is what keeps this two sources for one value rather
//! than the second spelling [ADR 0015] refuses. The file's *whole* content is the value, with one
//! trailing `\n` — and a `\r` immediately before it — stripped and nothing else trimmed. Not a
//! general trim: a password may legitimately begin or end with a space, and removing it is
//! [ADR 0095]'s failure of repairing input instead of reading it. One newline goes because
//! `echo secret > f` produces one and every injection system does; a value that genuinely ends in a
//! newline is written with two.
//!
//! **The pair is spelled out in [`materialize`]** — `[db.<name>] password` and nothing else today —
//! because [`mod@crate::directive`]'s registry carries a key's changeability and reloadability and
//! has no field for secrecy. When it grows one this becomes a sweep over the registry; until then a
//! secret directive added without a line here is a `_file` sibling that silently does nothing, and
//! the roster in [`mod@crate::tree`] is the place that gap shows.
//!
//! **The secret file is a configuration input like any other**, so it goes through
//! [`Files::trust`] before it is read: an account that can rewrite it
//! chooses the credential the server connects with, which is exactly what § 6 refuses about the
//! files naming it. What § 7 does *not* inherit is confidentiality — a readable secret file is
//! `W1005` and not a refusal, because a Compose secret is mounted `0444`. [`mod@crate::trust`]'s
//! module doc owns that split.
//!
//! **The value stays out of every message this module writes.** A refusal names the file, the key
//! and the shape of the problem — empty, whitespace-only, oversized, not UTF-8 — and never a byte
//! of the content, which is [ADR 0033]'s type-level meaning applied one layer below the language.
//! `password_file` is left set after the value is materialized for the same reason: § 9's dump
//! renders the value `<secret>` and names the file it came from, and it can only do that if the
//! file is still recorded.
//!
//! **The value never enters the merged table**, and that is what [`Secret`] exists for. The table is
//! the stream § 9 serializes whole for `nvs config dump --toml` and the one
//! [`Snapshot::retype`](crate::Snapshot) rebuilds the typed tree out of, so a content put into it
//! would have to be redacted again by every reader that walks it — and the one that forgets writes a
//! credential into a file an operator diffs. Carried beside it instead, a secret reaches exactly the
//! three readers that ask for it by name: [`apply`] puts it back onto a typed tree, `Core\Config`
//! answers `db.main.password` with it, and the dump renders it `<secret>`. That is also why
//! [`apply`] is a second function rather than the tail of [`materialize`] — a snapshot deserializes
//! the table more than once (a reload carries `Boot` values across and retypes), and each of those
//! rounds has to put the secrets back without re-reading a file.
//!
//! Cost: one trust check, one advisory and one whole-file read per secret file, at boot and again
//! at each `nvs ctl reload`. Nothing here runs per request.
//!
//! [ADR 0015]: ../../../docs/adr/0015-no-name-aliasing.md
//! [ADR 0033]: ../../../docs/adr/0033-secret-qualifier-for-confidential-values.md
//! [ADR 0095]: ../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md
//! [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, origin_note};
use crate::tree::Config;

/// One secret, materialized: the value § 7 read, and the file it came from.
///
/// The file is kept because it is half of what § 9's dump prints — `<secret>` says a value is in
/// force and the path says which file decides it, and an audit that cannot name the file cannot act
/// on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Secret {
    /// The file's whole content, cut to § 7's value.
    pub value: String,
    /// The trusted, absolute path it was read from.
    pub file: PathBuf,
}

/// What [`materialize`] read out of the tree: every secret, by the dotted key it is the value of.
#[derive(Clone, Debug, Default)]
pub struct Materialized {
    /// Each secret under the key it answers — `db.main.password`, never the `_file` sibling that
    /// named it, which is still in the table under its own key.
    pub secrets: BTreeMap<String, Secret>,
    /// § 7's advisories, `W1005` for a secret file another account can read. Never a refusal: that
    /// arrives as the `Err` instead.
    pub warnings: Vec<Diagnostic>,
}

/// § 7's cap on a secret file, in bytes.
///
/// Not a resource bound — a credential is never this large, so the cap is what catches a
/// `password_file` pointed at a log, a database or a device before its content becomes a password.
pub const MAX_SECRET_BYTES: usize = 64 * 1024;

/// Reads every secret file the flattened tree names, filling each `_file` sibling's value.
///
/// `origins` is the merge's key-to-file record: it says which file each `password_file` was written
/// in, which is both the directory § 5 resolves a relative path against and the file a refusal has
/// to name. A key with no origin is one no file set, so it is not read.
///
/// `config` comes back with every value filled — [`apply`] does that half, and is called again by
/// whoever deserializes the tree a second time.
///
/// # Errors
///
/// One [`Diagnostic`]: `E0608` for a pair set twice or a file § 7 will not take a value from,
/// `E0607` for a secret file outside the trust boundary, `E0605` for one that cannot be read.
///
/// Returns § 7's advisory warnings with the values — `W1005` for a secret file another account can
/// read — which are never a reason to stop.
pub fn materialize(
    config: &mut Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<Materialized, Diagnostic> {
    let mut out = Materialized::default();
    for (name, db) in &config.db {
        let Some(named) = db.password_file.as_deref() else {
            continue;
        };
        let key = format!("db.{name}.password_file");
        let written_in = origins.get(&key);
        if db.password.is_some() {
            return Err(both_set(&format!("db.{name}"), "password", written_in));
        }
        let secret = read(named, &key, written_in, files, &mut out.warnings)?;
        out.secrets.insert(format!("db.{name}.password"), secret);
    }
    apply(config, &out.secrets);
    Ok(out)
}

/// Puts every materialized secret onto a typed tree, at the key it is the value of.
///
/// The pair is spelled out here for the module doc's reason, and this is the *only* place a value
/// reaches [`Config`]: a tree deserialized from the merged table has `password_file` and no
/// `password`, so every deserialization is followed by this call — [`materialize`]'s own, and each
/// one a [`Snapshot`](crate::Snapshot) makes when it retypes.
///
/// A key naming a `[db]` block this tree does not have is skipped rather than creating one: the
/// secrets came from a table this same tree was built from, so that can only be a caller applying
/// one tree's secrets to another's configuration.
pub fn apply(config: &mut Config, secrets: &BTreeMap<String, Secret>) {
    for (name, db) in &mut config.db {
        if let Some(secret) = secrets.get(&format!("db.{name}.password")) {
            db.password = Some(secret.value.clone());
        }
    }
}

/// One secret file: trusted, advised on, read, and cut to its value.
fn read(
    named: &str,
    key: &str,
    written_in: Option<&Origin>,
    files: &dyn Files,
    warnings: &mut Vec<Diagnostic>,
) -> Result<Secret, Diagnostic> {
    // § 5: relative to the file it was written in, which is the same rule an `[[include]]` path
    // follows. A key with no origin cannot have been written anywhere, so there is nothing but the
    // path itself to resolve against.
    let base = written_in
        .and_then(|origin| origin.path.parent())
        .unwrap_or(Path::new("."));
    let path = crate::resolve::absolute(base, Path::new(named));

    let trusted = files.trust(&path).map_err(|why| {
        crate::resolve::untrusted(
            &path,
            &why,
            "a secret directive's `_file` names it, and whoever can write it chooses the \
             credential the server connects with",
        )
    })?;
    if let Some(how) = files.exposure(&trusted) {
        warnings.push(exposed(&trusted, key, &how));
    }
    let bytes = files.read_bytes(&trusted).map_err(|err| {
        crate::resolve::unreadable(&trusted, &err, "a secret directive's `_file` names it")
    })?;

    if bytes.len() > MAX_SECRET_BYTES {
        return Err(refusal(
            &trusted,
            key,
            &format!(
                "is larger than {} KiB ({} bytes)",
                MAX_SECRET_BYTES / 1024,
                bytes.len()
            ),
            written_in,
        )
        .with_help(
            "a credential is never this large; a `_file` this size is a path pointed at something \
             else"
                .to_string(),
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| refusal(&trusted, key, "is not valid UTF-8", written_in))?;

    let value = strip_one_newline(&text);
    if value.is_empty() {
        return Err(refusal(&trusted, key, "is empty", written_in));
    }
    if value.trim().is_empty() {
        return Err(refusal(&trusted, key, "holds only whitespace", written_in));
    }
    Ok(Secret {
        value: value.to_string(),
        file: trusted,
    })
}

/// One trailing `\n`, and the `\r` before it, and nothing else — § 7's whole trimming rule.
///
/// Only the last newline: a file ending in two of them yields a value ending in one, which is how
/// § 7 spells a credential that genuinely ends in a newline. Leading and interior bytes are never
/// touched, spaces included.
fn strip_one_newline(text: &str) -> &str {
    let cut = text.strip_suffix('\n').unwrap_or(text);
    cut.strip_suffix('\r').unwrap_or(cut)
}

/// `E0608` when both halves of a pair are set: two sources for one value, and § 7 allows one.
fn both_set(block: &str, value: &str, written_in: Option<&Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_SECRET_FILE,
        format!("`[{block}]` sets both `{value}` and `{value}_file`"),
    )
    .with_note(format!(
        "ADR 0103 § 7 gives a secret directive two sources and exactly one may be set: `{value}` \
         holds the value, `{value}_file` names the file whose content is the value{}",
        origin_note(written_in)
    ))
    .with_help(format!(
        "remove whichever of the two the deployment does not mean; a `{value}` left behind from \
         before the file existed is the usual one"
    ))
}

/// `E0608` about the file's content, phrased as a predicate the path is prefixed onto — the same
/// shape [`mod@crate::trust`] reports a breach in, so the two read alike in a boot log.
fn refusal(path: &Path, key: &str, problem: &str, written_in: Option<&Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_SECRET_FILE,
        format!("`{key}` names `{}`, which {problem}", path.display()),
    )
    .with_note(format!(
        "ADR 0103 § 7 makes that file's whole content the value, minus one trailing newline, and \
         refuses rather than carrying an unusable one forward: an empty credential otherwise fails \
         at the first request instead of at boot{}",
        origin_note(written_in)
    ))
}

/// `W1005`: the advisory half, which names the mode and stops.
fn exposed(path: &Path, key: &str, how: &str) -> Diagnostic {
    Diagnostic::warning(
        code::W_SECRET_FILE_READABLE,
        format!("`{key}` names `{}`, which {how}", path.display()),
    )
    .with_note(
        "ADR 0103 § 7 warns rather than refuses here because a Compose secret is mounted `0444` \
         and a Kubernetes secret volume defaults to `0644`: inside a container that is the norm, \
         on a shared host it is not, and nothing readable from here says which this is"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::strip_one_newline;

    /// § 7's trimming, on both sides of the one rule it has: one newline goes, a second stays, and
    /// nothing else is touched — the space a password may legitimately end with least of all.
    #[test]
    fn one_trailing_newline_is_stripped_and_nothing_else_is() {
        assert_eq!(strip_one_newline("hunter2\n"), "hunter2");
        assert_eq!(strip_one_newline("hunter2\r\n"), "hunter2");
        assert_eq!(strip_one_newline("hunter2\n\n"), "hunter2\n");
        assert_eq!(strip_one_newline("hunter2"), "hunter2");
        assert_eq!(strip_one_newline(" hunter2 \n"), " hunter2 ");
        assert_eq!(strip_one_newline("\nhunter2"), "\nhunter2");
    }
}
