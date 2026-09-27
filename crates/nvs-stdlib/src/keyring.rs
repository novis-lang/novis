//! The key ring every keyed member in this crate takes — `array<secret bytes>`,
//! newest first — written once so that `Core\SignedCookie` and the three doors
//! of `rule:core-api/signing-is-over-a-payload` cannot come to disagree about
//! what `$keys` means, or about what a ring that cannot key anything earns.
//!
//! # Newest first, and rotating is prepending
//!
//! `rule:security/protocol-roster` asks for key rotation in five words —
//! "verify against several keys, sign with the newest" — and every part of the
//! cost of getting it wrong is in which end of the list *newest* means. So it
//! is written into the surface rather than into a comment: **`$keys[0]` is the
//! newest**, a member that mints takes [`newest`] and nothing else, and a
//! member that checks walks [`entries`] in order. Rotating a key is prepending
//! one; retiring a key is dropping the tail.
//!
//! The alternative was a `{current, previous}` shape, which reads better at one
//! call site and stops working the moment an operator wants two overlapping
//! retirements — a real thing during a slow deploy, and the case where a
//! hand-written fallback would otherwise appear. A list has no such edge, and a
//! ring of one is the ordinary case spelled `[$key]`.
//!
//! One vocabulary is the whole reason this is a module rather than a pair of
//! private helpers in each module that needs one: a second protocol copying the
//! walk is a second protocol free to copy it slightly wrong, and the half of it
//! that is a refusal would then be a refusal a program has to learn twice.
//!
//! # A key is 32 octets at every door, even where the primitive takes any
//!
//! [`key_at`] refuses an entry that is not [`crate::crypto::KEY_LEN`] octets,
//! the same way whichever member asked. That is the construction's own number
//! where the key keys XChaCha20-Poly1305, which has exactly one key size, and
//! it is a **decision** where the key keys [`crate::hash::hmac_sha256`], which
//! accepts a key of any length at all: the ring is one ring. A program hands
//! the same `$keys` to a cookie and to a signature, so an entry that cannot key
//! both is a broken ring rather than a ring that happens to work at one door,
//! and the stricter door is the one that says so at the first request rather
//! than at the rotation months later.
//!
//! The three ways a ring is wrong are three different kinds of wrong:
//!
//! - An entry of the wrong **length** is a `LogicError`, raised before anything
//!   is checked against it — even where the message would have been refused
//!   anyway, because a ring that cannot key the construction is broken whatever
//!   arrives in it.
//! - An **empty** ring is a `LogicError` too, and reachable from source:
//!   `array<secret bytes>` says nothing about how many entries an array has,
//!   and `[]` is one of them.
//! - An entry of the wrong **tag** is a [`Fault::fatal`], because that same
//!   declared type is checked, so only compiled code that has already gone
//!   wrong can put anything else in a slot.
//!
//! # What it spends
//!
//! Nothing per entry. [`borrow`] borrows the caller's array rather than
//! separating it, and [`entries`] hands out one `Value` per live slot, borrowed
//! rather than retained (`rule:programs/memory-priority`). What a ring of `n`
//! costs is the calling member's — `n` opens, or `n` tags computed — and that
//! member's own module doc is where it is priced.

use std::mem::ManuallyDrop;

use nvs_runtime::{Fault, NvsArray, Tag, ThrownClass, Value};

use crate::registry::{CoreTy, Qual};

/// The ring's element type, written once so that every row taking a ring
/// declares the same thing: a `secret bytes`, so the array a program builds
/// out of what `Core\Crypto::generateKey()` answers is exactly the type those
/// rows want, and a plain `bytes` ring still widens onto it.
///
/// A row spells the parameter itself `CoreTy::Array(&KEY)`, and calls it
/// `$keys` — the refusals below name that spelling, so a row that renamed the
/// parameter would be a row whose messages point at nothing.
pub(crate) const KEY: CoreTy = CoreTy::SecretBlob(Qual::Neutral);

/// The key ring in slot `slot` of `args`, borrowed for the length of the call.
///
/// `who` names the member for a refusal, spelled `Core\Class::member`, as
/// [`crate::crypto::wrong_key_length`]'s does.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding anything but an array, and a
/// `LogicError` for an empty ring — the module doc is where those are two
/// different kinds of wrong. [`key_at`]'s refusals for the newest entry, which
/// is checked here so that a door refusing its input early (a verifier handed
/// no `_sig`, or two) still refuses a broken ring first, as the module doc
/// says every door does. An older entry is checked when a walk reaches it.
pub(crate) fn borrow(
    args: &[Value],
    slot: usize,
    who: &str,
) -> Result<ManuallyDrop<NvsArray>, Fault> {
    let raw = args[slot].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{who} expected {:?} for $keys, got tag {}",
            Tag::Array,
            args[slot].tag_byte()
        ))
    })?;
    let ring = crate::arr::borrowed(raw);
    if ring.next_slot(0).is_none() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{who}(): $keys is empty, and a key ring needs at least one key — \
                 Core\\Crypto::generateKey() answers one."
            ),
        ));
    }
    let (slot, held) = newest(&ring);
    key_at(&held, slot, who)?;
    Ok(ring)
}

