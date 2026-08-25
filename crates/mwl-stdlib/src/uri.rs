//! `Core\Uri` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 12's first table. This module is that table's **percent-encoding half**:
//! `encodeComponent`/`decodeComponent` and `encodeFormValue`/`decodeFormValue`,
//! which replace PHP's `rawurlencode`/`rawurldecode` and
//! `urlencode`/`urldecode`.
//!
//! # Two encodings, because PHP has two and the wire has two
//!
//! The pair split is not an accident of PHP's history that MWL is copying. A
//! percent-encoded *URI component* (RFC 3986 § 2.1) and an
//! `application/x-www-form-urlencoded` *form value* (WHATWG URL § 5) are
//! genuinely different encodings, and they disagree on one byte that matters:
//! a space is `%20` in a path segment and `+` in a form body. A program that
//! uses one where the other belongs produces a literal `+` inside a filename,
//! or a space where a `+` was typed — silently, and only for inputs nobody
//! tested with.
//!
//! So the members are named for **where the answer goes**, not for which C
//! function they came from: `encodeComponent` for anything that is part of a
//! URI, `encodeFormValue` for a value in a query string or a form body. That
//! is the whole of R10's argument-order-and-naming rule applied to the pair —
//! `rawurlencode` and `urlencode` differ by a prefix that says nothing about
//! which is which.
//!
//! # The two byte sets are PHP's, exactly
//!
//! | | left as itself | space | everything else |
//! |---|---|---|---|
//! | `encodeComponent` | `A-Z a-z 0-9 - _ . ~` | `%20` | `%XX`, upper-case hex |
//! | `encodeFormValue` | `A-Z a-z 0-9 - _ .` | `+` | `%XX`, upper-case hex |
//!
//! The one surprise in that table is that `~` is percent-encoded by
//! `encodeFormValue` and not by `encodeComponent`. It is not a typo and it is
//! not simplification worth taking: RFC 3986 § 2.3 added `~` to the unreserved
//! set, `application/x-www-form-urlencoded` is still defined against RFC 1738's
//! older set, and PHP's two functions each follow their own specification.
//! AGENTS.md ranks PHP-compatible observable behaviour (priority 2) above
//! simplicity of the implementation (priority 4), and the cost of collapsing
//! the two sets is paid by whoever compares a signature MWL computed against
//! one PHP computed — an HMAC over a form body differs by one byte and nothing
//! says why. Both spellings decode identically, so nothing is lost by matching.
//!
//! Hex digits are emitted **upper case**, which is PHP's choice and RFC 3986
//! § 2.1's recommendation. Both cases are read on the way back in.
//!
//! # Decoding is total, and a malformed escape is text
//!
//! `%` followed by anything that is not two hex digits — including a `%` at
//! the very end of the string — is left exactly as it stands rather than
//! throwing or dropping. That is PHP's behaviour for both functions, and it is
//! the right one for the position: a decoder sits at the edge of a request,
//! where a byte a client mistyped should not be able to abort a handler that
//! was going to reject the value anyway. The program still sees the `%`, so
//! nothing is silently swallowed.
//!
//! Neither decoder is a *validator*. `decodeComponent` will happily decode text
//! that could never have appeared in a URI; asking whether something is a URI
//! is `Uri::isValid`, which is not here yet — see gap 1.
//!
//! # What is not here: no dependency
//!
//! This half binds no outside crate, which is a deliberate exception to
//! [ground-rules.md](../../../../docs/adr/ground-rules.md)'s "an external
//! specification is a dependency rather than a hand-written parser". The rule
//! is about *grammars* — RFC 8259's, RFC 3986's — where a hand-written reader
//! accumulates divergences no test finds. There is no grammar here: the whole
//! specification is the two rows of the table above, and any crate that could
//! be bound would still need both byte sets written out beside it, because
//! neither is that crate's default. Binding one would add a dependency to
//! remove nothing.
//!
//! The **grammar** half of § 12's table — `parse`, `isValid`, `$uri->with`,
//! `$uri->resolve` — is RFC 3986, is therefore a dependency, and lands with
//! the pick that decides it. See gap 1.
//!
//! # What it spends
//!
//! One `string` per call, at most three bytes of output per input byte, freed
//! with the request that produced it. Nothing is retained between calls and no
//! table grows with traffic. Both encoders and both decoders are a single pass
//! with no backtracking, so a hostile input costs O(n) and cannot be made to
//! cost more.
//!
//! # Known gaps
//!
//! 1. **`Uri::parse`, `Uri::isValid`, `$uri->with` and `$uri->resolve` are not
//!    built**, so the spec table's grammar half is missing and this class has
//!    no instance shape yet. `isValid` is deliberately *not* hand-written
//!    ahead of `parse`: the two answer one question — "is this text a URI" —
//!    and a validator written against one reading of RFC 3986 beside a parser
//!    written against a crate's is the drift [`crate::uuid`] avoids by giving
//!    both members one `read`. Whichever crate `parse` binds decides `isValid`
//!    with it.
//! 2. **A decoder answers `string`, so it throws on bytes that are not valid
//!    UTF-8** — `decodeComponent("%FF")` throws rather than answering. The
//!    honest signature is `: bytes`, since percent-decoding is defined over
//!    octets and a client can send any of them; the throw is exactly what
//!    [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) § 3's checked
//!    `bytes as string` row would do one line later, so no program is denied
//!    an answer it could have used. It becomes `: bytes` when
//!    `mwl_runtime::Tag` gains a `Bytes` variant, which is
//!    [`crate::random`]'s gap 1. This is the one place this module diverges
//!    from PHP, whose strings are byte strings.
//! 3. **`parseQuery` and `buildQuery` are not built**, and they are where the
//!    spec's bracket convention lives. They are the other consumer of the two
//!    decoders here, not a second implementation of them.

