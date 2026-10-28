//! C2SP signed notes under an Ed25519 key, and the log checkpoint a note carries
//! (`rule:packaging/the-registry-keeps-an-append-only-log`).
//!
//! A note is a text that ends in a newline, a blank line, and one or more signature lines
//! `— <name> base64(key id || signature)`. A key is named by its name and its key id, the first
//! four bytes of SHA-256 over the name, a newline, the type byte `0x01` and the public key. A
//! verifier key is written `<name>+<hex id>+base64(0x01 || public key)`, and a key file holds
//! the signer key `PRIVATE+KEY+<name>+<hex id>+base64(0x01 || seed)`, both as Go's
//! `golang.org/x/mod/sumdb/note` writes them, so a key made by either tool works in the other.
//!
//! [`open`] follows C2SP's verifier rules. A signature from a key the caller did not pin is
//! ignored, even when its name or id alone matches one. A signature from a pinned key that does
//! not verify fails the whole note, and a note with no verified signature fails. The text is
//! returned only from a note that opened.
//!
//! A [`Checkpoint`] is the note text C2SP tlog-checkpoint defines: the log's origin, its size
//! and its root hash, one per line. Any other text the registry signs goes through the same
//! [`sign`] and [`open`].
//!
//! **Memory:** a note is read whole, and opening one holds its text and the verifiers that
//! signed it. Nothing is kept once the call returns.

// `nvs registry log` is this module's first caller outside its tests.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "no command signs or opens a note yet")
)]

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ring::rand::{SecureRandom as _, SystemRandom};
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair as _, UnparsedPublicKey};
use sha2::{Digest as _, Sha256};

use super::log::{Hash, empty_root};

/// The signature type byte C2SP assigns to Ed25519.
const ALG_ED25519: u8 = 0x01;

/// The bytes in an Ed25519 public key or seed.
const KEY_SIZE: usize = 32;

/// The most signature lines a note may have. C2SP requires at least 16, and Go reads 100.
const MAX_SIGNATURES: usize = 100;

/// What starts a signer key, before its name.
const SIGNER_PREFIX: &str = "PRIVATE+KEY+";

/// What starts a signature line, before the key name.
const SIGNATURE_PREFIX: &str = "— ";

/// Why a key, a note or a checkpoint was not accepted.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    /// A verifier key or signer key is not in the C2SP form, or its id does not match its key.
    MalformedKey(&'static str),
    /// A note is not in the C2SP form.
    MalformedNote(&'static str),
    /// A note's text is not a checkpoint.
    MalformedCheckpoint(&'static str),
    /// More than one pinned key has the name and id a signature line names.
    AmbiguousKey { name: String, id: u32 },
    /// A signature from a pinned key does not verify.
    BadSignature { name: String, id: u32 },
    /// No signature in the note is from a pinned key.
    Unverified,
    /// A checkpoint is for another log.
    WrongOrigin { want: String, found: String },
    /// The system's random source failed while a key was made.
    Random,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedKey(why) => write!(f, "malformed key: {why}"),
            Self::MalformedNote(why) => write!(f, "malformed note: {why}"),
            Self::MalformedCheckpoint(why) => write!(f, "malformed checkpoint: {why}"),
            Self::AmbiguousKey { name, id } => write!(f, "ambiguous key {name}+{id:08x}"),
            Self::BadSignature { name, id } => {
                write!(f, "the signature of key {name}+{id:08x} does not verify")
            }
            Self::Unverified => f.write_str("the note has no signature from a known key"),
            Self::WrongOrigin { want, found } => {
                write!(f, "the checkpoint is for log `{found}`, not `{want}`")
            }
            Self::Random => f.write_str("the system's random source failed"),
        }
    }
}

/// The key id of a key: the first four bytes, big-endian, of SHA-256 over the name, a newline,
/// and the type byte with the public key.
fn key_id(name: &str, typed_key: &[u8]) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\n");
    hasher.update(typed_key);
    let digest = hasher.finalize();
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// Whether `name` may name a key: not empty, and no space or `+`.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| c == '+' || c.is_whitespace())
}

