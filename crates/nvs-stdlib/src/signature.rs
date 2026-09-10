//! The canonical form every signature in the language is taken over —
//! `rule:core-api/signing-is-over-a-payload`'s "never over assembled text",
//! written once so that the three doors onto it (`Core\Signature`, `Core\Uri`
//! and `Core\Router`) cannot each canonicalize a payload slightly differently.
//!
//! `rule:core-classes/signature` places the class; what belongs here is the
//! wire format itself, why it is a document rather than a string, and what it
//! refuses.
//!
//! # A signature is taken over a document, and the document is built here
//!
//! Every hand-rolled signing helper in every language gets the same thing
//! wrong: it assembles text — `"$id|$until|$path"` — and then two sides of the
//! same protocol disagree about the assembly. A separator that appears in a
//! value, a key written in a different order, an integer rendered `1` on one
//! side and `1.0` on the other, and a signature that verifies over bytes the
//! program never meant. So nothing here is assembled. A payload is walked and
//! written into a **type-tagged, length-prefixed** document: every value
//! carries the tag of its own type, every variable-length body carries its
//! length before it, and no two distinct values encode to the same bytes.
//! `1` and `"1"` differ in their first byte; `["ab", "c"]` and `["a", "bc"]`
//! differ in their lengths; there is no separator to smuggle anything past.
//!
//! **A key is text**, and written as the text the language itself renders it
//! as. An array key is a `string` whatever shape the array is in — a list
//! holds `0` as a position and a hashed array holds it as `"0"`, and
//! `nvs_runtime::NvsArray::degrade`'s own docs are where "without changing a
//! single answer" is written down — so encoding a position as an integer would
//! give one array two signatures, depending only on whether it had ever had a
//! gap in it.
//!
//! **Keys are sorted**, by those octets, at every level. Two
//! programs that build the same map in a different order therefore sign
//! identically, which is the property a signed URL needs — the query string it
//! travels in has no ordering either ([`crate::uri`]'s RFC 3986 § 6.2.2
//! normalization is the one that says so). The consequence is the one thing to
//! know about a round trip: **insertion order is not signed and does not come
//! back.** A verified payload is in canonical order, not in the order the
//! signer wrote it.
//!
//! **The lifetime rides inside the document**, not beside it: [`Until`] is a
//! field of the signed region, so a holder cannot edit an expiry that is
//! covered by the tag, and there is no second parameter for the two sides to
//! keep in step (`rule:core-api/a-lifetime-is-written`).
//!
//! # The domain byte, so one ring cannot be replayed across three doors
//!
//! A program is expected to hand the same key ring to `Core\Signature` and to
//! `$uri->sign` — one rotation vocabulary, one ring. That makes cross-door
//! replay a real question rather than a theoretical one, so every document
//! names the [`Domain`] it was made for, inside the signed region and before
//! anything else. A token minted for a payload does not verify as a URL
//! signature whatever else matches, and the check costs one byte and one
//! comparison.
//!
//! # What it refuses, and why refusing is the safe answer
//!
//! An object, a closure and a resource have no canonical form at all — two
//! instances that a program calls equal are two different heaps — so a payload
//! carrying one is a `LogicError` naming the path to it rather than a
//! signature over something arbitrary. A `decimal` **is** signable, because
//! `nvs_runtime::Decimal::reduced` is a canonical form the language already
//! defines: `1.0` and `1.00` sign identically, exactly as they compare equal.
//!
//! Nesting is bounded at [`MAX_DEPTH`]. Both walks here recurse, so the bound
//! is a bound on stack, and a signed payload is a claim set rather than a
//! document — a program with a deep tree to sign signs its serialization as a
//! `string`, which is one value at depth one.
//!
//! # What it spends
//!
//! One `Vec` the size of the document, per `sign` and per `verify`, inside the
//! call and held nowhere between calls (`rule:programs/memory-priority`). The
//! sort is per array level over that level's keys. A document is the payload's
//! own octets plus one tag byte and one length varint per value, plus two
//! bytes of version and domain and one to thirteen of lifetime — so a token,
//! after its 32-octet tag and base64's four thirds, is about
//! `4/3 × (payload + 40)` characters.

