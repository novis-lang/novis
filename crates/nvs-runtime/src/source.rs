//! `rule:errors/a-record-names-where-it-was-produced`'s carrier: the bytes a
//! producer's own call site was compiled with, and the [`Source`] they decode
//! back to.
//!
//! A record says where it was produced, and the compiler is the only thing that
//! knows. `nvs_ir::lower::Lowering::source` derives that datum once, an
//! `nvs_ir::ir::InstKind::SourceConst` carries it, `nvs-codegen` bakes
//! [`encode`]'s bytes into the unit's own data section, and the producer is
//! handed their address as one more operand of the fixed
//! `rule:errors/propagation` signature. [`decode`] is what it reads back.
//!
//! **The format lives here because it is one agreement with two ends that
//! cannot see each other** — `nvs-codegen` writes it and `nvs-stdlib` reads it,
//! and this crate is below both. A second spelling of it is exactly the failure
//! the rule exists to rule out.
//!
//! **What it spends:** one read-only object per producer call site in the
//! artifact, and nothing on the heap until a record is actually built. A file
//! name repeats across the producers in a unit, which is bytes in a data
//! section and no run-time cost at all. A path that produces no record pays for
//! none of it, which is where `rule:errors/propagation` already puts this cost.

use std::ptr;
use std::slice;

use nvs_render::Source;

use crate::Value;

/// The bytes ahead of the two strings: the one-based line, the file's length
/// and the member's, each a little-endian `u32`, with the file's bytes and then
/// the member's laid out immediately after.
///
/// Little-endian rather than the host's order because the unit that carries a
/// blob and the runtime that reads it are two builds, and one fixed order means
/// they cannot disagree about which. The swap, where a machine needs one, is
/// paid at a producer's call and nowhere else.
const HEADER: usize = 12;

/// `u32::MAX` in the member's length slot is **no member** — a producer at file
/// scope. That is not the same fact as a member whose name is empty, and the
/// envelope omits an absent field rather than rendering it, so the two cannot
/// share a spelling.
const NO_MEMBER: u32 = u32::MAX;

/// The blob `nvs-codegen` bakes for one producer call site.
///
/// # Panics
///
/// Panics on a file name or member label past `u32::MAX` bytes, which is far
/// more than a source file this compiler read can hold.
#[must_use]
pub fn encode(source: &Source) -> Vec<u8> {
    let file = source.file.as_bytes();
    let member = source.member.as_deref().map(str::as_bytes);
    let mut out = Vec::with_capacity(HEADER + file.len() + member.map_or(0, <[u8]>::len));
    out.extend_from_slice(&source.line.to_le_bytes());
    out.extend_from_slice(&length(file).to_le_bytes());
    out.extend_from_slice(&member.map_or(NO_MEMBER, length).to_le_bytes());
    out.extend_from_slice(file);
    if let Some(member) = member {
        out.extend_from_slice(member);
    }
    out
}

/// One string's length as the header holds it.
fn length(bytes: &[u8]) -> u32 {
    u32::try_from(bytes.len()).expect("a name far longer than a source file can hold")
}

/// The [`Source`] `blob` carries, or `None` for the zero word — the producer
/// that was handed no call site, whose field the envelope omits.
///
/// # Safety
///
/// `blob` is null or an address [`encode`]'s bytes were baked at, readable for
/// the whole blob's length. A unit's data section is exactly that and outlives
/// every request served from it, which is why nothing here copies less than the
/// strings it returns.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the blob's liveness is the caller's obligation to state — it is a compiled unit's own data section"
)]
pub unsafe fn decode(blob: *const u8) -> Option<Source> {
    if blob.is_null() {
        return None;
    }
    // SAFETY: the caller's contract is that `blob` addresses `encode`'s bytes,
    // whose header is the first `HEADER` of them. `read_unaligned` because
    // nothing asked the data section for an alignment the header's `u32`s would
    // want, and a byte array has none to violate.
    let header: [u8; HEADER] = unsafe { ptr::read_unaligned(blob.cast::<[u8; HEADER]>()) };
    let line = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let file_len = usize::try_from(u32::from_le_bytes([
        header[4], header[5], header[6], header[7],
    ]))
    .ok()?;
    let member_len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    // SAFETY: the same contract — the file's bytes follow the header, and its
    // length is what the header just said.
    let file = unsafe { slice::from_raw_parts(blob.add(HEADER), file_len) };
    let member = if member_len == NO_MEMBER {
        None
    } else {
        let len = usize::try_from(member_len).ok()?;
        // SAFETY: and the member's bytes follow the file's, by the same
        // contract and the same header.
        let bytes = unsafe { slice::from_raw_parts(blob.add(HEADER + file_len), len) };
        Some(String::from_utf8_lossy(bytes).into_owned())
    };
    Some(Source {
        file: String::from_utf8_lossy(file).into_owned(),
        line,
        member,
    })
}