/// Whether `text` has no control character but the newline.
fn is_valid_text(text: &str) -> bool {
    !text.chars().any(|c| c < ' ' && c != '\n')
}

/// Splits `<name>+<hex id>+<base64>` and checks the name, the id's form and the type byte.
/// Returns the name, the id and the 32 key bytes after the type byte.
fn split_key(key: &str) -> Result<(&str, u32, [u8; KEY_SIZE]), Error> {
    // The base64 alphabet has `+` in it, so only the first two `+` separate.
    let mut parts = key.splitn(3, '+');
    let (Some(name), Some(id), Some(encoded)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(Error::MalformedKey("not three parts joined by `+`"));
    };
    if !is_valid_name(name) {
        return Err(Error::MalformedKey(
            "the name is empty or has a space or `+`",
        ));
    }
    if id.len() != 8 || !id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(Error::MalformedKey(
            "the key id is not eight lowercase hex digits",
        ));
    }
    let id = u32::from_str_radix(id, 16).map_err(|_| Error::MalformedKey("bad key id"))?;
    let typed = BASE64
        .decode(encoded)
        .map_err(|_| Error::MalformedKey("the key is not canonical base64"))?;
    let [alg, rest @ ..] = typed.as_slice() else {
        return Err(Error::MalformedKey("the key is empty"));
    };
    if *alg != ALG_ED25519 {
        return Err(Error::MalformedKey("the key is not an Ed25519 key"));
    }
    let bytes = <[u8; KEY_SIZE]>::try_from(rest)
        .map_err(|_| Error::MalformedKey("an Ed25519 key is 32 bytes"))?;
    Ok((name, id, bytes))
}

/// `0x01` followed by `key`.
fn typed(key: &[u8; KEY_SIZE]) -> [u8; KEY_SIZE + 1] {
    let mut out = [ALG_ED25519; KEY_SIZE + 1];
    out[1..].copy_from_slice(key);
    out
}

/// A pinned public key that a note's signature is checked against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Verifier {
    name: String,
    id: u32,
    key: [u8; KEY_SIZE],
}

impl Verifier {
    /// Reads a verifier key `<name>+<hex id>+base64(0x01 || public key)`, and checks that the
    /// id is the key's.
    pub(crate) fn parse(vkey: &str) -> Result<Self, Error> {
        let (name, id, key) = split_key(vkey)?;
        if key_id(name, &typed(&key)) != id {
            return Err(Error::MalformedKey("the key id is not this key's"));
        }
        Ok(Self {
            name: name.to_owned(),
            id,
            key,
        })
    }

    /// The key's name, which a signature line repeats.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The key's id, the first four bytes of each of its signatures.
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    fn verifies(&self, message: &[u8], signature: &[u8]) -> bool {
        UnparsedPublicKey::new(&ED25519, &self.key)
            .verify(message, signature)
            .is_ok()
    }
}

impl fmt::Display for Verifier {
    /// The verifier key in the form [`Verifier::parse`] reads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}+{:08x}+{}",
            self.name,
            self.id,
            BASE64.encode(typed(&self.key))
        )
    }
}

/// A private key that signs notes.
pub(crate) struct Signer {
    name: String,
    id: u32,
    pair: Ed25519KeyPair,
}

impl fmt::Debug for Signer {
    // The seed is never printed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signer")
            .field("name", &self.name)
            .field("id", &self.id)
            .finish()
    }
}

impl Signer {
    /// Reads a signer key `PRIVATE+KEY+<name>+<hex id>+base64(0x01 || seed)`, and checks
    /// that the id is the id of the public key the seed gives.
    pub(crate) fn parse(skey: &str) -> Result<Self, Error> {
        let rest = skey.strip_prefix(SIGNER_PREFIX).ok_or(Error::MalformedKey(
            "a signer key starts with `PRIVATE+KEY+`",
        ))?;
        let (name, id, seed) = split_key(rest)?;
        let signer = Self::from_seed(name, &seed)?;
        if signer.id != id {
            return Err(Error::MalformedKey("the key id is not this key's"));
        }
        Ok(signer)
    }