use mwl_runtime::{Fault, HelperResult, MwlStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Uri`'s fully-qualified name, written once so the registry row and
/// every diagnostic naming the class cannot drift apart.
pub const NAME: &str = r"Core\Uri";

/// `Core\Uri`'s registry rows — spec § 12's first table, percent-encoding
/// half. The grammar half is gap 1.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "encodeComponent",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_encode_component",
        },
        CoreMethod {
            name: "decodeComponent",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_decode_component",
        },
        CoreMethod {
            name: "encodeFormValue",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_encode_form_value",
        },
        CoreMethod {
            name: "decodeFormValue",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_decode_form_value",
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
        "mwl_core_uri_encode_component" => (mwl_core_uri_encode_component as *const ()).cast(),
        "mwl_core_uri_decode_component" => (mwl_core_uri_decode_component as *const ()).cast(),
        "mwl_core_uri_encode_form_value" => (mwl_core_uri_encode_form_value as *const ()).cast(),
        "mwl_core_uri_decode_form_value" => (mwl_core_uri_decode_form_value as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The two encodings — one implementation each, selected by a flag
// ============================================================================

/// Which of the module docs' two rows a call is running.
///
/// A parameter rather than four separate loops, because the two rows differ in
/// exactly two decisions and duplicating the pass would make it possible for
/// them to drift in the twenty they share.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// RFC 3986 § 2.1: `~` is unreserved and a space is `%20`.
    Component,
    /// `application/x-www-form-urlencoded`: `~` is encoded and a space is `+`.
    FormValue,
}

impl Form {
    /// Whether `byte` is written as itself rather than percent-encoded.
    ///
    /// The module docs' table, in code: both sets are the ASCII alphanumerics
    /// plus `-_.`, and `~` joins them only for a URI component.
    const fn unreserved(self, byte: u8) -> bool {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.')
            || (matches!(self, Self::Component) && byte == b'~')
    }
}

/// The upper-case hex digits a percent-escape is written with — PHP's case and
/// RFC 3986 § 2.1's recommendation.
const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// `text` percent-encoded under `form`.
///
/// Capacity is the input's length rather than three times it: the common
/// subject is mostly unreserved, so reserving for the worst case would triple
/// the allocation of every call to pay for the rare one that needs it.
fn encode(text: &str, form: Form) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        match byte {
            b' ' if form == Form::FormValue => out.push('+'),
            _ if form.unreserved(byte) => out.push(char::from(byte)),
            _ => {
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    out
}

/// The two hex digits at `at`, as the byte they spell, or `None` where either
/// is missing or is not a hex digit.
///
/// Either case reads, which is RFC 3986 § 6.2.2.1's normalization rule seen
/// from the reading side: `%2f` and `%2F` are one octet, so refusing the
/// lower-case spelling would reject text every other decoder accepts.
fn escaped(bytes: &[u8], at: usize) -> Option<u8> {
    let digit = |offset: usize| -> Option<u8> {
        let value = char::from(*bytes.get(at + offset)?).to_digit(16)?;
        u8::try_from(value).ok()
    };
    Some((digit(0)? << 4) | digit(1)?)
}

/// `text` percent-decoded under `form`, as the octets it spells.
///
/// Answers bytes rather than a `String` because that is what percent-decoding
/// produces — the UTF-8 question is the caller's, and gap 2 owns why it is
/// asked at all.
fn decode(text: &str, form: Form) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' if form == Form::FormValue => {
                out.push(b' ');
                at += 1;
            }
            // A malformed escape is text, not an error — see the module docs.
            b'%' => match escaped(bytes, at + 1) {
                Some(byte) => {
                    out.push(byte);
                    at += 3;
                }
                None => {
                    out.push(b'%');
                    at += 1;
                }
            },
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    out
}

// ============================================================================
// The boundary — arguments in, one `string` out
// ============================================================================

/// The `string` in argument slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member. Compiled code wrote the tag and
/// `mwl_types` already checked the declared type, so a slot holding anything
/// else is a runtime-contract violation rather than anything a program can
/// cause — the same treatment [`crate::path`] gives its own arguments.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    let bytes = args[0].as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uri::{member} expected {:?}, got tag {}",
            Tag::Str,
            args[0].tag_byte()
        ))
    })?;
    std::str::from_utf8(bytes).map_err(|_| {
        Fault::fatal(format!(
            "Core\\Uri::{member} received a `string` that is not valid UTF-8, which ADR 0009 \
             guarantees it cannot be"
        ))
    })
}

/// `text` as an owned `string` value.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(MwlStr::new(text.as_bytes())))
}

/// `octets` as a `string`, or a throw where they are not UTF-8.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the member and the offset of the first bad byte.
/// This is gap 2: percent-decoding answers octets, `string` is UTF-8, and the
/// throw is ADR 0009 § 3's checked `bytes as string` row reached one member
/// early. The offset is a position in text the caller supplied, so it is safe
/// to name and it is the one fact that makes the throw actionable — the octets
/// themselves are not quoted, since they are by definition not text.
fn decoded(octets: Vec<u8>, member: &str) -> HelperResult {
    let text = String::from_utf8(octets).map_err(|error| {
        Fault::thrown(format!(
            "Core\\Uri::{member}(): the decoded octets are not valid UTF-8 — byte {} begins a \
             sequence a `string` cannot hold. Percent-decoding answers octets, so text carrying \
             an escape for a non-UTF-8 byte has no `string` to decode to",
            error.utf8_error().valid_up_to()
        ))
    })?;
    produced(&text)
}

// ============================================================================
// The members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Uri::encodeComponent(string $s): string` — replacing PHP's
    /// `rawurlencode`.
    ///
    /// **For a piece of a URI**: a path segment, a fragment, or one side of a
    /// query pair being assembled by hand. A space becomes `%20`, because that
    /// is what a space is inside a URI — a `+` there is a literal `+`.
    ///
    /// Every byte outside RFC 3986 § 2.3's unreserved set is escaped,
    /// including the reserved delimiters `/ ? # & =`. That is what makes the
    /// answer safe to *interpolate*: a segment holding a `/` cannot climb out
    /// of its position in the path, and a value holding an `&` cannot open a
    /// second query pair. Escaping only the unsafe-looking bytes is how the
    /// injection this member exists to prevent gets back in.
    fn mwl_core_uri_encode_component(_ctx, args: [1]) {
        let text = text_of(args, "encodeComponent")?;

        produced(&encode(text, Form::Component))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::decodeComponent(string $s): string` — replacing PHP's
    /// `rawurldecode`.
    ///
    /// The exact inverse of [`mwl_core_uri_encode_component`] for text that
    /// member produced. A `+` is a literal `+`, which is the whole reason this
    /// is a different member from `decodeFormValue` rather than an option on
    /// one: reading a form value with this decoder turns every space the user
    /// typed into a `+`.
    ///
    /// A malformed escape decodes to itself and non-UTF-8 octets throw — the
    /// module docs and gap 2 own both.
    fn mwl_core_uri_decode_component(_ctx, args: [1]) {
        let text = text_of(args, "decodeComponent")?;

        decoded(decode(text, Form::Component), "decodeComponent")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::encodeFormValue(string $s): string` — replacing PHP's
    /// `urlencode`.
    ///
    /// **For a value in an `application/x-www-form-urlencoded` payload**: a
    /// query-string pair or a POST body. A space becomes `+` and `~` becomes
    /// `%7E`, which are the two bytes this member's set differs from
    /// [`mwl_core_uri_encode_component`]'s on; the module docs own why the
    /// difference is kept rather than collapsed.
    ///
    /// A program building a whole query string reaches for `Uri::buildQuery`
    /// instead (gap 3), which writes the `=` and the `&` as well. This member
    /// is one side of one pair.
    fn mwl_core_uri_encode_form_value(_ctx, args: [1]) {
        let text = text_of(args, "encodeFormValue")?;

        produced(&encode(text, Form::FormValue))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::decodeFormValue(string $s): string` — replacing PHP's
    /// `urldecode`.
    ///
    /// The inverse of [`mwl_core_uri_encode_form_value`]: `+` is a space, and
    /// `%2B` is the `+` the user actually typed. Both spellings of a space
    /// therefore read, since `%20` is still an escape — which is what makes
    /// this the right decoder for a query string written by something that
    /// followed RFC 3986 rather than the form encoding.
    ///
    /// A malformed escape decodes to itself and non-UTF-8 octets throw — the
    /// module docs and gap 2 own both.
    fn mwl_core_uri_decode_form_value(_ctx, args: [1]) {
        let text = text_of(args, "decodeFormValue")?;

        decoded(decode(text, Form::FormValue), "decodeFormValue")
    }
}

#[cfg(test)]
mod tests {
    use mwl_runtime::{Ctx, OutputSink, Value, call};

    use super::{Form, encode};

    /// Runs one member through the ADR 0002 boundary compiled code reaches it
    /// at — [`crate::random`]'s own test helper, for its reasons.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        subject: &str,
    ) -> Result<String, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(mwl_runtime::MwlStr::new(subject.as_bytes()));
        let answer = call(member, &mut ctx, &[argument]);
        let out = answer.map(|value| {
            let text = String::from_utf8(
                value
                    .as_str_bytes()
                    .expect("every member here answers with a `string`")
                    .to_vec(),
            )
            .expect("ADR 0009 guarantees a `string` is UTF-8");
            #[expect(unsafe_code, reason = "this frame owns the reference the helper built")]
            unsafe {
                value.release();
            }
            text
        });
        #[expect(
            unsafe_code,
            reason = "this frame owns the argument it built, and every member \
                      here borrows rather than consumes"
        )]
        unsafe {
            argument.release();
        }
        out
    }

    /// The whole of ASCII plus one multi-byte character, round-tripped both
    /// ways: the assertion that catches a byte one encoder escapes and its own
    /// decoder does not restore.
    #[test]
    fn every_byte_round_trips_through_both_encodings() {
        let subject: String = (0..=127_u8).map(char::from).chain(['é', '→']).collect();
        for (encoder, decoder) in [
            (
                super::mwl_core_uri_encode_component
                    as unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
                super::mwl_core_uri_decode_component
                    as unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
            ),
            (
                super::mwl_core_uri_encode_form_value,
                super::mwl_core_uri_decode_form_value,
            ),
        ] {
            let encoded = run(encoder, &subject).expect("an encoder never fails");
            assert!(
                encoded.is_ascii(),
                "an encoded answer is ASCII by construction"
            );
            assert_eq!(
                run(decoder, &encoded).expect("its own output decodes"),
                subject
            );
        }
    }

    /// The two sets differ on exactly two bytes — the module docs' table,
    /// asserted rather than described, so widening either set fails here.
    #[test]
    fn the_two_encodings_differ_on_exactly_space_and_tilde() {
        let differing: Vec<u8> = (0..=127_u8)
            .filter(|&byte| {
                encode(&char::from(byte).to_string(), Form::Component)
                    != encode(&char::from(byte).to_string(), Form::FormValue)
            })
            .collect();
        assert_eq!(differing, [b' ', b'~']);
    }

    /// A decoded octet outside UTF-8 has no `string` to land in, so the member
    /// throws rather than substituting — gap 2, and ADR 0009 § 3's rule.
    #[test]
    fn a_non_utf8_octet_throws_rather_than_being_replaced() {
        assert!(run(super::mwl_core_uri_decode_component, "a%FFb").is_err());
        assert!(run(super::mwl_core_uri_decode_form_value, "%C3%28").is_err());
    }

    /// PHP leaves a `%` that does not begin two hex digits exactly as it
    /// stands, and so does this — the module docs own why a decoder at the
    /// edge of a request does not throw over one.
    #[test]
    fn a_malformed_escape_decodes_to_itself() {
        for (subject, expected) in [
            ("%zz", "%zz"),
            ("%4", "%4"),
            ("100%", "100%"),
            ("%%41", "%A"),
            ("%2f", "/"),
        ] {
            assert_eq!(
                run(super::mwl_core_uri_decode_component, subject).expect("no throw"),
                expected
            );
        }
    }
}
