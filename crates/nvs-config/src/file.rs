//! Reading one file of the configuration: TOML in, a deserialized block tree out, and a refusal
//! that carries a span rather than a byte offset.
//!
//! `rule:config/the-file-is-nvs-toml-and-it-is-toml` fixes the format as TOML read through `serde`, and § 3 makes both a duplicate key
//! and an unknown one an error *per file*: in a root-owned file where one table grants
//! capabilities, an assignment silently shadowed by a later copy of itself is a security-relevant
//! failure that costs nothing to refuse. Neither refusal is machinery of ours — the `toml` crate
//! refuses the duplicate and `deny_unknown_fields` refuses the unknown key — so what this module
//! owns is the *reporting*: the file is registered in the caller's [`SourceMap`] before it is
//! parsed, so a failure renders through `rule:errors/diagnostic-record`'s one diagnostic record with the offending line
//! under it, exactly as a compiler error does — plus the one thing `serde` cannot say, which is
//! which **block** the key was found in ([`block_at`]).
//!
//! One unknown key is not refused: a table header the typed tree has no block for, with nothing
//! under it that sets a key, is removed before the file is typed and reported as `W1024`
//! ([`parse_with_warnings`]). The refusal exists so a typo cannot read as "granted nothing", and an
//! empty header reads as nothing whichever name it carries, so a file written for a release that
//! still had the block keeps starting. The removal runs only on the failure path, so a file that
//! types first time pays nothing for it.
//!
//! The typed tree is [`crate::tree::Config`]; this function stays generic over it so a caller
//! wanting the raw table still has one.
//!
//! Across an `[[include]]` the same key set twice is an override and not a duplicate, which is
//! [`crate::directive`]'s neighbour and `rule:config/later-wins-and-every-override-is-recorded`'s; this module answers about one file.
//!
//! Cost: the file's text is held once in the `SourceMap` for as long as the caller keeps it, which
//! is what buys the snippet under the diagnostic. Nothing here runs per request.

use std::ops::Range;

use nvs_diagnostics::{Code, Diagnostic, SourceId, SourceMap, Span, code};
use serde::de::DeserializeOwned;
use toml::de::{DeTable, DeValue};

/// Parses one configuration file into `T`, registering `name`'s text in `sources` first so that a
/// refusal can point at the line that caused it.
///
/// [`parse_with_warnings`] with its warnings dropped, for a caller that types one file and reports
/// nothing but a refusal. A dropped warning loses no boundary: the empty header it would have
/// named sets nothing either way.
///
/// # Errors
///
/// As [`parse_with_warnings`].
pub fn parse<T: DeserializeOwned>(
    sources: &mut SourceMap,
    name: &str,
    text: &str,
) -> (SourceId, Result<T, Diagnostic>) {
    let (id, parsed) = parse_with_warnings(sources, name, text);
    (id, parsed.map(|(value, _)| value))
}

/// Parses one configuration file into `T` and returns the `W1024` warnings it raised, registering
/// `name`'s text in `sources` first so that a refusal or a warning can point at its line.
///
/// `T` is the typed block tree for a whole file; `toml::Table` is the untyped stand-in, and the
/// difference is exactly whether an unknown key is refused — that is `serde`'s
/// `deny_unknown_fields` and not this function's. What this function adds is the one exception
/// the module doc names: an unknown table opened by a `[header]` line with no key under it at any
/// depth is removed and warned about, one warning per removed table, and the rest is typed again.
/// Only a plain table qualifies. An `[[entry]]` states that the entry exists even when it is empty,
/// as `crate::resolve`'s merge treats it, and `name = {}` is a key with a value.
///
/// The returned [`SourceId`] is the file's, on both paths: a caller resolving an override across an
/// include needs it to name the origin of a value it kept.
///
/// # Errors
///
/// One [`Diagnostic`] for the first thing TOML or `serde` refused — a duplicate key, an unknown
/// one, a value of the wrong type, or a syntax error.
pub fn parse_with_warnings<T: DeserializeOwned>(
    sources: &mut SourceMap,
    name: &str,
    text: &str,
) -> (SourceId, Result<(T, Vec<Diagnostic>), Diagnostic>) {
    let id = sources.add(name, text);
    (id, parse_at(id, text))
}