    /// A new key named `name`, and the signer key that [`Signer::parse`] reads it back from.
    pub(crate) fn generate(name: &str) -> Result<(Self, String), Error> {
        if !is_valid_name(name) {
            return Err(Error::MalformedKey(
                "the name is empty or has a space or `+`",
            ));
        }
        let mut seed = [0; KEY_SIZE];
        SystemRandom::new()
            .fill(&mut seed)
            .map_err(|_| Error::Random)?;
        let signer = Self::from_seed(name, &seed)?;
        let skey = format!(
            "{SIGNER_PREFIX}{name}+{:08x}+{}",
            signer.id,
            BASE64.encode(typed(&seed))
        );
        Ok((signer, skey))
    }

    fn from_seed(name: &str, seed: &[u8; KEY_SIZE]) -> Result<Self, Error> {
        let pair = Ed25519KeyPair::from_seed_unchecked(seed)
            .map_err(|_| Error::MalformedKey("the seed is not an Ed25519 seed"))?;
        let public = <[u8; KEY_SIZE]>::try_from(pair.public_key().as_ref())
            .map_err(|_| Error::MalformedKey("an Ed25519 key is 32 bytes"))?;
        let id = key_id(name, &typed(&public));
        Ok(Self {
            name: name.to_owned(),
            id,
            pair,
        })
    }

    /// The verifier key that checks this key's signatures.
    pub(crate) fn verifier(&self) -> Verifier {
        let mut key = [0; KEY_SIZE];
        key.copy_from_slice(self.pair.public_key().as_ref());
        Verifier {
            name: self.name.clone(),
            id: self.id,
            key,
        }
    }
}

/// Signs `text` with `signer`: the text, a blank line and one signature line.
/// The text must end in a newline and have no control character but the newline.
pub(crate) fn sign(text: &str, signer: &Signer) -> Result<String, Error> {
    if !text.ends_with('\n') {
        return Err(Error::MalformedNote("the text does not end in a newline"));
    }
    if !is_valid_text(text) {
        return Err(Error::MalformedNote("the text has a control character"));
    }
    let mut signature = signer.id.to_be_bytes().to_vec();
    signature.extend_from_slice(signer.pair.sign(text.as_bytes()).as_ref());
    Ok(format!(
        "{text}\n{SIGNATURE_PREFIX}{} {}\n",
        signer.name,
        BASE64.encode(signature)
    ))
}

/// A note that [`open`] accepted.
#[derive(Debug)]
pub(crate) struct Opened<'a> {
    /// The text, with its final newline.
    pub(crate) text: &'a str,
    /// Each pinned key with a signature that verified, once each, in the note's order.
    pub(crate) signed_by: Vec<Verifier>,
    /// The signature lines from keys that were not pinned.
    pub(crate) ignored: usize,
}

