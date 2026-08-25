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
//! `toHex`/`fromHex`. § 7's table also writes `toBase64`/`fromBase64`,
//! `toBase64Url`/`fromBase64Url`, `toBase32`/`fromBase32` and the
//! `encodeText`/`decodeText`/`isValidText` trio over a `Charset`; each is
//! unwritten, and `docs/agent/handoff.md` names which lands next.
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
//! well-used crate has already gotten wrong once and fixed — so those rows
//! will name a dependency when they land, and this comment is here so nobody
//! reads hex's answer as a rule for the section.
//!
//! # Encoding is total, decoding throws
//!
//! `toHex` cannot fail: every octet has a spelling. `fromHex` is the checked
//! direction ([ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)
//! R4) — it throws on an odd length or a non-hexadecimal character rather than
//! substituting, dropping or truncating, which is the same reason ADR 0009 § 3
//! refuses `iconv`'s `//IGNORE`. A caller who wants the question without the
//! throw asks it of the text before converting.

use mwl_runtime::{Fault, MwlStr, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Encoding`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Encoding";

/// `Core\Encoding`'s registry rows — § 7's hex pair, and nothing else yet.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
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

// ============================================================================
// The members
// ============================================================================

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

    #[test]
    fn a_quoted_operand_is_bounded() {
        let long = "a".repeat(SHOWN_CHARS * 2);
        let quoted = shown(&long);
        assert_eq!(quoted.chars().count(), SHOWN_CHARS + 1);
        assert!(quoted.ends_with('…'));
        assert_eq!(shown("ff"), "ff");
    }
}
