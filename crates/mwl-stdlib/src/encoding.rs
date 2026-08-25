//! `Core\Encoding` — docs/spec/01-core-library.md § 7, the members that sit
//! exactly on the `bytes`↔`string` boundary
//! ([ADR 0009](../../../../docs/adr/0009-string-and-bytes.md)).
//!
//! Everything this class does is a *representation* change: the same
//! information, spelled as text a person or a protocol can carry, or spelled
//! back as the octets it came from. That is why it is one class rather than a
//! member on each of `Core\Str` and `Core\Bytes` — the operation belongs to
//! neither type, it belongs to the seam between them.
//!
//! # What is registered here so far
//!
//! `toHex`/`fromHex`, `toBase64`/`fromBase64`, `toBase64Url`/`fromBase64Url`
//! and `toBase32`/`fromBase32`. § 7's table also writes the
//! `encodeText`/`decodeText`/`isValidText` trio over a `Charset`; that is
//! unwritten, and `docs/agent/handoff.md` names when it lands.
//!
//! # The four base64 members are two alphabets and two padding rules
//!
//! RFC 4648 defines one algorithm and two alphabets, and the wild has settled
//! on a different padding convention for each. MWL writes what each side
//! actually produces rather than a switch:
//!
//! | member | alphabet | padding |
//! |---|---|---|
//! | `toBase64` / `fromBase64` | `A-Za-z0-9+/` (§ 4) | **required** — `base64_encode`'s own output |
//! | `toBase64Url` / `fromBase64Url` | `A-Za-z0-9-_` (§ 5) | **absent** — JWT, and the `rtrim(strtr(…))` idiom the spec row names |
//!
//! Neither decoder accepts the other's spelling, and neither accepts
//! whitespace, a newline or a stray `=`. That is the same rule `fromHex` reads
//! one paragraph down and the same reason: PHP's `base64_decode` in its
//! default mode discards every character outside the alphabet, so a corrupted
//! transfer decodes to a *shorter* value instead of an error, and one truncated
//! signature compares unequal rather than reporting why.
//!
//! **Canonicality is checked too**, which is the part a hand-written sextet
//! loop gets wrong: the final character of a 2- or 3-character group carries
//! bits that no octet reads, so `"aa=="` and `"ab=="` would otherwise decode to
//! the same octet and only one of them round-trips. Both are refused unless the
//! unread bits are zero (RFC 4648 § 3.5).
//!
//! # base32 is one pair, so its decoder is lenient where base64's is not
//!
//! `toBase32` writes RFC 4648 § 6's alphabet **upper case and unpadded**,
//! which is how an `otpauth:` secret is written — TOTP
//! ([ADR 0060](../../../../docs/adr/0060-application-security-protocols.md))
//! being the consumer § 7's row names, since PHP has nothing here to replace.
//!
//! `fromBase32` then reads **either case, and padding that is either canonical
//! or absent**. That looks like the tolerance the two base64 decoders refuse
//! one section up, and it is a different thing:
//!
//! - The base64 refusal is a *disambiguation*. Two members exist, they differ
//!   only in alphabet and padding, and a decoder taking both spellings could
//!   not tell a caller which member they meant. There is one base32 pair, so
//!   there is nothing to be ambiguous with.
//! - Neither variance can change the octets. § 6's alphabet has no lower-case
//!   member, so folding case cannot collide with a symbol; padding carries no
//!   bits. Every rule that *could* change the answer is still enforced —
//!   a symbol outside the alphabet (a space grouping a secret for a reader is
//!   one, and so are the digits `0`, `1` and `8`), a truncated final group,
//!   non-canonical trailing bits, and padding present but wrong.
//!
//! Which is the same distinction the whole module runs on: refuse anything
//! that would silently answer a *different* value, and accept a second
//! spelling of the same one — exactly as `fromHex` reads either case.
//!
//! # Why base64 takes a dependency and hex does not
//!
//! Everything above is why: hex has no alphabet question, no padding, no
//! variant and no non-canonical spelling, so there is nothing for a crate to
//! know. base64 has four such rules, `fromBase64` is a member request bodies
//! reach, and every one of those rules is a documented CVE somewhere. The
//! `base64` crate is the pick under
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4 —
//! pure Rust, no build script, no C, and already in this tree's lock file
//! under `wasmtime-internal-cache`, so it adds no crate at all.
//! `Cargo.toml`'s `[workspace.dependencies]` comment states the pick; this
//! module owns which engine each member is, above.
//!
//! # Why hex takes no dependency
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4 asks
//! two questions of an outside crate, and base-16 answers both the wrong way:
//! the whole algorithm is a nibble table, there is no specification drift to
//! track and no security-relevant parsing to get wrong, so a dependency would
//! buy nothing and add a supply-chain edge to the runtime every request links.
//! **The base64 and base32 rows are a different answer** — their alphabets,
//! padding rules and URL-safe variants are exactly the kind of detail a
//! well-used crate has already gotten wrong once and fixed. base64's is
//! `base64` and base32's is `data-encoding`, both above. This comment is here
//! so nobody reads hex's answer as a rule for the section.
//!
//! # Encoding is total, decoding throws
//!
//! `toHex` cannot fail: every octet has a spelling. `fromHex` is the checked
//! direction ([ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)
//! R4) — it throws on an odd length or a non-hexadecimal character rather than
//! substituting, dropping or truncating, which is the same reason ADR 0009 § 3
//! refuses `iconv`'s `//IGNORE`. A caller who wants the question without the
//! throw asks it of the text before converting.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use data_encoding::{BASE32, BASE32_NOPAD};
use mwl_runtime::{Fault, MwlStr, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Encoding`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Encoding";

/// `Core\Encoding`'s registry rows — § 7's hex pair, its base64 family and its
/// base32 pair.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "toBase64",
            params: &[CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_encoding_to_base64",
        },
        CoreMethod {
            name: "fromBase64",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_encoding_from_base64",
        },
        CoreMethod {
            name: "toBase64Url",
            params: &[CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_encoding_to_base64_url",
        },
        CoreMethod {
            name: "fromBase64Url",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_encoding_from_base64_url",
        },
        CoreMethod {
            name: "toBase32",
            params: &[CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_encoding_to_base32",
        },
        CoreMethod {
            name: "fromBase32",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_encoding_from_base32",
        },
        CoreMethod {
            name: "toHex",
            params: &[CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_encoding_to_hex",
        },
        CoreMethod {
            name: "fromHex",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_encoding_from_hex",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_encoding_to_base64" => (mwl_core_encoding_to_base64 as *const ()).cast(),
        "mwl_core_encoding_from_base64" => (mwl_core_encoding_from_base64 as *const ()).cast(),
        "mwl_core_encoding_to_base64_url" => (mwl_core_encoding_to_base64_url as *const ()).cast(),
        "mwl_core_encoding_from_base64_url" => {
            (mwl_core_encoding_from_base64_url as *const ()).cast()
        }
        "mwl_core_encoding_to_base32" => (mwl_core_encoding_to_base32 as *const ()).cast(),
        "mwl_core_encoding_from_base32" => (mwl_core_encoding_from_base32 as *const ()).cast(),
        "mwl_core_encoding_to_hex" => (mwl_core_encoding_to_hex as *const ()).cast(),
        "mwl_core_encoding_from_hex" => (mwl_core_encoding_from_hex as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Reading arguments
// ============================================================================

/// The `bytes` in argument slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the slot carries another tag:
/// `mwl_types` already checked the declared type and compiled code wrote the
/// tag, so a mismatch is a runtime-contract violation rather than anything a
/// program can cause — the same treatment [`crate::uuid`] gives a `string`.
fn bytes_of<'a>(args: &'a [Value], member: &str) -> Result<&'a [u8], Fault> {
    args[0].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Encoding::{member} expected a `bytes`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

/// The `string` in argument slot 0, for [`bytes_of`]'s reason.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    let bytes = args[0].as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Encoding::{member} expected a `string`, got tag {}",
            args[0].tag_byte()
        ))
    })?;
    // A `string` is guaranteed-valid UTF-8 (ADR 0009 § 2), so this cannot fail
    // for anything compiled code produced.
    std::str::from_utf8(bytes)
        .map_err(|_| Fault::fatal(format!("Core\\Encoding::{member} got invalid UTF-8")))
}

/// The nibble `digit` spells, in either case, or `None` for anything else.
const fn nibble(digit: u8) -> Option<u8> {
    Some(match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        b'A'..=b'F' => digit - b'A' + 10,
        _ => return None,
    })
}

/// How much of a rejected operand a throw quotes.
///
/// The operand of a failed `fromHex` is text that arrived from somewhere, so
/// it is the one value here whose size a caller chooses. A message reaches a
/// log, and quoting it whole would let a request pick how many bytes that log
/// gains — [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// is the wider rule.
const SHOWN_CHARS: usize = 32;

/// `text` as it may be quoted back inside a throw message: the first
/// [`SHOWN_CHARS`] characters, with an ellipsis where anything was dropped.
fn shown(text: &str) -> String {
    let mut out: String = text.chars().take(SHOWN_CHARS).collect();
    if text.chars().nth(SHOWN_CHARS).is_some() {
        out.push('…');
    }
    out
}

/// Why `error` refused the text, in the same register [`mwl_core_encoding_from_hex`]
/// uses: what is wrong, and what the member wanted instead.
///
/// `base64`'s own `Display` names a byte value and an offset, which is precise
/// and unreadable; a `Core` throw is read by the person who wrote the call, so
/// it says which rule was broken. `variant` is `"base64"` or `"base64url"` —
/// the two decoders differ only in alphabet and padding, and those are exactly
/// the two things a caller confuses.
fn why_not_base64(error: &base64::DecodeError, variant: &str) -> String {
    match *error {
        base64::DecodeError::InvalidByte(offset, byte) => format!(
            "the byte {byte:#04x} at offset {offset} is not a {variant} character, and \
             whitespace, a newline and a misplaced `=` are each that byte rather than \
             something skipped"
        ),
        base64::DecodeError::InvalidLength(len) => format!(
            "it measures {len} symbols, and a base64 group is 2, 3 or 4 of them — this one \
             is truncated"
        ),
        base64::DecodeError::InvalidLastSymbol(offset, byte) => format!(
            "the final symbol {byte:#04x} at offset {offset} carries bits no octet reads, so \
             it is one of several spellings of the same value and not the canonical one \
             (RFC 4648 § 3.5)"
        ),
        base64::DecodeError::InvalidPadding => match variant {
            "base64url" => "it is padded, and the URL-safe form is written without `=`".to_owned(),
            _ => "its `=` padding is missing or malformed, and the standard form is written \
                  with it"
                .to_owned(),
        },
    }
}

/// Why `error` refused the text, in [`why_not_base64`]'s register.
fn why_not_base32(error: &data_encoding::DecodeError) -> String {
    let at = error.position;
    match error.kind {
        data_encoding::DecodeKind::Symbol => format!(
            "the byte at offset {at} is not one of `A`-`Z` or `2`-`7` — the digits `0`, `1` and \
             `8` are not in this alphabet, and a space grouping a secret for a reader is a byte \
             like any other"
        ),
        data_encoding::DecodeKind::Trailing => format!(
            "the symbol at offset {at} carries bits no octet reads, so it is one of several \
             spellings of the same value and not the canonical one (RFC 4648 § 3.5)"
        ),
        data_encoding::DecodeKind::Padding => format!(
            "the `=` padding at offset {at} is not what a base32 group takes — write it in full \
             or leave it off entirely"
        ),
        data_encoding::DecodeKind::Length => format!(
            "it ends at offset {at} part-way through a group, and a base32 group is 8 symbols — \
             this one is truncated"
        ),
    }
}

// ============================================================================
// The members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::toBase64(bytes $b): string` — replacing `base64_encode`.
    ///
    /// RFC 4648 § 4's alphabet, **padded**, which is byte-for-byte what
    /// `base64_encode` answers. Total: every octet sequence has a spelling.
    fn mwl_core_encoding_to_base64(_ctx, args: [1]) {
        let raw = bytes_of(args, "toBase64")?;
        Ok(Value::str(MwlStr::new(STANDARD.encode(raw).as_bytes())))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::fromBase64(string $s): bytes` — replacing
    /// `base64_decode`, and throwing where that function's default mode
    /// silently discarded whatever it did not recognise.
    ///
    /// Strict on all three counts the module doc lists: § 4's alphabet only,
    /// padding required and canonical, and no unread bits in the last symbol.
    /// A URL-safe operand is refused here rather than accepted as a courtesy —
    /// [`mwl_core_encoding_from_base64_url`] is the member that reads it, and a
    /// decoder that takes both cannot tell a caller which one they meant.
    fn mwl_core_encoding_from_base64(_ctx, args: [1]) {
        let text = text_of(args, "fromBase64")?;
        let raw = STANDARD.decode(text).map_err(|error| {
            Fault::thrown(format!(
                "Core\\Encoding::fromBase64(): \"{}\" is not base64 — {}",
                shown(text),
                why_not_base64(&error, "base64")
            ))
        })?;
        Ok(Value::bytes(MwlStr::new(&raw)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::toBase64Url(bytes $b): string` — replacing the
    /// `rtrim(strtr(base64_encode($b), "+/", "-_"), "=")` idiom, which is what
    /// the spec row names because it is what every PHP codebase writes.
    ///
    /// RFC 4648 § 5's alphabet, **unpadded**: that idiom's `rtrim` is not an
    /// optional flourish, it is what JWT, `Core\Uri` and every other consumer
    /// of a URL-safe spelling expects, and `=` is percent-encoded in a query
    /// string anyway.
    fn mwl_core_encoding_to_base64_url(_ctx, args: [1]) {
        let raw = bytes_of(args, "toBase64Url")?;
        Ok(Value::str(MwlStr::new(URL_SAFE_NO_PAD.encode(raw).as_bytes())))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::fromBase64Url(string $s): bytes` — the other half of
    /// [`mwl_core_encoding_to_base64_url`], strict in the same three ways and
    /// refusing padding rather than tolerating it.
    fn mwl_core_encoding_from_base64_url(_ctx, args: [1]) {
        let text = text_of(args, "fromBase64Url")?;
        let raw = URL_SAFE_NO_PAD.decode(text).map_err(|error| {
            Fault::thrown(format!(
                "Core\\Encoding::fromBase64Url(): \"{}\" is not URL-safe base64 — {}",
                shown(text),
                why_not_base64(&error, "base64url")
            ))
        })?;
        Ok(Value::bytes(MwlStr::new(&raw)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::toBase32(bytes $b): string` — replacing nothing in PHP,
    /// and needed by TOTP
    /// ([ADR 0060](../../../../docs/adr/0060-application-security-protocols.md)).
    ///
    /// RFC 4648 § 6's alphabet, **upper case and unpadded** — the form an
    /// `otpauth:` secret is written in. Total: every octet sequence has a
    /// spelling.
    fn mwl_core_encoding_to_base32(_ctx, args: [1]) {
        let raw = bytes_of(args, "toBase32")?;
        Ok(Value::str(MwlStr::new(BASE32_NOPAD.encode(raw).as_bytes())))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::fromBase32(string $s): bytes` — the other half of
    /// [`mwl_core_encoding_to_base32`], reading either case and padding that is
    /// either canonical or absent.
    ///
    /// The module doc's *base32 is one pair* section owns why that is not the
    /// leniency the base64 decoders refuse: neither variance can change which
    /// octets come out. What is still refused is a symbol outside the
    /// alphabet — including a space, which is how a user-facing secret is
    /// grouped — a truncated final group, non-canonical trailing bits, and
    /// padding that is present but wrong.
    fn mwl_core_encoding_from_base32(_ctx, args: [1]) {
        let text = text_of(args, "fromBase32")?;
        // Only `a`-`z` move; every other byte, valid or not, reaches the
        // decoder exactly as written, so an offset in the error still indexes
        // the caller's own text.
        let folded = text.to_ascii_uppercase();
        let engine = if folded.ends_with('=') { &BASE32 } else { &BASE32_NOPAD };
        let raw = engine.decode(folded.as_bytes()).map_err(|error| {
            Fault::thrown(format!(
                "Core\\Encoding::fromBase32(): \"{}\" is not base32 — {}",
                shown(text),
                why_not_base32(&error)
            ))
        })?;
        Ok(Value::bytes(MwlStr::new(&raw)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::toHex(bytes $b): string` — replacing `bin2hex` and the
    /// `unpack("H*", …)` idiom written where `bin2hex` was forgotten.
    ///
    /// **Lowercase**, which is `bin2hex`'s own answer and the one every
    /// protocol that names a case names. [`mwl_core_encoding_from_hex`] reads
    /// either case back, so nothing round-trips differently for it.
    ///
    /// Total: every octet has a spelling, so there is nothing to reject.
    fn mwl_core_encoding_to_hex(_ctx, args: [1]) {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";

        let raw = bytes_of(args, "toHex")?;
        let mut out = Vec::with_capacity(raw.len() * 2);
        for octet in raw {
            out.push(DIGITS[usize::from(octet >> 4)]);
            out.push(DIGITS[usize::from(octet & 0x0f)]);
        }
        Ok(Value::str(MwlStr::new(&out)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Encoding::fromHex(string $s): bytes` — replacing `hex2bin`, and
    /// throwing where that function warned and answered `false`.
    ///
    /// **Either case, and no separators.** `"ff"`, `"FF"` and `"fF"` are one
    /// octet; a space, a `0x` prefix or a colon between pairs is a throw, not
    /// a value silently skipped. An odd number of digits is a throw for the
    /// same reason: `hex2bin("abc")` guessing which nibble the caller meant is
    /// exactly the substitution ADR 0009 § 3 removes from the language.
    fn mwl_core_encoding_from_hex(_ctx, args: [1]) {
        let text = text_of(args, "fromHex")?;
        let digits = text.as_bytes();
        let refused = |why: &str| {
            Fault::thrown(format!(
                "Core\\Encoding::fromHex(): \"{}\" is not hexadecimal — {why}",
                shown(text)
            ))
        };
        if digits.len() % 2 != 0 {
            return Err(refused("it has an odd number of digits, and an octet takes two"));
        }
        let mut out = Vec::with_capacity(digits.len() / 2);
        for pair in digits.chunks_exact(2) {
            let (Some(high), Some(low)) = (nibble(pair[0]), nibble(pair[1])) else {
                return Err(refused(
                    "only the digits `0`-`9`, `a`-`f` and `A`-`F` spell one, with nothing between \
                     the pairs",
                ));
            };
            out.push((high << 4) | low);
        }
        Ok(Value::bytes(MwlStr::new(&out)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nibble_reads_in_either_case_and_nothing_else() {
        assert_eq!(nibble(b'0'), Some(0));
        assert_eq!(nibble(b'9'), Some(9));
        assert_eq!(nibble(b'a'), Some(10));
        assert_eq!(nibble(b'F'), Some(15));
        assert_eq!(nibble(b'g'), None);
        assert_eq!(nibble(b' '), None);
        assert_eq!(nibble(b'-'), None);
    }

    /// The two engines are constants imported from `base64`, so what this
    /// module promises about padding and canonicality is a property of *which*
    /// constant rather than of any code here. Pin it, so a version bump that
    /// changed a default would fail here rather than in a caller's round trip.
    #[test]
    fn the_two_engines_differ_in_padding_and_refuse_a_non_canonical_tail() {
        assert_eq!(STANDARD.encode([0xff]), "/w==");
        assert_eq!(URL_SAFE_NO_PAD.encode([0xff]), "_w");

        // Each refuses the other's padding rule.
        assert!(STANDARD.decode("/w").is_err());
        assert!(URL_SAFE_NO_PAD.decode("_w==").is_err());

        // `/x` and `/w` name the same octet; only the one with zero unread
        // bits decodes (RFC 4648 § 3.5).
        assert_eq!(STANDARD.decode("/w==").unwrap(), [0xff]);
        assert!(matches!(
            STANDARD.decode("/x=="),
            Err(base64::DecodeError::InvalidLastSymbol(..))
        ));
    }

    /// The same pin as the base64 engines, for the three rules the module doc
    /// says `fromBase32` keeps while it folds case and padding.
    #[test]
    fn base32_is_unpadded_on_the_way_out_and_canonical_on_the_way_in() {
        assert_eq!(BASE32_NOPAD.encode(b"mwl"), "NV3WY");
        assert_eq!(BASE32.encode(b"mwl"), "NV3WY===");

        // A trailing bit no octet reads, and a symbol outside the alphabet.
        assert!(BASE32_NOPAD.decode(b"NV3WZ").is_err());
        assert!(BASE32_NOPAD.decode(b"NV3W1").is_err());

        // A group cut short.
        assert!(BASE32_NOPAD.decode(b"NV3WYA").is_err());
    }

    #[test]
    fn a_quoted_operand_is_bounded() {
        let long = "a".repeat(SHOWN_CHARS * 2);
        let quoted = shown(&long);
        assert_eq!(quoted.chars().count(), SHOWN_CHARS + 1);
        assert!(quoted.ends_with('…'));
        assert_eq!(shown("ff"), "ff");
    }
}