#![allow(
    dead_code,
    reason = "`Core\\Signature`'s two rows are the first caller of all of this, and they land in \
              the slice after the one that wrote it — until then the tests below are what reaches \
              it. The attribute goes with those rows."
)]

use nvs_runtime::{Decimal, Fault, NvsArray, NvsStr, SlotKey, Tag, ThrownClass, Value};

/// The document format's own version, first octet of every document and inside
/// the signed region.
///
/// It is *not* a negotiation: a reader accepts this version and no other. What
/// it buys is that a future format change cannot be made to look like the
/// current one by an old token, and that the failure is a refusal rather than
/// a misreading.
const VERSION: u8 = 1;

/// The deepest nesting a payload may carry, counted the way
/// [`crate::json::DEFAULT_MAX_DEPTH`] counts: a scalar payload is depth 1, and
/// `[[1]]` is depth 3.
///
/// Far below JSON's, and deliberately: a document is what
/// [`crate::json`] is for, and a signed payload is a claim set. The number
/// bounds the native stack both walks below recurse on, which is the reason it
/// exists at all.
pub(crate) const MAX_DEPTH: u32 = 32;

/// The tag of each type, on the wire.
///
/// Its own numbering rather than [`Tag`]'s, because these octets are a **wire
/// format** — a token minted today is read back after the runtime's internal
/// discriminants have been renumbered, and nothing stops that renumbering
/// except this module not depending on it.
mod tags {
    pub(super) const NULL: u8 = 0x00;
    pub(super) const FALSE: u8 = 0x01;
    pub(super) const TRUE: u8 = 0x02;
    /// An `int`, as eight big-endian octets of two's complement.
    pub(super) const INT: u8 = 0x03;
    /// A `uint`, as eight big-endian octets.
    pub(super) const UINT: u8 = 0x04;
    /// A `float`, as the eight big-endian octets of its IEEE-754 bits — the
    /// bits and not a rendering, so the round trip is exact and `-0.0` stays
    /// itself.
    pub(super) const FLOAT: u8 = 0x05;
    /// A `decimal`, as the sixteen big-endian octets of
    /// `Decimal::reduced().to_bits()`.
    pub(super) const DECIMAL: u8 = 0x06;
    /// A `string`: a length varint, then that many octets of UTF-8.
    pub(super) const STR: u8 = 0x07;
    /// A `bytes`: a length varint, then that many octets.
    pub(super) const BYTES: u8 = 0x08;
    /// An array: a count varint, then that many key-then-value pairs, the keys
    /// in ascending order of their own encoding.
    pub(super) const ARRAY: u8 = 0x09;
}

/// The lifetime field's two spellings: absent, and present.
mod lifetime {
    pub(super) const FOREVER: u8 = 0x00;
    pub(super) const UNTIL: u8 = 0x01;
}

/// Which door a document was made for — the domain separation the module doc's
/// own section explains.
///
/// The octet is part of the signed region, so a value here is a wire constant:
/// a variant's number is never reused for another door.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Domain {
    /// `Core\Signature::sign` — a payload map the caller wrote.
    Payload = 1,
    /// `$uri->sign` — one URL in the normalized form `Core\Uri::compareTo`
    /// defines.
    Uri = 2,
    /// `Core\Router::urlSigned` — a route's name and its typed parameters,
    /// which is what survives a remount when a path does not.
    Route = 3,
}

impl Domain {
    /// The octet this domain is written as.
    const fn byte(self) -> u8 {
        self as u8
    }

    /// The domain `byte` names, or `None` for an octet no door writes.
    const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Payload),
            2 => Some(Self::Uri),
            3 => Some(Self::Route),
            _ => None,
        }
    }
}

/// The instant a signature stops being valid, in `Core\Time\Instant`'s own two
/// parts.
///
/// Two parts rather than one count because that is what an `Instant` holds
/// (`crate::time`'s `INSTANT` has an epoch-second slot and a subsecond one),
/// so nothing converts on the way in or out and a lifetime that travels in a
/// token is the lifetime that was written. `rule:core-api/a-lifetime-is-written`
/// makes the field required and `null` — [`Option::None`] here — the forever
/// spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Until {
    /// Whole seconds since the Unix epoch, negative before it.
    pub(crate) second: i64,
    /// The subsecond part, `0..1_000_000_000`. A reader refuses anything
    /// larger, so a decoded `Until` is always one an `Instant` can hold.
    pub(crate) nano: u32,
}

