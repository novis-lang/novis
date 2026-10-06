//! What an `[[extension]]` entry still owes once it has deserialized:
//! `rule:packaging/extension-loading-is-root-controlled`'s `path` and `sha256`, both present, and a
//! pin that is 64 hexadecimal digits. At boot and at every reload.
//!
//! **The two keys are typed optional in [`Extension`] and required here**, because a refusal has
//! to name the entry's file and line, and a `serde` "missing field" names neither the entry nor,
//! for an entry an include appended, the file. This pass runs over the merged tree, finds the
//! file that wrote each entry in the merge's origins, and points at that entry's own
//! `[[extension]]` header — or at its `sha256` line when the pin is the part that is wrong.
//!
//! **What it does not check is the file.** Whether the digest matches, whether the component
//! validates and what its manifest says are the loader's (`nvs-ext`), which reads the bytes. This
//! pass reads only the configuration, so `nvs check` and a reload's dry pass refuse an unpinned
//! entry without touching the disk.
//!
//! A pin is accepted in either case, since `sha256sum` prints lower case and PowerShell's
//! `Get-FileHash` upper case; the loader compares it case-insensitively. The optional `memory`
//! ceiling is read as a size, `[limits] memory`'s unit, so a value that is not one is the same
//! `E0601` it is there.
//!
//! Cost: one pass over the `[[extension]]` array at boot and at reload, and a scan of one file's
//! text on the refusal path only. Nothing here runs on a request path.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, SourceMap, Span, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Extension};
use crate::value::{Quantity, Unit};

/// Every `[[extension]]` entry in the merged tree is complete, or the first that is not is refused.
///
/// `origins` names the file each entry's keys were written in, keyed `extension.<index>.<key>` as
/// the merge records them, and `sources` holds that file's text so the refusal can point at its line. An empty
/// map leaves the file and the line off and still refuses.
///
/// # Errors
///
/// `E0651` for an entry with no `path`, no `sha256`, or a `sha256` that is not 64 hexadecimal
/// digits. `E0601` for a `memory` that is not a size.
pub fn validate(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    sources: &SourceMap,
) -> Result<(), Diagnostic> {
    for (index, entry) in config.extension.iter().enumerate() {
        check(index, entry, origins, sources)?;
    }
    Ok(())
}

/// Whether `pin` is a SHA-256 digest as text: exactly 64 hexadecimal digits, either case.
#[must_use]
pub fn is_pin(pin: &str) -> bool {
    pin.len() == 64 && pin.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One entry's refusal, or none.
fn check(
    index: usize,
    entry: &Extension,
    origins: &BTreeMap<String, Origin>,
    sources: &SourceMap,
) -> Result<(), Diagnostic> {
    let key = format!("extension.{index}");
    let written_in = written_in(index, origins);
    let at = |line: Option<&str>| locate(index, line, origins, sources);

    let Some(path) = entry.path.as_deref().filter(|path| !path.is_empty()) else {
        return Err(refuse(
            "This `[[extension]]` entry has no `path`".to_owned(),
            written_in,
            at(None),
            "write the `.nvsx` file to load, as `path = \"geo.nvsx\"`",
        ));
    };
    let Some(pin) = entry.sha256.as_deref() else {
        return Err(refuse(
            format!("The extension `{path}` has no `sha256`"),
            written_in,
            at(None),
            "write the file's SHA-256 as 64 hexadecimal digits, as `sha256 = \"9f86…0a08\"`",
        ));
    };
    if !is_pin(pin) {
        return Err(refuse(
            format!(
                "The `sha256` of the extension `{path}` is not 64 hexadecimal digits: it has {} \
                 characters",
                pin.chars().count()
            ),
            written_in,
            at(Some("sha256")),
            "write the file's SHA-256 as 64 hexadecimal digits",
        ));
    }
    if let Some(memory) = entry.memory.as_ref() {
        let memory_key = format!("{key}.memory");
        Quantity::parse(&memory_key, Unit::Bytes, memory)
            .map_err(|invalid| invalid.diagnostic(origins.get(&memory_key).or(written_in)))?;
    }
    Ok(())
}

/// The refusal, in one phrasing for every incomplete entry.
fn refuse(
    message: String,
    written_in: Option<&Origin>,
    span: Option<Span>,
    help: &str,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(code::E_BAD_EXTENSION_ENTRY, message)
        .with_note(format!(
            "every extension needs a path and the SHA-256 of its file, and nothing is loaded \
             without both{}",
            origin_note(written_in)
        ))
        .with_help(help.to_owned());
    if let Some(span) = span {
        diagnostic = diagnostic.with_primary(span, "this entry");
    }
    diagnostic
}

/// The file entry `index` was written in. The merge records an origin for each key an entry
/// writes and none for the entry itself, so this is the origin of its first key; an entry that
/// writes no key at all has none.
fn written_in(index: usize, origins: &BTreeMap<String, Origin>) -> Option<&Origin> {
    let prefix = format!("extension.{index}.");
    origins
        .range(prefix.clone()..)
        .take_while(|(key, _)| key.starts_with(&prefix))
        .map(|(_, origin)| origin)
        .next()
}

/// The span of entry `index`'s header line in the file that wrote it, or of its `key` line inside
/// that entry when `key` is given and written there.
///
/// The merge keeps a file per entry and no offset, so the entry is found by counting: it is the
/// `n`th `[[extension]]` header in its file, where `n` is how many earlier entries came from the
/// same file. `None` when the file is not in `sources` or the entry was written another way — an
/// inline array — and the refusal then names the file alone.
fn locate(
    index: usize,
    key: Option<&str>,
    origins: &BTreeMap<String, Origin>,
    sources: &SourceMap,
) -> Option<Span> {
    let origin = written_in(index, origins)?;
    let ordinal = (0..index)
        .filter(|earlier| {
            written_in(*earlier, origins).is_some_and(|other| other.path == origin.path)
        })
        .count();
    let text = sources.get(origin.source)?.text();

    let mut offset = 0;
    let mut headers = 0;
    let mut header = None;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // The next header ends the entry, so a `key` it did not write is answered by the header.
            if header.is_some() {
                break;
            }
            if trimmed.starts_with("[[extension]]") {
                if headers == ordinal {
                    header = line_span(origin, start, line);
                    if key.is_none() {
                        break;
                    }
                }
                headers += 1;
            }
        } else if header.is_some()
            && let Some(key) = key
            && trimmed
                .strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        {
            return line_span(origin, start, line);
        }
    }
    header
}

/// `line`, starting at byte `start` of `origin`'s file, as a span without its line ending.
fn line_span(origin: &Origin, start: usize, line: &str) -> Option<Span> {
    let end = start + line.trim_end_matches(['\n', '\r']).len();
    Some(Span::new(
        origin.source,
        u32::try_from(start).ok()?,
        u32::try_from(end).ok()?,
    ))
}
