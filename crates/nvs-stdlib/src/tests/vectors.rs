//! The frozen WebCrypto vector set, read once and reached as JSON — the one
//! door onto `crates/nvs-stdlib/tests/vectors/webcrypto.json`.
//!
//! `bun nv webcrypto-vectors` (`tools/nv/cmd/webcrypto-vectors.ts`) writes that
//! file from `crypto.subtle`, the W3C API a browser ships, and its own header is
//! the home of how and of what makes it reproducible. What belongs here is why
//! the tests read the file rather than carry its octets: a vector copied into a
//! `#[test]` is a second copy of a number whose first copy is what the other
//! implementation actually produced, and the copy is the one that goes stale.
//! Every interop section —
//! the key agreements, and the tokens `Core\Jwe` and `Core\Jwt` are held to —
//! is asserted against the file itself, so regenerating it is what changes a
//! test's expectations and nothing else is.
//!
//! A vector published *with a specification* is the other kind of evidence and
//! stays inline where it is asserted: RFC 7748's exchange and RFC 5869's
//! derivations are printed in their documents, so a reader checks them against
//! the document rather than against this tree.
//!
//! It lives under `crate::tests` for `granting`'s reason, which
//! `crates/nvs-stdlib/tests/capability.rs`'s own case states: that scan stops at
//! a file's first `#[cfg(test)]` and asserts there is one, so a helper every
//! module's cases share is declared inside the crate root's test module rather
//! than beside the modules that ship. The JSON is therefore compiled into the
//! test binary alone.
//!
//! A test reads one part of the set, such as `jws/keys`, and only that part is
//! parsed, because Miri runs each test in a process of its own and a parse of
//! the whole set is a large share of a vector test's time there. A part is
//! found by its key line in the text, which `JSON.stringify` indents by two
//! spaces a level. Splitting the set with `serde_json`'s `RawValue` costs more
//! under Miri than the whole parse does.
//! [`tests::every_part_reads_as_the_whole_parse_does`] fails when the writer's
//! layout changes.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

/// The set as it sits in the tree, compiled into the test binary.
const SOURCE: &str = include_str!("../../tests/vectors/webcrypto.json");

/// The text of the member `key` of the object `within`, whose members are
/// written `indent` spaces in. The member's value is an object or an array.
fn member(within: &'static str, indent: usize, key: &str) -> Option<&'static str> {
    let pad = " ".repeat(indent);
    let head = format!("\n{pad}\"{key}\": ");
    let rest = &within[within.find(&head)? + head.len()..];
    if rest.starts_with("[]") || rest.starts_with("{}") {
        return Some(&rest[..2]);
    }
    let close = match rest.as_bytes().first()? {
        b'[' => ']',
        b'{' => '}',
        _ => return None,
    };
    let end = rest.find(&format!("\n{pad}{close}"))? + 1 + indent + 1;
    Some(&rest[..end])
}

/// One `section/kind` part of the set, parsed on its first use and kept for
/// the process.
fn part(section: &str, kind: &str) -> Option<&'static Value> {
    static PARTS: OnceLock<Mutex<HashMap<(String, String), &'static Value>>> = OnceLock::new();
    let mut parts = PARTS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let at = (section.to_owned(), kind.to_owned());
    if let Some(&value) = parts.get(&at) {
        return Some(value);
    }
    let text = member(member(SOURCE, 2, section)?, 4, kind)?;
    let value: Value = serde_json::from_str(text).expect("bun nv webcrypto-vectors writes JSON");
    let value: &'static Value = Box::leak(Box::new(value));
    parts.insert(at, value);
    Some(value)
}

/// A section's vectors — the cases where an operation has an answer.
pub(crate) fn vectors(section: &str) -> &'static [Value] {
    list(section, "vectors")
}

/// A section's refusals — the cases where WebCrypto itself refused, each of
/// which this runtime has to refuse too.
pub(crate) fn refusals(section: &str) -> &'static [Value] {
    list(section, "refusals")
}

/// A section's signing cases — the ones where this runtime writes the octets
/// and WebCrypto's own output from the same inputs is what they are held to.
pub(crate) fn signs(section: &str) -> &'static [Value] {
    list(section, "signs")
}

/// One array out of one section, or a panic naming what was asked for: a test
/// reaching a section the script does not write is a test asserting nothing,
/// which is worse than a failing one.
fn list(section: &str, kind: &str) -> &'static [Value] {
    part(section, kind)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("the vector set has no {section}/{kind} array"))
        .as_slice()
}

/// One node of the set by its pointer from the root, or a panic naming it.
///
/// The door onto what a case *names* rather than carries: a signature vector
/// says which key it was made under, so the key material sits once under
/// `/jws/keys` and every case that uses it agrees with every other by
/// construction.
///
/// The pointer names a section and a kind first, as `/jws/keys/rsa-1` does.
pub(crate) fn node(pointer: &str) -> &'static Value {
    let mut tokens = pointer.splitn(4, '/');
    let found = match (tokens.next(), tokens.next(), tokens.next(), tokens.next()) {
        (Some(""), Some(section), Some(kind), rest) => part(section, kind)
            .and_then(|value| rest.map_or(Some(value), |rest| value.pointer(&format!("/{rest}")))),
        _ => None,
    };
    found.unwrap_or_else(|| panic!("the vector set has no node at {pointer}"))
}

/// A hex field of one case, as the octets it stands for.
///
/// `pointer` is RFC 6901's, so a nested field is `/a/raw` and a top-level one is
/// `/secret` — the set nests a key pair under the party that holds it.
pub(crate) fn octets(case: &Value, pointer: &str) -> Vec<u8> {
    data_encoding::HEXLOWER
        .decode(text(case, pointer).as_bytes())
        .expect("the set writes every octet string in lower-case hex")
}

/// A whole-number field of one case, by the same pointer [`octets`] takes.
///
/// The set writes a count as a number rather than as text — an iteration count
/// is the one field a reader compares against a bound rather than against
/// octets.
pub(crate) fn number(case: &Value, pointer: &str) -> u64 {
    case.pointer(pointer)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("the vector set's case has no whole number at {pointer}"))
}

/// A text field of one case, by the same pointer [`octets`] takes.
pub(crate) fn text<'a>(case: &'a Value, pointer: &str) -> &'a str {
    case.pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("the vector set's case has no text at {pointer}"))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    /// Every `section/kind` part that [`super::part`] finds by its key line is
    /// the part a parse of the whole set has at that place.
    #[test]
    #[cfg_attr(
        miri,
        ignore = "the whole parse is the cost this module avoids under Miri"
    )]
    fn every_part_reads_as_the_whole_parse_does() {
        let whole: Value = serde_json::from_str(super::SOURCE).expect("the set is JSON");
        let mut parts = 0;
        for (section, kinds) in whole.as_object().expect("the set is an object") {
            let Some(kinds) = kinds.as_object() else {
                continue;
            };
            for (kind, value) in kinds {
                assert_eq!(
                    super::part(section, kind),
                    Some(value),
                    "{section}/{kind} reads as the whole parse does"
                );
                parts += 1;
            }
        }
        assert!(parts > 10, "the set has its parts, {parts} found");
    }
}