/// One nanosecond past the largest [`Until::nano`] a reader accepts.
const NANOS_PER_SECOND: u32 = 1_000_000_000;

/// The signed region for `payload`, under `domain` and expiring at `until`.
///
/// This is the whole of what a signature is taken over, and the whole of what
/// travels in a token: `verify` authenticates these octets and only then reads
/// them back with [`read_document`], so no route exists in which something is
/// parsed before its tag has been checked.
///
/// `who` names the member for a refusal, spelled `Core\Class::member`, and
/// `root` is what the payload is called at the call site — `$payload` — so a
/// message can name the path to the value it refused.
///
/// # Errors
///
/// A `LogicError` for a value with no canonical form — an object, a closure, a
/// resource — and for a payload nested past [`MAX_DEPTH`], both naming the
/// path.
pub(crate) fn document(
    domain: Domain,
    until: Option<Until>,
    payload: &Value,
    who: &str,
    root: &str,
) -> Result<Vec<u8>, Fault> {
    let mut out = vec![VERSION, domain.byte()];
    match until {
        None => out.push(lifetime::FOREVER),
        Some(until) => {
            out.push(lifetime::UNTIL);
            out.extend_from_slice(&until.second.to_be_bytes());
            out.extend_from_slice(&until.nano.to_be_bytes());
        }
    }
    let mut path = String::from(root);
    write_value(&mut out, payload, 1, &mut path, who)?;
    Ok(out)
}

/// The domain, the lifetime and the payload a document holds, or `None` for
/// octets that are not one.
///
/// `None` covers every malformed shape at once — a version this runtime does
/// not write, a truncated body, a length that runs past the end, a key order
/// that is not the canonical one, an octet left over at the end. The caller
/// folds all of it into its own one refusal
/// (`rule:core-api/one-refusal-except-expiry`), because a reader that says
/// *which* part failed is a reader telling a forger which half landed.
///
/// **Strict on purpose.** A document that authenticates but does not re-encode
/// to itself would mean two spellings of one payload, which is the property
/// the whole module exists to deny, so a non-minimal length or an unsorted key
/// is refused here even though the tag has already been checked.
pub(crate) fn read_document(bytes: &[u8]) -> Option<(Domain, Option<Until>, Value)> {
    let mut reader = Reader { bytes, at: 0 };
    if reader.octet()? != VERSION {
        return None;
    }
    let domain = Domain::from_byte(reader.octet()?)?;
    let until = match reader.octet()? {
        lifetime::FOREVER => None,
        lifetime::UNTIL => {
            let second = i64::from_be_bytes(reader.take(8)?.try_into().ok()?);
            let nano = u32::from_be_bytes(reader.take(4)?.try_into().ok()?);
            if nano >= NANOS_PER_SECOND {
                return None;
            }
            Some(Until { second, nano })
        }
        _ => return None,
    };
    let payload = reader.value(1)?;
    if reader.at == reader.bytes.len() {
        return Some((domain, until, payload));
    }
    // An octet past the end is a document that would re-encode to something
    // shorter, so it is not this one. The payload built above owns references
    // that no caller is going to take.
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `Reader::value` just produced"
    )]
    unsafe {
        payload.release();
    }
    None
}

// ============================================================================
// Writing
// ============================================================================

