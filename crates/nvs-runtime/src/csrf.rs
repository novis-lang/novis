//! The CSRF token's format: a session binding sealed under one key, and the
//! constant-time comparison that is the only thing a holder of one can do.
//!
//! `rule:security/protocol-roster` puts the token behind `Core\Csrf`, and
//! `rule:security/csrf-is-on-by-default` gives it a second reader that runs
//! before any application code does — the server door, which refuses an unsafe
//! verb whose token does not verify. Those two readers are in crates that cannot
//! see each other: `nvs-stdlib` sits above `nvs-server` and is deliberately not a
//! dependency of it. **So the format lives here, under both of them**, and each
//! reader is a caller rather than a second implementation. A door that rebuilt
//! the construction would be exactly the drift the entry exists to prevent: a
//! token an application issues and a door refuses, with nothing in either crate
//! saying which of the two is wrong.
//!
//! # What a token is
//!
//! [`Key::issue`] seals [`DOMAIN`] followed by the session identifier under
//! XChaCha20-Poly1305, prefixes the nonce, and answers unpadded URL-safe base64.
//! [`Key::verify`] rebuilds the same plaintext and compares it to what opened,
//! through `subtle`, so a token that is authentic under this key but bound to
//! another session takes the same path and the same time as one that is not
//! authentic at all.
//!
//! [`DOMAIN`] is what keeps a token from being confused with another value under
//! the same key: a signed cookie carrying a session identifier would otherwise
//! *be* that session's token, and the reverse. Its version digit is part of the
//! binding, so a later change to what is bound refuses yesterday's tokens rather
//! than reading them under new rules. Its trailing NUL keeps the tag a prefix
//! rather than the head of the identifier — without it a session named `1x`
//! under `v1` and a session named `x` under a hypothetical `v11` would seal the
//! same bytes.
//!
//! # The nonce is an argument
//!
//! Drawing it is `nvs_stdlib::random`'s seam, one crate up: this tree has one
//! CSPRNG and a `#[Test(seed: …)]` reproduces a sealed message byte for byte, and
//! a module below that seam that drew its own would be a second source. The door
//! never issues, so the half that needs a nonce has exactly one caller and it is
//! the one that can reach the generator.
//!
//! # What it spends
//!
//! Per issued token, one buffer of the sealed message — the nonce, the tag,
//! [`DOMAIN`] and the identifier — and its base64, charged to the request through
//! [`crate::affordable`]. Per verification, the opened plaintext, which is
//! shorter than the token the caller is already holding, so the answer costs the
//! request nothing it has not already been charged for. Nothing is held past
//! either call: a key is not cached and no expected token is ever materialized.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use subtle::ConstantTimeEq as _;

use crate::Fault;

/// The octets a key is: what `Core\Crypto::generateKey()` answers.
pub const KEY_LEN: usize = 32;

/// `nvs_config::http` states this number too, because it refuses a `[http]
/// csrf_key` that is not a key at boot and sits below this crate, so it cannot
/// ask [`Key::new`]. This is what keeps the two from drifting.
const _: () = assert!(KEY_LEN == nvs_config::http::CSRF_KEY_BYTES);

/// XChaCha20's nonce width, which is also the token's own prefix.
pub const NONCE_LEN: usize = 24;

/// Poly1305's tag.
const TAG_LEN: usize = 16;

/// What every token's plaintext begins with, so a sealed value produced for
/// another purpose under the same key is not one of these.
pub const DOMAIN: &[u8] = b"nvs.csrf.v1\0";

/// A key a token can be issued under and verified against, and the only way to
/// reach either operation.
///
/// A newtype rather than a loose pair of functions taking `&[u8]`, so that the
/// one length check is the one place a key becomes usable: a caller holding one
/// of these has already been told whether its `bytes` was a key, and the two
/// members below have no failure mode left to describe for a key at all.
/// Its `Debug` says the type and nothing else, which is
/// `rule:security/algorithm-comes-from-the-key`'s "no API exposes the raw
/// value" applied to the one shape in which a key reaches a line something
/// might print.
pub struct Key(XChaCha20Poly1305);

impl std::fmt::Debug for Key {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Key(<secret>)")
    }
}

impl Key {
    /// The construction keyed by `key`, or `None` for a `bytes` that is not a
    /// key — anything other than [`KEY_LEN`] octets.
    #[must_use]
    pub fn new(key: &[u8]) -> Option<Self> {
        XChaCha20Poly1305::new_from_slice(key).ok().map(Self)
    }