/// Checks `note`'s signatures against the pinned `verifiers`, and returns its text.
pub(crate) fn open<'a>(note: &'a [u8], verifiers: &[Verifier]) -> Result<Opened<'a>, Error> {
    let note = std::str::from_utf8(note).map_err(|_| Error::MalformedNote("not UTF-8"))?;
    if !is_valid_text(note) {
        return Err(Error::MalformedNote("a control character"));
    }
    // The text may have blank lines of its own. The last one separates the signatures.
    let split = note
        .rfind("\n\n")
        .ok_or(Error::MalformedNote("no blank line before the signatures"))?;
    let (text, signatures) = (&note[..=split], &note[split + 2..]);
    let signatures = signatures.strip_suffix('\n').ok_or(Error::MalformedNote(
        "no signature line, or one without a newline",
    ))?;
    let lines: Vec<&str> = signatures.split('\n').collect();
    if lines.len() > MAX_SIGNATURES {
        return Err(Error::MalformedNote("too many signatures"));
    }

    let mut signed_by: Vec<Verifier> = Vec::new();
    let mut ignored = 0;
    for line in lines {
        let rest = line
            .strip_prefix(SIGNATURE_PREFIX)
            .ok_or(Error::MalformedNote(
                "a signature line does not start with `— `",
            ))?;
        let (name, encoded) = rest
            .split_once(' ')
            .ok_or(Error::MalformedNote("a signature line has no signature"))?;
        if !is_valid_name(name) {
            return Err(Error::MalformedNote("a key name is empty or has a `+`"));
        }
        let bytes = BASE64
            .decode(encoded)
            .map_err(|_| Error::MalformedNote("a signature is not canonical base64"))?;
        let [a, b, c, d, signature @ ..] = bytes.as_slice() else {
            return Err(Error::MalformedNote(
                "a signature is shorter than its key id",
            ));
        };
        if signature.is_empty() {
            return Err(Error::MalformedNote("a signature is empty"));
        }
        let id = u32::from_be_bytes([*a, *b, *c, *d]);

        let mut matching = verifiers.iter().filter(|v| v.name == name && v.id == id);
        let Some(verifier) = matching.next() else {
            ignored += 1;
            continue;
        };
        if matching.next().is_some() {
            return Err(Error::AmbiguousKey {
                name: name.to_owned(),
                id,
            });
        }
        if !verifier.verifies(text.as_bytes(), signature) {
            return Err(Error::BadSignature {
                name: name.to_owned(),
                id,
            });
        }
        if !signed_by.contains(verifier) {
            signed_by.push(verifier.clone());
        }
    }
    if signed_by.is_empty() {
        return Err(Error::Unverified);
    }
    Ok(Opened {
        text,
        signed_by,
        ignored,
    })
}

/// A log's state as a checkpoint states it: the log's origin, its size and its root hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Checkpoint {
    origin: String,
    size: u64,
    root: Hash,
}

impl Checkpoint {
    /// A checkpoint of the log `origin` at `size` records with root hash `root`. The origin
    /// is one line of text that is not empty.
    pub(crate) fn new(origin: &str, size: u64, root: Hash) -> Result<Self, Error> {
        if origin.is_empty() || origin.contains('\n') || !is_valid_text(origin) {
            return Err(Error::MalformedCheckpoint(
                "the origin is not one line of text",
            ));
        }
        Ok(Self {
            origin: origin.to_owned(),
            size,
            root,
        })
    }

    /// The checkpoint of the log `origin` with no records.
    pub(crate) fn empty(origin: &str) -> Result<Self, Error> {
        Self::new(origin, 0, empty_root())
    }

    pub(crate) fn origin(&self) -> &str {
        &self.origin
    }

    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    pub(crate) fn root(&self) -> &Hash {
        &self.root
    }

    /// The note text: the origin, the size in decimal and the base64 root hash, one per line.
    pub(crate) fn body(&self) -> String {
        format!(
            "{}\n{}\n{}\n",
            self.origin,
            self.size,
            BASE64.encode(self.root)
        )
    }

    /// Reads a checkpoint's note text. Lines after the root hash are extensions, and are
    /// skipped.
    pub(crate) fn parse(text: &str) -> Result<Self, Error> {
        let body = text.strip_suffix('\n').ok_or(Error::MalformedCheckpoint(
            "the text does not end in a newline",
        ))?;
        let mut lines = body.split('\n');
        let (Some(origin), Some(size), Some(root)) = (lines.next(), lines.next(), lines.next())
        else {
            return Err(Error::MalformedCheckpoint("fewer than three lines"));
        };
        if lines.any(str::is_empty) {
            return Err(Error::MalformedCheckpoint("an empty extension line"));
        }
        let canonical = !size.is_empty()
            && size.bytes().all(|b| b.is_ascii_digit())
            && (size == "0" || !size.starts_with('0'));
        let size = canonical
            .then(|| size.parse::<u64>().ok())
            .flatten()
            .ok_or(Error::MalformedCheckpoint(
                "the size is not a decimal number",
            ))?;
        let root = BASE64
            .decode(root)
            .ok()
            .and_then(|bytes| Hash::try_from(bytes).ok())
            .ok_or(Error::MalformedCheckpoint(
                "the root is not a base64 SHA-256 hash",
            ))?;
        Self::new(origin, size, root)
    }