/// The [`Source`] a producer's argument 0 carries, or `None` where that slot
/// holds the zero word — the producer the compiler had no call site for, whose
/// field the envelope omits rather than rendering empty.
///
/// The one reader a `Core` member wants: `nvs_stdlib::registry`'s
/// `SOURCE_MEMBERS` is what puts the operand there, and this is what turns it
/// back into the datum, so no producer spells the two steps for itself.
///
/// # Safety
///
/// `operand` is a slot a `nvs_ir::ir::InstKind::SourceConst` materialized —
/// [`encode`]'s bytes in the compiled unit's own data section, which outlives
/// every request served from that unit, or the zero word.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the blob's liveness is the caller's obligation to state, and `decode`'s contract is the whole of it"
)]
pub unsafe fn of_operand(operand: Value) -> Option<Source> {
    // SAFETY: forwarding the caller's own contract, narrowed by the tag check
    // — a slot that is not an engine-owned address answers `None` before any
    // read happens at all.
    unsafe { decode(operand.as_source_const()?) }
}

/// The `Throwable::$location` spelling of a [`Source`]: the file as the program
/// named it and its one-based line, `file:line`.
///
/// The property is a *position*, and the enclosing member is deliberately not
/// repeated in it — the record's envelope carries all three parts beside it,
/// and every frame label the backtrace collects already reads
/// `Class::member() at file:line`. What the rule asks for is one construction
/// rather than one spelling, and this renders the constant the compiler baked
/// at the throw.
#[must_use]
pub fn location(source: &Source) -> String {
    format!("{}:{}", source.file, source.line)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    use nvs_render::Source;

    /// The datum survives the section it was baked into: one construction, two
    /// ends, and the rule's own reason for there being only one.
    #[test]
    fn a_source_round_trips_through_the_bytes_a_unit_bakes() {
        let source = Source {
            file: "app/Http/Handler.nvs".to_owned(),
            line: 118,
            member: Some("Handler::respond".to_owned()),
        };
        let bytes = encode(&source);
        // SAFETY: the blob is this frame's own, and outlives the call.
        #[expect(unsafe_code, reason = "reading back what this test just baked")]
        let back = unsafe { decode(bytes.as_ptr()) };
        assert_eq!(back, Some(source));
    }

    /// A file-scope producer has no member, and that is a different answer from
    /// a member whose name is empty — the two share no spelling.
    #[test]
    fn a_file_scope_producer_decodes_to_no_member_and_an_empty_name_does_not() {
        let scope = Source {
            file: "script.nvs".to_owned(),
            line: 1,
            member: None,
        };
        let empty = Source {
            member: Some(String::new()),
            ..scope.clone()
        };
        let (baked_scope, baked_empty) = (encode(&scope), encode(&empty));
        #[expect(unsafe_code, reason = "reading back what this test just baked")]
        // SAFETY: both blobs are this frame's own, and outlive the calls.
        let (read_scope, read_empty) =
            unsafe { (decode(baked_scope.as_ptr()), decode(baked_empty.as_ptr())) };
        assert_eq!(read_scope, Some(scope));
        assert_eq!(read_empty, Some(empty));
    }

    /// The zero word is the producer that was handed no call site at all, and
    /// it decodes to nothing rather than to an empty source.
    #[test]
    fn the_zero_word_decodes_to_nothing() {
        #[expect(unsafe_code, reason = "null is in `decode`'s contract")]
        // SAFETY: null is the one address the contract admits without bytes.
        let read = unsafe { decode(std::ptr::null()) };
        assert_eq!(read, None);
    }
}