    /// A token binding `session`, sealed under this key with the nonce it is
    /// handed.
    ///
    /// `who` names the member for the throw, spelled `Core\Class::member`.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` when the request cannot afford the sealed buffer, and
    /// one when the plaintext is past what this construction can encrypt under
    /// one nonce — the second unreachable from source, since no identifier a
    /// request can hold comes near ChaCha20's per-nonce bound.
    pub fn issue(
        &self,
        nonce: &[u8; NONCE_LEN],
        session: &str,
        who: &str,
    ) -> Result<String, Fault> {
        let plain = bound(session);
        // The seal's size is known exactly here, so the policy seam is asked
        // once with the real number rather than twice with halves of it.
        crate::affordable(Some(plain.len().saturating_add(NONCE_LEN + TAG_LEN)), who)?;

        let body = self
            .0
            .encrypt(
                &XNonce::from(*nonce),
                Payload {
                    msg: &plain,
                    aad: &[],
                },
            )
            .map_err(|_| {
                Fault::thrown(format!(
                    "{who}(): the token could not be sealed — the session identifier is larger \
                     than this construction can encrypt under one key"
                ))
            })?;

        let mut sealed = Vec::with_capacity(NONCE_LEN + body.len());
        sealed.extend_from_slice(nonce);
        sealed.extend_from_slice(&body);
        Ok(URL_SAFE_NO_PAD.encode(&sealed))
    }

    /// Whether `token` is a token this key issued for `session`.
    ///
    /// **`false` is one answer for every way of not being that**: text that is
    /// not base64, a buffer too short to hold a nonce and a tag, a tag that does
    /// not verify under this key, and an authentic token bound to some other
    /// session. A caller that could tell those apart would learn which half of a
    /// forgery attempt to fix, and the door would have a timing channel where the
    /// entry promises a comparison.
    #[must_use]
    pub fn verify(&self, token: &str, session: &str) -> bool {
        let Ok(sealed) = URL_SAFE_NO_PAD.decode(token) else {
            return false;
        };
        // `split_first_chunk` rather than a length check and a slice, so the
        // nonce arrives as a `[u8; NONCE_LEN]` and there is no second place
        // where it could be the wrong width.
        let Some((nonce, body)) = sealed.split_first_chunk::<NONCE_LEN>() else {
            return false;
        };
        if body.len() < TAG_LEN {
            return false;
        }
        let Ok(plain) = self.0.decrypt(
            &XNonce::from(*nonce),
            Payload {
                msg: body,
                aad: &[],
            },
        ) else {
            return false;
        };
        bool::from(plain.ct_eq(&bound(session)))
    }
}

/// What a token for `session` seals: the domain tag, then the identifier.
///
/// One function so that issuing and verifying cannot disagree about the binding
/// — the whole entry is that they do not.
fn bound(session: &str) -> Vec<u8> {
    let mut plain = Vec::with_capacity(DOMAIN.len() + session.len());
    plain.extend_from_slice(DOMAIN);
    plain.extend_from_slice(session.as_bytes());
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:security/protocol-roster`'s second entry, asserted over the
    /// construction: a token is bound to the session that issued it, to the key
    /// that sealed it, and to this purpose rather than to any other value under
    /// the same key.
    #[test]
    fn a_token_is_bound_to_its_session_its_key_and_this_purpose() {
        let key = Key::new(&[7_u8; KEY_LEN]).expect("a 32-octet key keys");
        let other = Key::new(&[3_u8; KEY_LEN]).expect("a 32-octet key keys");
        let token = key
            .issue(&[1_u8; NONCE_LEN], "sid-ada", "Core\\Csrf::issue")
            .expect("a short identifier seals");

        assert!(key.verify(&token, "sid-ada"));
        assert!(!key.verify(&token, "sid-grace"));
        assert!(!other.verify(&token, "sid-ada"));
        assert!(!key.verify("not base64 at all !!", "sid-ada"));
        assert!(!key.verify("", "sid-ada"));

        // The binding is in the plaintext and not beside it, which is what makes
        // a sealed value from elsewhere under this key not a token.
        assert!(bound("sid-ada").starts_with(DOMAIN));
    }

    /// Two tokens for one session differ, so a caller who compares two tokens
    /// rather than calling [`Key::verify`] gets `false` from a pair that are
    /// both valid — the failure mode the entry wants visibly broken rather than
    /// merely undocumented.
    #[test]
    fn two_tokens_for_one_session_are_different_and_both_verify() {
        let key = Key::new(&[9_u8; KEY_LEN]).expect("a 32-octet key keys");
        let first = key
            .issue(&[1_u8; NONCE_LEN], "sid-ada", "Core\\Csrf::issue")
            .expect("a short identifier seals");
        let second = key
            .issue(&[2_u8; NONCE_LEN], "sid-ada", "Core\\Csrf::issue")
            .expect("a short identifier seals");

        assert_ne!(first, second);
        assert!(key.verify(&first, "sid-ada"));
        assert!(key.verify(&second, "sid-ada"));
    }

    /// A `bytes` that is not a key never becomes one, which is what leaves both
    /// members above with no key-shaped failure to answer for.
    #[test]
    fn a_bytes_that_is_not_a_key_does_not_key() {
        assert!(Key::new(&[]).is_none());
        assert!(Key::new(&[0_u8; 31]).is_none());
        assert!(Key::new(&[0_u8; 33]).is_none());
        assert!(Key::new(&[0_u8; KEY_LEN]).is_some());
    }
}