/// Writes `value` at `depth`, with `path` naming where it sits in the payload.
///
/// # Errors
///
/// The `LogicError`s [`document`] documents.
fn write_value(
    out: &mut Vec<u8>,
    value: &Value,
    depth: u32,
    path: &mut String,
    who: &str,
) -> Result<(), Fault> {
    if depth > MAX_DEPTH {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{who}(): {path} nests deeper than {MAX_DEPTH}, which is as deep as a signed \
                 payload goes — a document belongs in one value, as text or as JSON, rather \
                 than as a tree of its own"
            ),
        ));
    }
    match value.tag() {
        Some(Tag::Null) => out.push(tags::NULL),
        Some(Tag::Bool) => out.push(if value.as_bool().unwrap_or(false) {
            tags::TRUE
        } else {
            tags::FALSE
        }),
        Some(Tag::Int) => {
            out.push(tags::INT);
            out.extend_from_slice(&value.as_int().unwrap_or(0).to_be_bytes());
        }
        Some(Tag::Uint) => {
            out.push(tags::UINT);
            out.extend_from_slice(&value.as_uint().unwrap_or(0).to_be_bytes());
        }
        Some(Tag::Float) => {
            out.push(tags::FLOAT);
            out.extend_from_slice(&value.as_float().unwrap_or(0.0).to_bits().to_be_bytes());
        }
        Some(Tag::Decimal) => {
            let held = value
                .as_decimal()
                .unwrap_or_else(Decimal::zero)
                .reduced()
                .to_bits();
            out.push(tags::DECIMAL);
            out.extend_from_slice(&held.to_be_bytes());
        }
        Some(Tag::Str) => {
            out.push(tags::STR);
            write_run(out, value.as_str_bytes().unwrap_or_default());
        }
        Some(Tag::Bytes) => {
            out.push(tags::BYTES);
            write_run(out, value.as_bytes().unwrap_or_default());
        }
        Some(Tag::Array) => {
            let raw = value.array_ptr().expect("a `Tag::Array` value is an array");
            write_array(out, &crate::arr::borrowed(raw), depth, path, who)?;
        }
        _ => {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{who}(): {path} is {}, which has no canonical form to sign — a signed \
                     payload holds null, bool, int, uint, float, decimal, string, bytes and \
                     arrays of those, and anything else is signed by writing down what about \
                     it matters",
                    value.tag().map_or("a value of no type", Tag::describe)
                ),
            ));
        }
    }
    Ok(())
}

/// Writes `array`'s entries, keys first and in ascending order of their own
/// encoding.
///
/// The order is the array's *canonical* one and never its insertion order: the
/// module doc's own section is the home of why, and of what a caller gives up.
///
/// # Errors
///
/// The `LogicError`s [`document`] documents, for any entry.
fn write_array(
    out: &mut Vec<u8>,
    array: &NvsArray,
    depth: u32,
    path: &mut String,
    who: &str,
) -> Result<(), Fault> {
    let mut entries: Vec<(Vec<u8>, usize)> = Vec::with_capacity(array.count());
    let mut slot = 0;
    while let Some(live) = array.next_slot(slot) {
        slot = live + 1;
        let key = array.slot_key(live).expect("a live slot has a key");
        entries.push((key.to_str().as_bytes().to_vec(), live));
    }
    // An array holds each key once, so ordering by the key alone is total and
    // the sort has nothing to break ties over.
    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    out.push(tags::ARRAY);
    write_varint(
        out,
        u64::try_from(entries.len()).expect("an array's length fits a `u64`"),
    );
    let root = path.len();
    for (key, live) in entries {
        out.push(tags::STR);
        write_run(out, &key);
        let held = array.value_at(live).expect("a live slot holds a value");
        path.push('[');
        path.push_str(&String::from_utf8_lossy(&key));
        path.push(']');
        write_value(out, &held, depth + 1, path, who)?;
        path.truncate(root);
    }
    Ok(())
}

/// Writes a length varint and then `run`.
fn write_run(out: &mut Vec<u8>, run: &[u8]) {
    write_varint(
        out,
        u64::try_from(run.len()).expect("a run's length fits a `u64`"),
    );
    out.extend_from_slice(run);
}

/// Writes `value` as an unsigned LEB128, which is minimal by construction —
/// [`Reader::varint`] accepts nothing else.
fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let octet = u8::try_from(value & 0x7f).expect("seven bits fit an octet");
        value >>= 7;
        if value == 0 {
            out.push(octet);
            return;
        }
        out.push(octet | 0x80);
    }
}

// ============================================================================
// Reading
// ============================================================================

/// A cursor over a document, answering `None` for every malformed shape.
struct Reader<'a> {
    /// The document.
    bytes: &'a [u8],
    /// How much of it has been read.
    at: usize,
}