/// [`parse_with_warnings`] over a file already in the caller's [`SourceMap`] as `id`, which is
/// how a file loaded from its path keeps that path: a server watches every file a compile read
/// by it.
///
/// # Errors
///
/// As [`parse_with_warnings`].
pub fn parse_at<T: DeserializeOwned>(
    id: SourceId,
    text: &str,
) -> Result<(T, Vec<Diagnostic>), Diagnostic> {
    match toml::from_str::<T>(text) {
        Ok(value) => Ok((value, Vec::new())),
        Err(err) => without_empty_unknown_tables(id, text, err),
    }
}

/// The failure path of [`parse_with_warnings`]: while the refusal is an unknown key naming an
/// empty table opened by a header line, remove that table from the parsed document, warn, and type
/// the rest again. Anything else is refused as the first attempt was.
///
/// Each round removes one table, so the loop ends; each clones the document once, which costs one
/// copy of the file's tree per empty unknown header and only on a file that failed to type.
fn without_empty_unknown_tables<T: DeserializeOwned>(
    id: SourceId,
    text: &str,
    first: toml::de::Error,
) -> Result<(T, Vec<Diagnostic>), Diagnostic> {
    let mut err = first;
    let mut document = None;
    let mut warnings = Vec::new();
    while let Some(range) = empty_header_candidate(text, &err) {
        if document.is_none() {
            // The first attempt parsed the same bytes, so this succeeds; on the impossible path
            // the first refusal stands.
            document = DeTable::parse(text).ok();
        }
        let Some(root) = document.as_mut() else {
            break;
        };
        if !remove_empty_table(root.get_mut(), &range) {
            break;
        }
        warnings.push(ignored(id, text, &range, err.message()));
        match T::deserialize(toml::de::Deserializer::from(root.clone())) {
            Ok(value) => return Ok((value, warnings)),
            Err(next) => err = next,
        }
    }
    Err(refusal(id, text, &err))
}

/// The key range of an unknown-key refusal whose key is written on a `[header]` line, or `None`
/// for any other refusal. Whether the table under it is empty is [`remove_empty_table`]'s check.
fn empty_header_candidate(text: &str, err: &toml::de::Error) -> Option<Range<usize>> {
    if !err.message().starts_with("unknown field") {
        return None;
    }
    let range = err.span()?;
    let line = line_at(text, range.start)?.trim_start();
    (line.starts_with('[') && !line.starts_with("[[")).then_some(range)
}

/// Removes the table whose key was written at `at`, anywhere in `table` including inside an
/// `[[entry]]`, when nothing under it sets a key. `true` when it removed one.
fn remove_empty_table(table: &mut DeTable<'_>, at: &Range<usize>) -> bool {
    let mut found = None;
    for (key, value) in table.iter_mut() {
        if key.span() == *at {
            if sets_nothing(value.get_ref()) {
                found = Some(key.get_ref().to_string());
            }
            break;
        }
        let removed = match value.get_mut() {
            DeValue::Table(inner) => remove_empty_table(inner, at),
            DeValue::Array(entries) => {
                entries
                    .as_mut()
                    .iter_mut()
                    .any(|entry| match entry.get_mut() {
                        DeValue::Table(inner) => remove_empty_table(inner, at),
                        _ => false,
                    })
            }
            _ => false,
        };
        if removed {
            return true;
        }
    }
    found.is_some_and(|key| table.remove(key.as_str()).is_some())
}

/// A table with no key under it at any depth: `[a]` alone, or `[a]` and `[a.b]` with nothing
/// written under either.
fn sets_nothing(value: &DeValue<'_>) -> bool {
    match value {
        DeValue::Table(inner) => inner.values().all(|value| sets_nothing(value.get_ref())),
        _ => false,
    }
}

