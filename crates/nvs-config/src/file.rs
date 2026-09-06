//! Reading one file of the configuration: TOML in, a deserialized block tree out, and a refusal
//! that carries a span rather than a byte offset.
//!
//! ADR 0064 § 1 fixes the format as TOML read through `serde`, and § 3 makes both a duplicate key
//! and an unknown one an error *per file*: in a root-owned file where one table grants
//! capabilities, an assignment silently shadowed by a later copy of itself is a security-relevant
//! failure that costs nothing to refuse. Neither refusal is machinery of ours — the `toml` crate
//! refuses the duplicate and `deny_unknown_fields` refuses the unknown key — so what this module
//! owns is the *reporting*: the file is registered in the caller's [`SourceMap`] before it is
//! parsed, so a failure renders through `rule:errors/diagnostic-record`'s one diagnostic record with the offending line
//! under it, exactly as a compiler error does — plus the one thing `serde` cannot say, which is
//! which **block** the key was found in ([`block_at`]).
//!
//! The typed tree is [`crate::tree::Config`]; this function stays generic over it so a caller
//! wanting the raw table still has one.
//!
//! Across an `[[include]]` the same key set twice is an override and not a duplicate, which is
//! [`crate::directive`]'s neighbour and ADR 0103 § 3's; this module answers about one file.
//!
//! Cost: the file's text is held once in the `SourceMap` for as long as the caller keeps it, which
//! is what buys the snippet under the diagnostic. Nothing here runs per request.

use std::ops::Range;

use nvs_diagnostics::{Code, Diagnostic, SourceId, SourceMap, Span, code};
use serde::de::DeserializeOwned;

/// Parses one configuration file into `T`, registering `name`'s text in `sources` first so that a
/// refusal can point at the line that caused it.
///
/// `T` is the typed block tree for a whole file; `toml::Table` is the untyped stand-in, and the
/// difference is exactly whether an unknown key is refused — that is `serde`'s
/// `deny_unknown_fields` and not this function's.
///
/// The returned [`SourceId`] is the file's, on both paths: a caller resolving an override across an
/// include needs it to name the origin of a value it kept.
///
/// # Errors
///
/// One [`Diagnostic`] for the first thing TOML or `serde` refused — a duplicate key, an unknown
/// one, a value of the wrong type, or a syntax error.
pub fn parse<T: DeserializeOwned>(
    sources: &mut SourceMap,
    name: &str,
    text: &str,
) -> (SourceId, Result<T, Diagnostic>) {
    let id = sources.add(name, text);
    let parsed = toml::from_str::<T>(text).map_err(|err| {
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
    });
    (id, parsed)
}

/// Which stable code a `toml` failure is reported under.
///
/// The `toml` crate gives one error type for every refusal and no discriminant, so the message is
/// the only thing to read: a duplicate is its own code because it is a distinct operator mistake
/// with a distinct fix, while an unknown key, a bad value and a syntax error are all
/// `E_BAD_DIRECTIVE`, whose own doc comment already covers "does not exist, or an invalid value"
/// (ADR 0064 § 3 names that code for the unknown key).
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
/// A refusal names the *line*, which [ADR 0064 § 3] requires and `serde` supplies for free. It does
/// not name the **block**, and that is the half an operator needs: an unknown key is nearly always a
/// key written under the wrong header, and `unknown field \`memory\`` reads as nonsense until you
/// know it was found under `[metrics]`. `serde` cannot supply it, because by the time a field is
/// rejected the deserializer knows only the struct it was rejected by and not what the file called
/// it — so the answer comes from the text, by scanning back to the nearest header line.
///
/// It is a scan of the file so far, run only on the failure path, so a boot that succeeds pays
/// nothing for it.
///
/// [ADR 0064 § 3]: ../../../docs/adr/0064-configuration-file-format.md
fn block_at(text: &str, offset: usize) -> Option<&str> {
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