/// Every live entry, newest first: its position in the ring, and the value it
/// holds borrowed rather than retained.
///
/// The position is the array's own slot, which for the list a program writes
/// is the index it wrote — so a refusal naming `$keys[1]` names the entry the
/// program can point at.
pub(crate) fn entries(ring: &NvsArray) -> impl Iterator<Item = (usize, Value)> + '_ {
    let mut from = 0;
    std::iter::from_fn(move || {
        let live = ring.next_slot(from)?;
        from = live + 1;
        Some((
            live,
            ring.value_at(live).expect("a live slot holds a value"),
        ))
    })
}

/// The newest key: `$keys[0]` for the list a program writes, and the first live
/// slot for one that has had its front unset.
///
/// # Panics
///
/// On an empty ring, which [`borrow`] has already refused and which nothing
/// else in this crate builds.
pub(crate) fn newest(ring: &NvsArray) -> (usize, Value) {
    entries(ring)
        .next()
        .expect("`borrow` refused an empty ring")
}

/// The key held at ring position `slot`, checked to be one.
///
/// # Errors
///
/// A [`Fault::fatal`] for an entry that is not a `bytes`, and the shared
/// `LogicError` [`crate::crypto::wrong_key_length`] writes for one that is a
/// `bytes` of the wrong length.
pub(crate) fn key_at<'a>(held: &'a Value, slot: usize, who: &str) -> Result<&'a [u8], Fault> {
    let key = held.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{who} expected a `bytes` at $keys[{slot}], got tag {}",
            held.tag_byte()
        ))
    })?;
    if key.len() == crate::crypto::KEY_LEN {
        Ok(key)
    } else {
        Err(crate::crypto::wrong_key_length(
            who,
            &format!("$keys[{slot}]"),
            key.len(),
        ))
    }
}

/// The cipher keyed by the ring entry at `slot`.
///
/// Here rather than beside either caller, for the reason this module exists at
/// all: `Core\SignedCookie` and `Core\Cache\Store`'s sealed door both walk a
/// ring, and two copies of "take the entry, check it, key the construction"
/// would be two places for what a ring entry means to drift.
///
/// # Errors
///
/// Whatever [`key_at`] refuses the entry for, which is the same refusal every
/// other door over a ring writes.
pub(crate) fn cipher_at(
    held: &Value,
    slot: usize,
    who: &str,
) -> Result<chacha20poly1305::XChaCha20Poly1305, Fault> {
    let key = key_at(held, slot, who)?;
    Ok(crate::crypto::cipher(key)
        .expect("`keyring::key_at` answers a key of the construction's own length"))
}

/// This module's own cases, and the two fixtures every other module's cases
/// build a ring with.
///
/// `pub(crate)` and holding the fixtures rather than a sibling `#[cfg(test)]`
/// block beside them, because `tests/capability.rs`'s OS-gate scan stops at
/// the first `#[cfg(test)]` in a file and asserts there is only one. Sharing
/// them at all is the rest of this module's argument applied to its tests: two
/// doors building their rings two ways is two doors testing two different
/// things.
#[cfg(test)]
pub(crate) mod tests {
    use nvs_runtime::NvsStr;

    use super::*;

    /// A ring of the keys given, as the `array<secret bytes>` a program
    /// writes.
    pub(crate) fn ring_of(keys: &[&[u8]]) -> Value {
        let mut array = NvsArray::new();
        for key in keys {
            array.append(Value::bytes(NvsStr::new(key)));
        }
        Value::array(array)
    }

    /// The ring `held` holds, borrowed for a test's own length.
    pub(crate) fn borrowed(held: &Value) -> ManuallyDrop<NvsArray> {
        crate::arr::borrowed(held.array_ptr().expect("a ring is an array"))
    }

    /// Releases what a test built; every entry belongs to the ring.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// The `LogicError` message a fault carries, or a panic naming what it was
    /// instead — a `Fault::fatal` here would mean a refusal a program cannot
    /// catch.
    fn thrown(fault: Fault) -> String {
        match fault {
            Fault::Thrown(_, message) => message.into_owned(),
            other => panic!("a program-reachable ring is refused by a throw, got {other:?}"),
        }
    }