impl Reader<'_> {
    /// The next octet.
    fn octet(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    /// The next `len` octets.
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(len)?;
        let run = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(run)
    }

    /// The next unsigned LEB128, refusing any spelling but the minimal one.
    ///
    /// A padded varint would be a second encoding of a document that already
    /// has one, which is the property this module denies; refusing it here is
    /// what makes a document that authenticates re-encode to itself.
    fn varint(&mut self) -> Option<u64> {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        loop {
            let octet = self.octet()?;
            if shift >= 64 || (shift == 63 && octet > 1) {
                return None;
            }
            value |= u64::from(octet & 0x7f) << shift;
            if octet & 0x80 == 0 {
                // A final octet of zero past the first is padding, and so is a
                // continuation that carried nothing.
                return (shift == 0 || octet != 0).then_some(value);
            }
            shift += 7;
        }
    }

    /// The next length varint's worth of octets.
    fn run(&mut self) -> Option<&[u8]> {
        let len = usize::try_from(self.varint()?).ok()?;
        self.take(len)
    }

    /// The next value, at `depth`.
    ///
    /// The caller owns whatever references the answer holds; every failure
    /// below releases what it had built before answering `None`.
    fn value(&mut self, depth: u32) -> Option<Value> {
        if depth > MAX_DEPTH {
            return None;
        }
        Some(match self.octet()? {
            tags::NULL => Value::null(),
            tags::FALSE => Value::bool(false),
            tags::TRUE => Value::bool(true),
            tags::INT => Value::int(i64::from_be_bytes(self.take(8)?.try_into().ok()?)),
            tags::UINT => Value::uint(u64::from_be_bytes(self.take(8)?.try_into().ok()?)),
            tags::FLOAT => Value::float(f64::from_bits(u64::from_be_bytes(
                self.take(8)?.try_into().ok()?,
            ))),
            tags::DECIMAL => {
                let bits = u128::from_be_bytes(self.take(16)?.try_into().ok()?);
                let held = Decimal::from_bits(bits)?;
                // A `decimal` is written reduced, so one that is not is a
                // second spelling of a value that already has one.
                if held.to_bits() != held.reduced().to_bits() {
                    return None;
                }
                Value::decimal(held)
            }
            tags::STR => Value::str(NvsStr::new(
                std::str::from_utf8(self.run()?).ok()?.as_bytes(),
            )),
            tags::BYTES => Value::bytes(NvsStr::new(self.run()?)),
            tags::ARRAY => Value::array(self.array(depth)?),
            _ => return None,
        })
    }

    /// The next array's entries, refusing a key order that is not the one
    /// [`write_array`] writes.
    fn array(&mut self, depth: u32) -> Option<NvsArray> {
        let count = usize::try_from(self.varint()?).ok()?;
        let mut array = NvsArray::new();
        let mut previous: Option<Vec<u8>> = None;
        for _ in 0..count {
            let key = self.key()?;
            if previous.is_some_and(|before| before >= key) {
                return None;
            }
            // A failure below drops `array`, whose own `Drop` releases every
            // value already stored in it.
            let held = self.value(depth + 1)?;
            crate::arr::store_at(&mut array, SlotKey::Str(NvsStr::new(&key)), held);
            previous = Some(key);
        }
        Some(array)
    }

    /// The next array key, which is text and whose octets are its order.
    fn key(&mut self) -> Option<Vec<u8>> {
        (self.octet()? == tags::STR)
            .then(|| self.run().map(<[u8]>::to_vec))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload map, built in the order given.
    fn payload(entries: &[(&str, Value)]) -> Value {
        let mut array = NvsArray::new();
        for (key, held) in entries {
            array.set(NvsStr::new(key.as_bytes()), *held);
        }
        Value::array(array)
    }

    /// Releases what a test built, so the crate's allocation gate sees a
    /// balanced run.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// The document for a payload, under the domain and lifetime a test names.
    fn signed(payload: &Value, until: Option<Until>) -> Vec<u8> {
        document(
            Domain::Payload,
            until,
            payload,
            "Core\\Signature::sign",
            "$payload",
        )
        .expect("a payload of scalars encodes")
    }

    /// The document for a map written inline, which the helper builds, signs
    /// and releases — every reference in `entries` belongs to the map.
    fn signed_map(entries: &[(&str, Value)], until: Option<Until>) -> Vec<u8> {
        let held = payload(entries);
        let wire = signed(&held, until);
        dropped(held);
        wire
    }

    /// Stage 2's first canonicalization check: the order a program writes a
    /// map in is not part of what it signs.
    ///
    /// Asserted over the tag as well as over the document, because the
    /// property that matters to a caller is that the two *sign* alike, and a
    /// document compared to itself would still hold if the tag were taken over
    /// something else.
    #[test]
    fn the_canonical_encoding_sorts_keys_so_two_orderings_sign_identically() {
        let one = payload(&[
            ("user", Value::int(7)),
            ("action", Value::int(2)),
            ("scope", Value::int(9)),
        ]);
        let other = payload(&[
            ("scope", Value::int(9)),
            ("user", Value::int(7)),
            ("action", Value::int(2)),
        ]);

        let left = signed(&one, None);
        let right = signed(&other, None);
        assert_eq!(left, right, "the same pairs in two orders are one document");
        assert_eq!(
            crate::hash::hmac_sha256(&[3_u8; 32], &left),
            crate::hash::hmac_sha256(&[3_u8; 32], &right),
            "so the two sign identically, which is what a query string needs"
        );

        // And the order they come back in is the canonical one, not either
        // one that went in.
        let (_, _, back) = read_document(&left).expect("its own document reads");
        let raw = back.array_ptr().expect("a payload map is an array");
        assert_eq!(
            crate::arr::borrowed(raw).keys(),
            vec![b"action".to_vec(), b"scope".to_vec(), b"user".to_vec()],
            "a verified payload is in canonical order and not in the signer's"
        );

        dropped(back);
        dropped(one);
        dropped(other);
    }

    /// The other half of a canonical form: a value carries its type, so no two
    /// payloads a program can tell apart sign alike.
    ///
    /// Written over the pair every ad-hoc `implode`-and-hash helper conflates
    /// — the number one and the text of it — and over the same pair in key
    /// position, which is the half a value-only encoding still gets wrong.
    #[test]
    fn an_int_one_and_a_string_one_encode_to_different_bytes() {
        let numeric = payload(&[("id", Value::int(1))]);
        let textual = payload(&[("id", Value::str(NvsStr::new(b"1")))]);
        assert_ne!(
            signed(&numeric, None),
            signed(&textual, None),
            "`1` and `\"1\"` are two payloads and sign as two"
        );

        // The same question of a *key* has the opposite answer, and it is the
        // language's rather than this module's: a key is a `string` whichever
        // shape the array is in, so a position and the text of it are one key
        // and sign as one. A list that has degraded to the hash form must not
        // stop verifying, which is what this pins.
        let mut indexed = NvsArray::new();
        indexed.set_index(1, Value::bool(true));
        let indexed = Value::array(indexed);
        let named = payload(&[("1", Value::bool(true))]);
        assert_eq!(
            signed(&indexed, None),
            signed(&named, None),
            "a positional key and the text of its own number are one key"
        );

        // Nor does any other pair of neighbouring types collide, including
        // the two that hold the same octets and the two that hold none.
        let builders: [fn() -> Value; 7] = [
            Value::null,
            || Value::bool(false),
            || Value::int(1),
            || Value::uint(1),
            || Value::float(1.0),
            || Value::str(NvsStr::new(b"")),
            || Value::bytes(NvsStr::new(b"")),
        ];
        let mut seen: Vec<Vec<u8>> = builders
            .iter()
            .map(|build| signed_map(&[("v", build())], None))
            .collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total, "each type encodes to bytes of its own");

        dropped(numeric);
        dropped(textual);
        dropped(indexed);
        dropped(named);
    }

    /// Every shape a document takes survives the round trip, including the
    /// two the wire format spends bytes to keep exact.
    #[test]
    fn a_document_reads_back_as_the_payload_that_was_written() {
        let mut nested = NvsArray::new();
        nested.append(Value::float(-0.0));
        nested.append(Value::bytes(NvsStr::new(&[0_u8, 0xff, 0x80])));
        let held = payload(&[
            ("map", Value::array(nested)),
            ("text", Value::str(NvsStr::new("héllo".as_bytes()))),
            ("big", Value::uint(u64::MAX)),
            ("low", Value::int(i64::MIN)),
            ("none", Value::null()),
        ]);
        let until = Until {
            second: 1_800_000_000,
            nano: 123_456_789,
        };

        let wire = signed(&held, Some(until));
        let (domain, read_until, back) = read_document(&wire).expect("its own document reads");
        assert_eq!(domain, Domain::Payload);
        assert_eq!(read_until, Some(until), "the lifetime rides inside");
        assert_eq!(
            signed(&back, Some(until)),
            wire,
            "and what comes back re-encodes to the document it came from"
        );

        dropped(back);
        dropped(held);
    }

    /// A document is refused unless it is *the* encoding of what it holds —
    /// the property that makes "signed" and "canonical" the same word.
    #[test]
    fn a_document_that_would_re_encode_to_something_else_is_refused() {
        let held = payload(&[("a", Value::int(1)), ("b", Value::int(2))]);
        let wire = signed(&held, None);
        assert!(read_document(&wire).is_some(), "the canonical one reads");

        let mut trailing = wire.clone();
        trailing.push(0);
        assert!(read_document(&trailing).is_none(), "an octet past the end");

        let mut short = wire.clone();
        short.pop();
        assert!(read_document(&short).is_none(), "a truncated one");

        let mut version = wire.clone();
        version[0] = VERSION + 1;
        assert!(
            read_document(&version).is_none(),
            "a version we never wrote"
        );

        let mut domain = wire.clone();
        domain[1] = Domain::Uri.byte();
        let (read, _, other) = read_document(&domain).expect("another door's is still a document");
        assert_eq!(read, Domain::Uri, "and the door it names is the one read");
        dropped(other);

        // The two spellings that hold the same payload and are not canonical:
        // the keys the other way round, and a padded length varint.
        assert_eq!(
            signed_map(&[("b", Value::int(2)), ("a", Value::int(1))], None),
            wire,
            "the writer sorts, so the order a caller wrote is already gone"
        );
        // Both entries are the same size — a one-octet key and an `int` — so
        // putting the second first is one rotation past the five octets of
        // version, domain, lifetime, array tag and count.
        const HEADER: usize = 5;
        let mut swapped = wire.clone();
        swapped[HEADER..].rotate_left((wire.len() - HEADER) / 2);
        assert_ne!(swapped, wire, "the rotation moved something");
        assert!(
            read_document(&swapped).is_none(),
            "and a hand-swapped pair is not a document this writer could have written"
        );

        let mut padded = vec![
            VERSION,
            Domain::Payload.byte(),
            lifetime::FOREVER,
            tags::ARRAY,
        ];
        write_varint(&mut padded, 1);
        // The key `"a"`, whose length is written `0x81 0x00` — one, with a
        // continuation octet carrying nothing.
        padded.extend_from_slice(&[tags::STR, 0x81, 0x00, b'a', tags::NULL]);
        assert!(read_document(&padded).is_none(), "a padded length varint");
        let mut minimal = vec![
            VERSION,
            Domain::Payload.byte(),
            lifetime::FOREVER,
            tags::ARRAY,
        ];
        write_varint(&mut minimal, 1);
        minimal.extend_from_slice(&[tags::STR, 0x01, b'a', tags::NULL]);
        let (_, _, one) = read_document(&minimal).expect("and the minimal one reads");
        dropped(one);

        dropped(held);
    }

    /// The refusals: a value with no canonical form, and a payload deeper than
    /// the stack budget both walks share.
    #[test]
    fn a_value_with_no_canonical_form_and_a_payload_too_deep_are_refused() {
        let mut deep = NvsArray::new();
        deep.append(Value::int(1));
        let mut deep = Value::array(deep);
        for _ in 0..MAX_DEPTH {
            let mut outer = NvsArray::new();
            outer.append(deep);
            deep = Value::array(outer);
        }
        let refusal = document(
            Domain::Payload,
            None,
            &deep,
            "Core\\Signature::sign",
            "$payload",
        )
        .expect_err("past the ceiling");
        assert!(
            format!("{refusal:?}").contains("nests deeper"),
            "the depth ceiling names itself: {refusal:?}"
        );

        // And the reader will not build one either, whatever authenticated it.
        let mut wire = vec![VERSION, Domain::Payload.byte(), lifetime::FOREVER];
        for _ in 0..=MAX_DEPTH {
            wire.push(tags::ARRAY);
            write_varint(&mut wire, 1);
            wire.push(tags::INT);
            wire.extend_from_slice(&0_i64.to_be_bytes());
        }
        wire.push(tags::NULL);
        assert!(
            read_document(&wire).is_none(),
            "nor does the reader recurse"
        );

        dropped(deep);
    }
}
