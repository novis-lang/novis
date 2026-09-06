//! The canonical UUID text, read at the bottom of the crate tree: RFC 9562
//! § 4's `8-4-4-4-12` hyphenated form, and the sixteen octets it spells.
//!
//! # Why the grammar is here and not beside the class
//!
//! `Core\Uuid` is `nvs_stdlib::uuid`'s — the class, its slot layout, its
//! members and every reason its reader is strict. What is *not* that class's is
//! the question [`crate::routes`] asks:
//! `rule:routing/a-capture-narrows-to-a-closed-set`
//! lets a route capture declare `Core\Uuid`, and a capture is accepted or
//! refused in this crate, one *below* the one that owns the type. That module's
//! gap 2 states the choice: the sixteen-byte parse sits down here, or a
//! declared narrowing goes unenforced. It sits here,
//! for the reason [`crate::decimal`] does — a matcher that cannot
//! perform a conversion matches a segment the declared type would have
//! refused, and a route table that admits what its own signature rejects is
//! § 5's narrowing stated and not kept.
//!
//! **One reader, not two.** `nvs_stdlib::uuid`'s own `parse`
//! reads through [`read`], so a segment a route accepts and a string a program
//! parses are one grammar with one home — which is the rule [`crate::routes`]'
//! gap 4 keeps for percent-decoding and this module's own arm keeps for
//! `decimal`.
//!
//! **What it spends:** nothing per request beyond the sixteen bytes a matched
//! capture holds, which are smaller than the 36-character text they replace.
//! The class, the rendering and every member stay one crate up: what lives here
//! is a length check and a call.

use uuid::Uuid;

/// The sixteen octets `text` spells, or `None` where it is not RFC 9562 § 4's
/// canonical hyphenated form.
///
/// The length check is what narrows the `uuid` crate's accepted spellings
/// to the one Novis reads — the unhyphenated 32 characters, the `{…}` braces
/// and the `urn:uuid:` prefix are each a second spelling of one value, and
/// `nvs_stdlib::uuid`'s module doc is the home of why that is refused rather
/// than repaired. Nothing about the 36 characters that remain is decided here:
/// the crate's own reader decides all of it, including that every 128-bit
/// pattern is a UUID.
///
/// The octets are in the order the canonical text writes them, which is what
/// makes them the argument `nvs_stdlib::uuid::of_octets` already takes.
#[must_use]
pub fn read(text: &str) -> Option<[u8; 16]> {
    (text.len() == uuid::fmt::Hyphenated::LENGTH)
        .then(|| Uuid::try_parse(text).ok())
        .flatten()
        .map(Uuid::into_bytes)
}

#[cfg(test)]
mod tests {
    use super::read;

    /// The octets are the text's own order, big-endian and un-permuted, because
    /// that is the contract `nvs_stdlib::uuid::of_octets` is written against —
    /// a `Core\Uuid` built from these has to render as the segment that arrived.
    #[test]
    fn the_octets_are_the_canonical_texts_own_order() {
        assert_eq!(
            read("00112233-4455-6677-8899-aabbccddeeff"),
            Some([
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff
            ])
        );
    }

    /// Both halves of the narrowing, named together: either case parses, and the
    /// non-canonical spellings of a UUID the `uuid` crate would otherwise
    /// take do not. A reader that accepted everything would pass the first half
    /// alone, which is the whole reason a capture can refuse.
    #[test]
    fn the_canonical_form_parses_in_either_case_and_the_other_spellings_do_not() {
        let upper = read("0FB0BC5C-4E24-4E4E-8E1E-6A6B8A0E1B2C").expect("a UUID");
        let lower = read("0fb0bc5c-4e24-4e4e-8e1e-6a6b8a0e1b2c").expect("a UUID");
        assert_eq!(upper, lower);

        for text in [
            "0fb0bc5c4e244e4e8e1e6a6b8a0e1b2c",
            "{0fb0bc5c-4e24-4e4e-8e1e-6a6b8a0e1b2c}",
            "urn:uuid:0fb0bc5c-4e24-4e4e-8e1e-6a6b8a0e1b2c",
            "0fb0bc5c-4e24-4e4e-8e1e-6a6b8a0e1b2",
            "",
        ] {
            assert!(read(text).is_none(), "{text} is not the canonical form");
        }
    }
}