    /// The checkpoint as a note signed by `signer`.
    pub(crate) fn sign(&self, signer: &Signer) -> Result<String, Error> {
        sign(&self.body(), signer)
    }

    /// Opens `note` with the pinned `verifiers` and reads the checkpoint in it, which must be
    /// for the log `origin`.
    pub(crate) fn open(note: &[u8], origin: &str, verifiers: &[Verifier]) -> Result<Self, Error> {
        let checkpoint = Self::parse(open(note, verifiers)?.text)?;
        if checkpoint.origin != origin {
            return Err(Error::WrongOrigin {
                want: origin.to_owned(),
                found: checkpoint.origin,
            });
        }
        Ok(checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Vector {
        vkey: String,
        skey: Option<String>,
        text: String,
        note: String,
    }

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../tests/fixtures/tlog/note.json"))
            .expect("the vectors parse")
    }

    fn vectors() -> Vec<Vector> {
        let text = |value: &serde_json::Value| value.as_str().expect("a string").to_owned();
        fixture()["notes"]
            .as_array()
            .expect("a list of notes")
            .iter()
            .map(|note| Vector {
                vkey: text(&note["vkey"]),
                skey: note["skey"].as_str().map(str::to_owned),
                text: text(&note["text"]),
                note: text(&note["note"]),
            })
            .collect()
    }

    /// Go's `PeterNeumann` vector, whose signer key is known.
    fn peter() -> Vector {
        vectors()
            .into_iter()
            .find(|v| v.skey.is_some())
            .expect("a vector with a signer key")
    }

    /// Go's `EnochRoot` vector, the second key that signs the same text.
    fn enoch() -> Vector {
        vectors()
            .into_iter()
            .find(|v| v.vkey.starts_with("EnochRoot+"))
            .expect("Enoch's vector")
    }

    /// The signature line of a note with one signature.
    fn signature_line(vector: &Vector) -> &str {
        &vector.note[vector.text.len() + 1..]
    }

    fn verifier(vector: &Vector) -> Verifier {
        Verifier::parse(&vector.vkey).expect("the vector's key parses")
    }

    #[test]
    fn each_vector_verifies_under_its_key() {
        for vector in vectors() {
            let key = verifier(&vector);
            assert_eq!(
                key.to_string(),
                vector.vkey,
                "a verifier key writes back unchanged"
            );
            let opened = open(vector.note.as_bytes(), std::slice::from_ref(&key))
                .unwrap_or_else(|e| panic!("{}: {e}", vector.vkey));
            assert_eq!(opened.text, vector.text);
            assert_eq!(opened.signed_by, [key]);
            assert_eq!(opened.ignored, 0);
        }
    }

    #[test]
    fn the_signer_key_signs_its_vector_byte_for_byte() {
        // Ed25519 is deterministic, so signing the text again gives the vector's note.
        let vector = peter();
        let signer = Signer::parse(vector.skey.as_deref().unwrap()).expect("the signer key parses");
        assert_eq!(signer.verifier(), verifier(&vector));
        assert_eq!(sign(&vector.text, &signer).unwrap(), vector.note);
    }

    #[test]
    fn a_mangled_key_is_refused() {
        // Go's TestNewVerifier and TestNewSigner: every shorter piece of a key, and the key
        // with any one byte changed.
        let vector = peter();
        let skey = vector.skey.clone().unwrap();
        for (key, parse) in [
            (
                vector.vkey.clone(),
                (|k: &str| Verifier::parse(k).is_ok()) as fn(&str) -> bool,
            ),
            (skey, |k: &str| Signer::parse(k).is_ok()),
        ] {
            assert!(parse(&key));
            let bytes = key.as_bytes();
            for start in 0..bytes.len() {
                for end in start + 1..=bytes.len() {
                    if (start, end) != (0, bytes.len()) {
                        assert!(!parse(&key[start..end]), "{:?} parsed", &key[start..end]);
                    }
                }
            }
            for at in 0..bytes.len() {
                let mut changed = bytes.to_vec();
                changed[at] += 1;
                let changed = String::from_utf8(changed).unwrap();
                assert!(!parse(&changed), "{changed:?} parsed");
            }
        }
        for bad in fixture()["bad_vkeys"].as_array().unwrap() {
            assert!(
                Verifier::parse(bad.as_str().unwrap()).is_err(),
                "{bad} parsed"
            );
        }
    }

    #[test]
    fn a_generated_key_reads_back_and_verifies_its_own_notes() {
        let (signer, skey) = Signer::generate("example.com/log").unwrap();
        let read = Signer::parse(&skey).unwrap();
        assert_eq!(read.verifier(), signer.verifier());
        let note = sign("hi\n", &read).unwrap();
        let opened = open(note.as_bytes(), &[signer.verifier()]).unwrap();
        assert_eq!(opened.text, "hi\n");
        assert!(Signer::generate("a b").is_err());
    }

    #[test]
    fn a_changed_byte_fails_the_note() {
        let vector = peter();
        let key = verifier(&vector);
        let bad = |name: &str| Error::BadSignature {
            name: name.to_owned(),
            id: key.id(),
        };

        // A byte of the text.
        let mut note = vector.note.clone().into_bytes();
        note[3] ^= 0x20;
        assert_eq!(
            open(&note, std::slice::from_ref(&key)).unwrap_err(),
            bad(key.name())
        );

        // Go's TestOpen: four base64 characters inserted into the signature, past the key id.
        let line = signature_line(&vector);
        let note = format!("{}\n{}ABCD{}", vector.text, &line[..60], &line[60..]);
        assert_eq!(
            open(note.as_bytes(), std::slice::from_ref(&key)).unwrap_err(),
            bad(key.name())
        );
    }

    #[test]
    fn a_signature_from_an_unknown_key_id_is_ignored() {
        // Go's wrongHash: the first base64 characters, and so the key id, changed.
        let vector = peter();
        let note = vector.note.replace("x08g", "xxxx");
        assert_eq!(
            open(note.as_bytes(), &[verifier(&vector)]).unwrap_err(),
            Error::Unverified
        );
        // The same name with a known id is ignored too.
        let note = vector.note.replace("PeterNeumann", "CarmenSandiego");
        assert_eq!(
            open(note.as_bytes(), &[verifier(&vector)]).unwrap_err(),
            Error::Unverified
        );
    }

    #[test]
    fn a_second_signature_from_an_unknown_key_is_ignored() {
        let (peter, enoch) = (peter(), enoch());
        let note = format!(
            "{}\n{}{}",
            peter.text,
            signature_line(&peter),
            signature_line(&enoch)
        );
        let opened = open(note.as_bytes(), &[verifier(&peter)]).unwrap();
        assert_eq!(opened.text, peter.text);
        assert_eq!(opened.signed_by, [verifier(&peter)]);
        assert_eq!(opened.ignored, 1);

        let both = [verifier(&peter), verifier(&enoch)];
        let opened = open(note.as_bytes(), &both).unwrap();
        assert_eq!(opened.signed_by, both);
        assert_eq!(opened.ignored, 0);

        assert_eq!(open(note.as_bytes(), &[]).unwrap_err(), Error::Unverified);
    }

    #[test]
    fn a_repeated_signature_counts_once_and_a_repeated_key_is_ambiguous() {
        let (peter, enoch) = (peter(), enoch());
        let peter_line = signature_line(&peter);
        let note = format!("{}\n{peter_line}{peter_line}", peter.text);
        let opened = open(note.as_bytes(), &[verifier(&peter)]).unwrap();
        assert_eq!(opened.signed_by.len(), 1);

        let keys = [verifier(&enoch), verifier(&peter), verifier(&enoch)];
        let note = format!("{}\n{}", enoch.text, signature_line(&enoch));
        assert!(matches!(
            open(note.as_bytes(), &keys),
            Err(Error::AmbiguousKey { .. })
        ));
        // A key repeated in the list but not named by any signature is not an error.
        open(peter.note.as_bytes(), &keys).unwrap();
    }

    #[test]
    fn a_malformed_note_is_refused() {
        let vector = peter();
        let (text, line) = (vector.text.as_str(), signature_line(&vector));
        let key = [verifier(&vector)];
        let mut bad: Vec<Vec<u8>> = [
            text.to_owned(),
            format!("{text}\n"),
            format!("{text}\n{}", &line[..line.len() - 1]),
            format!("\x01{text}\n{line}"),
            format!("{text}\n— Bad Name {}", &line["— PeterNeumann ".len()..]),
            format!("{text}\n{line}Unexpected line.\n"),
            format!("{text}\n{}", line.repeat(MAX_SIGNATURES + 1)),
        ]
        .into_iter()
        .map(String::into_bytes)
        .collect();
        bad.push([b"\xff".as_slice(), vector.note.as_bytes()].concat());
        for note in bad {
            let error = open(&note, &key).unwrap_err();
            assert!(
                matches!(error, Error::MalformedNote(_)),
                "{note:?}: {error}"
            );
        }
    }

    #[test]
    fn a_checkpoint_signs_and_opens_under_its_origin() {
        let signer = Signer::parse(peter().skey.as_deref().unwrap()).unwrap();
        let checkpoint = Checkpoint::new("example.com/log", 7, [7; 32]).unwrap();
        assert_eq!(
            checkpoint.body(),
            "example.com/log\n7\nBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=\n"
        );
        let note = checkpoint.sign(&signer).unwrap();
        let keys = [signer.verifier()];
        assert_eq!(
            Checkpoint::open(note.as_bytes(), "example.com/log", &keys),
            Ok(checkpoint)
        );
        assert_eq!(
            Checkpoint::open(note.as_bytes(), "example.com/other", &keys).unwrap_err(),
            Error::WrongOrigin {
                want: "example.com/other".to_owned(),
                found: "example.com/log".to_owned()
            }
        );

        let empty = Checkpoint::empty("example.com/log").unwrap();
        assert_eq!(Checkpoint::parse(&empty.body()), Ok(empty.clone()));
        assert_eq!(
            (empty.origin(), empty.size(), empty.root()),
            ("example.com/log", 0, &empty_root())
        );
    }

    #[test]
    fn a_checkpoint_text_is_read_strictly() {
        let root = BASE64.encode([1; 32]);
        let parsed = Checkpoint::parse(&format!("o\n12\n{root}\nan extension\n")).unwrap();
        assert_eq!(parsed.size(), 12);
        for bad in [
            format!("o\n12\n{root}"),
            "o\n12\n".to_owned(),
            format!("\n12\n{root}\n"),
            format!("o\n012\n{root}\n"),
            format!("o\n+12\n{root}\n"),
            format!("o\n18446744073709551616\n{root}\n"),
            format!("o\n12\n{}\n", BASE64.encode([1; 31])),
            format!("o\n12\n{root}\n\n"),
        ] {
            assert!(Checkpoint::parse(&bad).is_err(), "{bad:?} parsed");
        }
        assert!(Checkpoint::new("two\nlines", 0, [0; 32]).is_err());
    }
}