    /// The walk is what a door reads the ring through, so what it must promise
    /// is that the newest key is the first one and that nothing live is
    /// skipped — asserted over a ring with a retired key unset out of its
    /// middle, because that is where a walk counting its own steps and a walk
    /// reading slots stop agreeing.
    #[test]
    fn a_ring_walk_starts_at_the_newest_and_names_each_key_by_its_own_slot() {
        let mut array = NvsArray::new();
        for key in [&[1_u8; 32], &[2_u8; 32], &[3_u8; 32]] {
            array.append(Value::bytes(NvsStr::new(key)));
        }
        array.unset(b"1");
        let held = Value::array(array);
        let ring = borrowed(&held);

        let walked: Vec<(usize, u8)> = entries(&ring)
            .map(|(slot, key)| {
                (
                    slot,
                    key_at(&key, slot, "Core\\Signature::sign").expect("every entry is a key")[0],
                )
            })
            .collect();
        assert_eq!(
            walked,
            [(0, 1), (2, 3)],
            "the walk visits every live slot in order, under the array's own positions"
        );
        assert_eq!(
            newest(&ring).0,
            0,
            "the newest key is the ring's first live slot"
        );

        dropped(held);
    }

    /// One refusal, whichever door asks: the two ways a ring is unusable are
    /// the same sentence under `Core\Signature` as under `Core\SignedCookie`,
    /// differing only in the member that names itself. A door that grew its own
    /// copy of the walk fails here while still reading correctly on its own
    /// line.
    #[test]
    fn every_door_refuses_an_unusable_ring_with_the_same_two_sentences() {
        let doors = ["Core\\SignedCookie::seal", "Core\\Signature::sign"];

        let empty = ring_of(&[]);
        let stunted = ring_of(&[&[7_u8; 8]]);
        let refusals: Vec<(String, String)> = doors
            .iter()
            .map(|who| {
                let no_keys = thrown(
                    borrow(&[Value::null(), empty], 1, who).expect_err("an empty ring is refused"),
                );
                let ring = borrowed(&stunted);
                let (slot, key) = newest(&ring);
                let not_a_key =
                    thrown(key_at(&key, slot, who).expect_err("an 8-octet bytes is not a key"));
                (
                    no_keys.replace(who, "<door>"),
                    not_a_key.replace(who, "<door>"),
                )
            })
            .collect();

        assert_eq!(
            refusals[0], refusals[1],
            "the ring's refusals name the member and are otherwise one sentence each"
        );
        assert!(
            refusals[0].0.contains("$keys is empty"),
            "the empty-ring refusal says so: {}",
            refusals[0].0
        );
        assert!(
            refusals[0].1.contains("is 8 octets, and a key is 32"),
            "the wrong-length refusal names both lengths: {}",
            refusals[0].1
        );

        dropped(empty);
        dropped(stunted);
    }

    /// The 32-octet bound, asserted on both sides at once: a key one octet
    /// short and one octet long are refused and the length between them is
    /// accepted, so a check written `>=` or `<=` fails here rather than on the
    /// day a ring carries a longer key.
    #[test]
    fn a_key_is_exactly_thirty_two_octets_and_the_lengths_either_side_are_refused() {
        for len in [31, 32, 33] {
            let key = vec![5_u8; len];
            let ring = ring_of(&[&key]);
            let (slot, key) = newest(&borrowed(&ring));
            let answer = key_at(&key, slot, "Core\\Signature::sign");
            assert_eq!(
                answer.is_ok(),
                len == crate::crypto::KEY_LEN,
                "a key of {len} octets: only {} is one",
                crate::crypto::KEY_LEN
            );
            dropped(ring);
        }
    }

    /// A door that refuses its input before it walks the ring still refuses a
    /// broken ring first, because [`borrow`] checks the newest entry itself:
    /// `Core\Router::signedRoute` handed a request with two `_sig` parameters
    /// answered the forgery sentence for a ring whose first key was 31 octets.
    /// An older entry of the wrong length is left to the walk that reaches it.
    // covers: Core\Router::signedRoute
    #[test]
    fn borrowing_a_ring_refuses_a_newest_key_of_the_wrong_length_and_nothing_older() {
        let who = "Core\\Router::signedRoute";
        let short = ring_of(&[&[5_u8; 31]]);
        let refusal = thrown(
            borrow(&[short], 0, who).expect_err("a 31-octet newest key is refused on borrow"),
        );
        assert!(
            refusal.contains("is 31 octets, and a key is 32"),
            "the refusal is key_at's own sentence: {refusal}"
        );

        let retired_short = ring_of(&[&[5_u8; 32], &[6_u8; 31]]);
        assert!(
            borrow(&[retired_short], 0, who).is_ok(),
            "an older entry is checked by the walk that reaches it, not on borrow"
        );

        dropped(short);
        dropped(retired_short);
    }
}