/// The `W1024` warning for the header line at `at`. `refused` is the unknown-key message, whose
/// `expected …` half is the list of names the place accepts.
fn ignored(id: SourceId, text: &str, at: &Range<usize>, refused: &str) -> Diagnostic {
    let header =
        line_at(text, at.start).map_or("", |line| line.split('#').next().unwrap_or(line).trim());
    let mut warning = Diagnostic::warning(
        code::W_EMPTY_UNKNOWN_BLOCK,
        format!("`{header}` is not a block of the configuration, so Novis ignores it"),
    )
    .with_note("there is no setting under it, so it changes nothing".to_string());
    if let Some(span) = span_of(id, Some(at.clone())) {
        warning = warning.with_primary(span, "not a block");
    }
    if let Some((_, expected)) = refused.split_once("expected ") {
        let expected = expected.strip_prefix("one of ").unwrap_or(expected);
        warning = warning.with_note(format!("the names allowed here are {expected}"));
    }
    warning.with_help("delete this line".to_string())
}

/// The refusal for a `toml` failure: the message, its line, and the block it was found in.
fn refusal(id: SourceId, text: &str, err: &toml::de::Error) -> Diagnostic {
    let message = err.message().to_string();
    let range = err.span();
    let mut diagnostic = Diagnostic::error(code_for(&message), message);
    if let Some(span) = span_of(id, range.clone()) {
        diagnostic = diagnostic.with_primary(span, "here");
    }
    if let Some(block) = range.and_then(|range| block_at(text, range.start)) {
        diagnostic = diagnostic.with_note(format!("in `{block}`"));
    }
    diagnostic
}

/// The whole line `offset` is on, without its line break.
fn line_at(text: &str, offset: usize) -> Option<&str> {
    let start = text.get(..offset)?.rfind('\n').map_or(0, |at| at + 1);
    text.get(start..)?.lines().next()
}

/// Which stable code a `toml` failure is reported under.
///
/// The `toml` crate gives one error type for every refusal and no discriminant, so the message is
/// the only thing to read: a duplicate is its own code because it is a distinct operator mistake
/// with a distinct fix, while an unknown key, a bad value and a syntax error are all
/// `E_BAD_DIRECTIVE`, whose own doc comment already covers "does not exist, or an invalid value"
/// (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` names that code for the unknown key).
fn code_for(message: &str) -> Code {
    if message.starts_with("duplicate key") {
        code::E_DUPLICATE_DIRECTIVE
    } else {
        code::E_BAD_DIRECTIVE
    }
}

/// The header of the block `offset` is written inside — `[limits.hard]`, `[[app]]` — or `None` when
/// it is in the root table.
///
/// A refusal names the *line*, which `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`
/// requires and `serde` supplies for free. It does
/// not name the **block**, and that is the half an operator needs: an unknown key is nearly always a
/// key written under the wrong header, and `unknown field \`memory\`` reads as nonsense until you
/// know it was found under `[metrics]`. `serde` cannot supply it, because by the time a field is
/// rejected the deserializer knows only the struct it was rejected by and not what the file called
/// it — so the answer comes from the text, by scanning back to the nearest header line.
///
/// It is a scan of the file so far, run only on the failure path, so a boot that succeeds pays
/// nothing for it.
///
/// A key written on a header line is part of that header, which the snippet already shows, so it
/// has no block to name: `[capabilties]` and the `local` of `[cache.local]` give `None`.
fn block_at(text: &str, offset: usize) -> Option<&str> {
    if line_at(text, offset)?.trim_start().starts_with('[') {
        return None;
    }
    text.get(..offset)?
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with('['))
        .map(|line| line.split('#').next().unwrap_or(line).trim())
}

/// The failure's byte range as a span in `id`, dropped rather than clamped when it does not fit —
/// a diagnostic with no label still names the file and the message, which is strictly better than
/// one pointing at the wrong line.
fn span_of(id: SourceId, range: Option<Range<usize>>) -> Option<Span> {
    let range = range?;
    let start = u32::try_from(range.start).ok()?;
    let end = u32::try_from(range.end).ok()?;
    Some(Span::new(id, start, end))
}
