//! DKIM (RFC 6376): the `DKIM-Signature` header a block with a `dkim_key_file` writes on every
//! message it sends.
//!
//! The signature covers the composed message as `DATA` carries it, with relaxed canonicalization
//! of the headers and the body, and the headers on [`SIGNED`] that the message has. The key is an
//! RSA key, `rsa-sha256`, or an Ed25519 key, `ed25519-sha256` (RFC 8463), and `ring` signs with
//! both. The signing identity is the deployment's, like `from`, so no call site chooses the key,
//! the selector or the domain.
//!
//! The boot reads the key once, through [`nvs_config::mail::dkim_key`], so a key a receiver would
//! refuse never reaches a send. A send reads it again only when the file changed: [`KEYS`] keeps
//! one parsed key per key file, checked by its modification time and length. Memory: one key per
//! `dkim_key_file` the configuration names, bounded by the configuration and never by traffic.
//! Cost per message: one SHA-256 of the body, one of the signed headers, and one signature.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_config::mail::{DkimAlgorithm, dkim_key};
use nvs_runtime::{Ctx, Fault, ThrownClass};
use ring::digest::{SHA256, digest};
use ring::signature::{Ed25519KeyPair, RSA_PKCS1_SHA256, RsaKeyPair};

use super::configured;

/// The headers a signature covers, in the order `h=` names them, where the message has them.
///
/// Each is one a reader sees or a thread is built from, so a relay cannot change it without
/// breaking the signature. `Auto-Submitted` and the `Content-*` lines of the parts are not on it:
/// the body hash already covers what the parts say.
pub(super) const SIGNED: [&str; 13] = [
    "From",
    "To",
    "Cc",
    "Subject",
    "Date",
    "Message-ID",
    "MIME-Version",
    "Content-Type",
    "Reply-To",
    "In-Reply-To",
    "References",
    "List-Unsubscribe",
    "List-Unsubscribe-Post",
];

/// A key `ring` signs with.
pub(super) enum Key {
    /// An RSA key of 2048 to 4096 bits.
    Rsa(RsaKeyPair),
    /// An Ed25519 key.
    Ed25519(Ed25519KeyPair),
}

impl Key {
    /// The key in `pem`, a PKCS#8 `PRIVATE KEY`.
    ///
    /// # Errors
    ///
    /// The sentence [`dkim_key`] gives, or one naming a key `ring` cannot sign with.
    pub(super) fn of(pem: &str) -> Result<Self, String> {
        let read = dkim_key(pem)?;
        let refused = |_| "its values are not a valid key".to_owned();
        match read.algorithm {
            DkimAlgorithm::Rsa => RsaKeyPair::from_pkcs8(&read.pkcs8)
                .map(Self::Rsa)
                .map_err(refused),
            DkimAlgorithm::Ed25519 => Ed25519KeyPair::from_pkcs8_maybe_unchecked(&read.pkcs8)
                .map(Self::Ed25519)
                .map_err(refused),
        }
    }

    /// The `a=` tag's value.
    const fn algorithm(&self) -> &'static str {
        match self {
            Self::Rsa(_) => "rsa-sha256",
            Self::Ed25519(_) => "ed25519-sha256",
        }
    }

    /// The signature over `data`. Ed25519 signs the SHA-256 of `data`, as RFC 8463 says.
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, ring::error::Unspecified> {
        match self {
            Self::Rsa(pair) => {
                let mut signature = vec![0; pair.public().modulus_len()];
                pair.sign(
                    &RSA_PKCS1_SHA256,
                    &ring::rand::SystemRandom::new(),
                    data,
                    &mut signature,
                )?;
                Ok(signature)
            }
            Self::Ed25519(pair) => Ok(pair.sign(digest(&SHA256, data).as_ref()).as_ref().to_vec()),
        }
    }
}

/// What a block signs with: its key, its selector and its domain.
pub(super) struct Signer {
    /// The parsed `dkim_key_file`.
    pub(super) key: Arc<Key>,
    /// `dkim_selector`.
    pub(super) selector: String,
    /// `dkim_domain`, or the domain of `from`.
    pub(super) domain: String,
}

/// One kept key, with what the file looked like when it was read.
struct Kept {
    /// The file's modification time.
    modified: Option<SystemTime>,
    /// The file's length.
    len: u64,
    /// The key.
    key: Arc<Key>,
}

/// Every parsed key, by the path of its file.
static KEYS: Mutex<BTreeMap<PathBuf, Kept>> = Mutex::new(BTreeMap::new());

/// The signer of the block `endpoint`, whose `from` is `from`, or `None` when it has no
/// `dkim_selector` and no `dkim_key_file`.
///
/// # Errors
///
/// A catchable `RuntimeError` for a block the boot would have refused: half of the pair, or a key
/// that cannot sign. An `IOError` for a key file that cannot be read now.
pub(super) fn signer_of(
    ctx: &Ctx,
    endpoint: &str,
    from: &str,
    member: &str,
) -> Result<Option<Signer>, Fault> {
    let (selector, file) = match (
        configured(ctx, endpoint, "dkim_selector"),
        configured(ctx, endpoint, "dkim_key_file"),
    ) {
        (Some(selector), Some(file)) => (selector, file),
        (None, None) => return Ok(None),
        _ => {
            return Err(Fault::thrown(format!(
                "{member}: `[mail.{endpoint}]` sets one of `dkim_selector` and `dkim_key_file`, \
                 and a signature needs both"
            )));
        }
    };
    let domain = match configured(ctx, endpoint, "dkim_domain") {
        Some(domain) => domain,
        None => from
            .rsplit_once('@')
            .map_or("", |(_, domain)| domain)
            .to_owned(),
    };
    let key = key_at(Path::new(&file)).map_err(|(class, why)| {
        Fault::thrown_as(
            class,
            format!("{member}: `[mail.{endpoint}] dkim_key_file` cannot sign mail: {why}"),
        )
    })?;
    Ok(Some(Signer {
        key,
        selector,
        domain,
    }))
}

/// The key in the file at `path`: the kept one while the file is unchanged, else read anew.
fn key_at(path: &Path) -> Result<Arc<Key>, (ThrownClass, String)> {
    let io = |error: std::io::Error| (ThrownClass::Io, format!("it cannot be read: {error}"));
    let meta = std::fs::metadata(path).map_err(io)?;
    let modified = meta.modified().ok();
    let len = meta.len();
    {
        let keys = KEYS.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(kept) = keys.get(path)
            && kept.modified == modified
            && kept.len == len
        {
            return Ok(Arc::clone(&kept.key));
        }
    }
    let text = std::fs::read_to_string(path).map_err(io)?;
    let key = Arc::new(Key::of(&text).map_err(|why| (ThrownClass::Runtime, why))?);
    KEYS.lock().unwrap_or_else(PoisonError::into_inner).insert(
        path.to_owned(),
        Kept {
            modified,
            len,
            key: Arc::clone(&key),
        },
    );
    Ok(key)
}

/// `message` with a `DKIM-Signature` header before its first line, signed at `now`.
///
/// # Errors
///
/// A catchable `RuntimeError` when `ring` cannot sign, which a key it parsed does not do.
pub(super) fn sign(
    signer: &Signer,
    message: &str,
    now: SystemTime,
    member: &str,
) -> Result<String, Fault> {
    let (head, body) = split(message);
    let fields = fields(head);
    let mut signed = Vec::new();
    let mut data = String::new();
    for name in SIGNED {
        if let Some(field) = fields
            .iter()
            .rev()
            .find(|field| field.name.eq_ignore_ascii_case(name))
        {
            signed.push(name.to_ascii_lowercase());
            data.push_str(&relaxed_header(field.name, field.value));
        }
    }
    let body_hash = STANDARD.encode(digest(&SHA256, relaxed_body(body).as_bytes()));
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());

    // Each tag group is on a line of its own: under 78 bytes for a common domain, and under
    // RFC 5322's 998 for any domain. `b=` is last, because a receiver reads the header with its
    // value removed.
    let mut value = format!(
        "v=1; a={}; c=relaxed/relaxed;\r\n\td={}; s={};\r\n\tt={seconds}; h=",
        signer.key.algorithm(),
        signer.domain,
        signer.selector,
    );
    let mut line = format!("\tt={seconds}; h=").len();
    for (at, name) in signed.iter().enumerate() {
        let piece = if at + 1 < signed.len() {
            format!("{name}:")
        } else {
            format!("{name};")
        };
        if line + piece.len() > 76 {
            value.push_str("\r\n\t");
            line = 1;
        }
        value.push_str(&piece);
        line += piece.len();
    }
    value.push_str(&format!("\r\n\tbh={body_hash};\r\n\tb="));
    let own = relaxed_header("DKIM-Signature", &value);
    data.push_str(own.trim_end_matches("\r\n"));

    let signature = signer
        .key
        .sign(data.as_bytes())
        .map_err(|_| Fault::thrown(format!("{member}: the DKIM key could not sign the message")))?;
    let encoded = STANDARD.encode(signature);
    let folded = encoded
        .as_bytes()
        .chunks(72)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\r\n\t");
    Ok(format!("DKIM-Signature: {value}{folded}\r\n{message}"))
}

/// The header block of `message` and its body, split at the first empty line. The header block
/// keeps its last `\r\n`.
fn split(message: &str) -> (&str, &str) {
    match message.find("\r\n\r\n") {
        Some(at) => (&message[..at + 2], &message[at + 4..]),
        None => (message, ""),
    }
}

/// One header field: its name and its value, with any folding still in it.
struct Field<'a> {
    /// The name, as written.
    name: &'a str,
    /// Everything after the colon, up to the field's last `\r\n`.
    value: &'a str,
}

/// Every field of a header block, in order.
fn fields(head: &str) -> Vec<Field<'_>> {
    let mut out: Vec<Field<'_>> = Vec::new();
    let mut start = 0;
    for (at, _) in head.match_indices("\r\n") {
        let next = &head[at + 2..];
        if next.starts_with([' ', '\t']) {
            continue;
        }
        let field = &head[start..at];
        if let Some((name, value)) = field.split_once(':') {
            out.push(Field { name, value });
        }
        start = at + 2;
    }
    out
}

/// A header field in RFC 6376's relaxed form: the name in lowercase, the value unfolded, every
/// run of spaces and tabs one space, none at either end, and a `\r\n` after it.
pub(super) fn relaxed_header(name: &str, value: &str) -> String {
    let unfolded = value.replace("\r\n", "");
    let mut out = name.trim_end_matches([' ', '\t']).to_ascii_lowercase();
    out.push(':');
    out.push_str(&collapsed(&unfolded));
    out.push_str("\r\n");
    out
}

/// `text` with every run of spaces and tabs one space, and none at either end.
fn collapsed(text: &str) -> String {
    text.split([' ', '\t'])
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A body in RFC 6376's relaxed form: every run of spaces and tabs one space, none at a line's
/// end, no empty line at the end, and a `\r\n` after the last line. An empty body stays empty.
pub(super) fn relaxed_body(body: &str) -> String {
    let mut lines: Vec<String> = body
        .split("\r\n")
        .map(|line| {
            let mut out = String::with_capacity(line.len());
            let mut space = false;
            for each in line.chars() {
                if each == ' ' || each == '\t' {
                    space = true;
                } else {
                    if space {
                        out.push(' ');
                        space = false;
                    }
                    out.push(each);
                }
            }
            out
        })
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let mut out = String::with_capacity(body.len());
    for line in lines {
        out.push_str(&line);
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use ring::signature::{
        ED25519, KeyPair as _, RSA_PKCS1_1024_8192_SHA256_FOR_LEGACY_USE_ONLY,
        RSA_PKCS1_2048_8192_SHA256, UnparsedPublicKey, VerificationAlgorithm,
    };

    use super::*;

    /// An RSA key of 2048 bits, made for these tests and used nowhere else.
    const RSA_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCw5pBjblkF5z/K
2N9qmczG/N5Bsousp1L23TBqd0E66RmX4zdTFheCzlWA7n8JU/ctOqnJxUHcFBdO
mw/emPA4CaQMkHrllNvI7zQ57CBWbXLoYi2+wUCmnt2wrw7/qIrmULViuaDH2VeU
LvF8s7rkMULAZK7TMm44elYWp3ShiQ/5l6XHKlzdxhQuD3WB62KAbwLFYiPebA8b
lcVEvDd/TliVruW7OnmzZRE/pYBLW34RhyXXRHbFct7X8ETU2GstrX2DAClJUsi4
rs5PZr9+/fNJw0nceXot5cFgON4yemDDRlQNRX0e2e0UVaBWUfs+Q83J8aVeBADn
LCOTPFS5AgMBAAECggEAAlepBC+Y2a3bB7KRFTqyL/GTomYgbhK0Ig/ekp1xed9c
lTOoHRSfAJPwsy7xuzEx6XNhjHStIvC0nDDvnWMnqgsvTdYQtaZDUnHm3nGXWpWf
tdCJ4wf/H9sHu/g6WCSxKmGgtbCJ16AQN/S99AiTVpmXnAwwXXG7Jbw/22mMaAJg
j7JRw+KuuOU8PcNn0Nqzklt0cb8s43i7xIcAE4FpYnlSWfWM3yhJ1xp3pN29uL2A
nfxkCKUvStqoUhvkIcCN37BkfqU2QgdUbV/J1vK16uAl1CmyqtjJBqCR2vacTiyR
m1XIsBsn6iST+AUR/LeBl+IjwurozK/jM2bgjck2FQKBgQDVR6Q/tlCknUtf/Ki8
qFmt+E2a9DgtPpTKADUs75NTmx25/huE+hnMmR42OdNdQofOozLFK/JHFbugmLQx
hJrIJ4FXTZbcJBktY0Qp4kRPeHDJ9X0bKHSWVxtSOFXxAndk1B0fQckKZ9FzDuT+
TTho6H/Iks/gzt49guM3V3/OHQKBgQDUVYEhpeWPw987oTgbCCQLpzyURwujGM2F
lc3PzG6l/f2N+zDVUs8T04asi1Up0RSDD9zZT/aVYiFGl5sPGakeK8NrmHHcNqlM
/A0CAG4BMi77Eg5MPSY4oq/4uJ35QXyDhZnGCAf5vl6Gg2zHkB7MW+y4U8Drj3o7
JASgrrDOTQKBgDqMgzt2SicWgyWjj5LSyGOowW0jU/SftGrLqc1fK1uNPpjbeCSG
H+T5t8B9GVgFtpB0XrCgAs+e2LYalYTueSsvJSOZSiu+wSajlIfG9Ke4UYYvnRuC
fJb07KwiiP0zhyEn1aLKBW2U2Hu0gAqLmku6X7HpE6YMRBgwwT1lCNHFAoGBANHy
OVwrzG/6HB+5qOARjJJ5KUzzcm4xeAU9ueJdIIrAOJTKad6DonzYCbFRk0NlmAJ/
DL9ScV4DSSOjg43KsY1DeFZKxUoYQg38ql/wTNSh6hIWKEz5qXEWayhiYtCyD4t7
7jAAgOJPeOdHy6cGuW0W91jKd1Zy50jStECseeWBAoGBAJnssfUYb8ZZ3IBIP+pT
eqcQU3WrKUeTzlNjzkkw7hDxDzC0u/2DOzBTGN/gLDegztnsPUKCOSCQgfmuzjQP
eQAKlaiAEJndaHJGv8ppgyypwoU49wF96io1vtgSUEocm5IIzRRvjXsNy0z3YzQ+
fhtfMgPTO3Qo9MxMkrEYuNNR
-----END PRIVATE KEY-----
";

    /// RFC 8032 § 7.1's first secret key, which RFC 8463's example signs with, in PKCS#8.
    const ED25519_KEY: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";

    /// The message of RFC 8463's Appendix A.3, without its signatures.
    const RFC_8463_MESSAGE: &str = "From: Joe SixPack <joe@football.example.com>\r\n\
        To: Suzie Q <suzie@shopping.example.net>\r\n\
        Subject: Is dinner ready?\r\n\
        Date: Fri, 11 Jul 2003 21:00:37 -0700 (PDT)\r\n\
        Message-ID: <20030712040037.46341.5F8J@football.example.com>\r\n\
        \r\n\
        Hi.\r\n\
        \r\n\
        We lost the game.  Are you hungry yet?\r\n\
        \r\n\
        Joe.\r\n";

    /// A message as `compose` writes one, with every header on [`SIGNED`] and one that is not.
    fn composed() -> String {
        [
            "Date: Thu, 26 Nov 2026 10:00:00 +0000",
            "Message-ID: <0123abcd@example.com>",
            "From: \"Example Shop\" <noreply@example.com>",
            "To: <ada@example.org>,\r\n <grace@example.org>",
            "Cc: <linus@example.org>",
            "Reply-To: <help@example.com>",
            "Subject: Your order",
            "In-Reply-To: <first@example.com>",
            "References: <first@example.com>",
            "List-Unsubscribe: <https://example.com/unsubscribe>",
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click",
            "Auto-Submitted: auto-generated",
            "MIME-Version: 1.0",
            "Content-Type: text/plain; charset=UTF-8",
            "Content-Transfer-Encoding: base64",
            "",
            "SGVsbG8u",
            "",
        ]
        .join("\r\n")
    }

    /// The tags of the first `DKIM-Signature` of `signed`, with every space removed from each value.
    fn tags(signed: &str) -> BTreeMap<String, String> {
        let (head, _) = split(signed);
        let field = fields(head)
            .into_iter()
            .find(|field| field.name.eq_ignore_ascii_case("DKIM-Signature"))
            .expect("a DKIM-Signature header");
        field
            .value
            .split(';')
            .filter_map(|tag| tag.split_once('='))
            .map(|(name, value)| {
                let clean = |text: &str| text.chars().filter(|c| !c.is_whitespace()).collect();
                (clean(name), clean(value))
            })
            .collect()
    }

    /// Checks the first `DKIM-Signature` of `signed` as a receiver does, with `public` under
    /// `algorithm`: the body hash, then the signature over the headers `h=` names.
    fn verify(signed: &str, algorithm: &'static dyn VerificationAlgorithm, public: &[u8]) {
        let tags = tags(signed);
        let (head, body) = split(signed);
        let fields = fields(head);
        let body_hash = STANDARD.encode(digest(&SHA256, relaxed_body(body).as_bytes()));
        assert_eq!(tags["bh"], body_hash, "the body hash");

        let mut data = String::new();
        let mut used: BTreeMap<String, usize> = BTreeMap::new();
        for name in tags["h"].split(':') {
            let seen = used.entry(name.to_ascii_lowercase()).or_default();
            let found = fields
                .iter()
                .filter(|field| field.name.eq_ignore_ascii_case(name))
                .rev()
                .nth(*seen);
            *seen += 1;
            if let Some(field) = found {
                data.push_str(&relaxed_header(field.name, field.value));
            }
        }
        let own = fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case("DKIM-Signature"))
            .expect("a DKIM-Signature header");
        let b_at = own
            .value
            .match_indices("b=")
            .find(|(at, _)| {
                own.value[..*at]
                    .trim_end_matches([' ', '\t', '\r', '\n'])
                    .ends_with(';')
            })
            .map(|(at, _)| at)
            .expect("a b= tag");
        let emptied = &own.value[..b_at + 2];
        data.push_str(relaxed_header(own.name, emptied).trim_end_matches("\r\n"));

        let signature = STANDARD.decode(&tags["b"]).expect("b= is base64");
        let message = if tags["a"] == "ed25519-sha256" {
            digest(&SHA256, data.as_bytes()).as_ref().to_vec()
        } else {
            data.into_bytes()
        };
        UnparsedPublicKey::new(algorithm, public)
            .verify(&message, &signature)
            .expect("the signature verifies");
    }

    /// RFC 6376 § 3.4.5's example of relaxed canonicalization, header and body.
    #[test]
    fn relaxed_canonicalization_matches_rfc_6376() {
        let head = "A: X\r\nB : Y\t\r\n\tZ  \r\n";
        let body = " C \r\nD \t E\r\n\r\n\r\n";
        let canonical: String = fields(head)
            .iter()
            .map(|field| relaxed_header(field.name, field.value))
            .collect();
        assert_eq!(canonical, "a:X\r\nb:Y Z\r\n");
        assert_eq!(relaxed_body(body), " C\r\nD E\r\n");
        assert_eq!(relaxed_body(""), "");
        assert_eq!(relaxed_body("\r\n\r\n"), "");
        assert_eq!(relaxed_body("last line"), "last line\r\n");
    }

    /// Both signatures of RFC 8463's Appendix A verify through this module's canonicalization, and
    /// a message this module signs with the same Ed25519 key verifies too.
    #[test]
    fn the_rfc_8463_ed25519_vector_verifies() {
        let ed25519 = "DKIM-Signature: v=1; a=ed25519-sha256; c=relaxed/relaxed;\r\n \
             d=football.example.com; i=@football.example.com;\r\n \
             q=dns/txt; s=brisbane; t=1528637909; h=from : to :\r\n \
             subject : date : message-id : from : subject : date;\r\n \
             bh=2jUSOH9NhtVGCQWNr9BrIAPreKQjO6Sn7XIkfJVOzv8=;\r\n \
             b=/gCrinpcQOoIfuHNQIbq4pgh9kyIK3AQUdt9OdqQehSwhEIug4D11Bus\r\n \
             Fa3bT3FY5OsU7ZbnKELq+eXdp1Q1Dw==\r\n";
        let public = STANDARD
            .decode("11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=")
            .unwrap();
        verify(&format!("{ed25519}{RFC_8463_MESSAGE}"), &ED25519, &public);

        let rsa = "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed;\r\n \
             d=football.example.com; i=@football.example.com;\r\n \
             q=dns/txt; s=test; t=1528637909; h=from : to : subject :\r\n \
             date : message-id : from : subject : date;\r\n \
             bh=2jUSOH9NhtVGCQWNr9BrIAPreKQjO6Sn7XIkfJVOzv8=;\r\n \
             b=F45dVWDfMbQDGHJFlXUNB2HKfbCeLRyhDXgFpEL8GwpsRe0IeIixNTe3\r\n \
             DhCVlUrSjV4BwcVcOF6+FF3Zo9Rpo1tFOeS9mPYQTnGdaSGsgeefOsk2Jz\r\n \
             dA+L10TeYt9BgDfQNZtKdN1WO//KgIqXP7OdEFE4LjFYNcUxZQ4FADY+8=\r\n";
        // The DNS record's `p=` is a SubjectPublicKeyInfo. Its last 140 bytes are the
        // `RSAPublicKey` `ring` reads.
        let spki = STANDARD
            .decode(
                "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDkHlOQoBTzWRiGs5V6NpP3idY6Wk08a5qhdR6wy5bdOKb2jLQ\
                 iY/J16JYi0Qvx/byYzCNb3W91y3FutACDfzwQ/BC/e/8uBsCR+yz1Lxj+PL6lHvqMKrM3rG4hstT5QjvHO9Pzox\
                 ZyVYLzBfO2EeC3Ip3G+2kryOTIKT+l/K4w3QIDAQAB",
            )
            .unwrap();
        verify(
            &format!("{rsa}{RFC_8463_MESSAGE}"),
            &RSA_PKCS1_1024_8192_SHA256_FOR_LEGACY_USE_ONLY,
            &spki[spki.len() - 140..],
        );

        let Key::Ed25519(pair) = Key::of(ED25519_KEY).unwrap() else {
            panic!("the key is Ed25519");
        };
        assert_eq!(pair.public_key().as_ref(), public.as_slice());
        let signer = Signer {
            key: Arc::new(Key::Ed25519(pair)),
            selector: "brisbane".to_owned(),
            domain: "football.example.com".to_owned(),
        };
        let signed = sign(&signer, RFC_8463_MESSAGE, SystemTime::now(), "test").unwrap();
        assert_eq!(
            tags(&signed)["bh"],
            "2jUSOH9NhtVGCQWNr9BrIAPreKQjO6Sn7XIkfJVOzv8="
        );
        verify(&signed, &ED25519, &public);
    }

    /// A message signed with an RSA key verifies with its public key, and names `rsa-sha256`,
    /// relaxed canonicalization, the selector, the domain and the time.
    #[test]
    #[cfg_attr(miri, ignore = "an RSA signature takes minutes under Miri")]
    fn an_rsa_signature_verifies_with_its_public_key() {
        let key = Key::of(RSA_KEY).unwrap();
        let Key::Rsa(pair) = &key else {
            panic!("the key is RSA");
        };
        let public = pair.public().as_ref().to_vec();
        let signer = Signer {
            key: Arc::new(key),
            selector: "mail2026".to_owned(),
            domain: "example.com".to_owned(),
        };
        let at = UNIX_EPOCH + std::time::Duration::from_secs(1_795_000_000);
        let signed = sign(&signer, &composed(), at, "test").unwrap();
        assert!(signed.ends_with(&composed()));
        verify(&signed, &RSA_PKCS1_2048_8192_SHA256, &public);
        let tags = tags(&signed);
        assert_eq!(tags["a"], "rsa-sha256");
        assert_eq!(tags["c"], "relaxed/relaxed");
        assert_eq!(tags["s"], "mail2026");
        assert_eq!(tags["d"], "example.com");
        assert_eq!(tags["t"], "1795000000");
        for line in signed.split("\r\n") {
            assert!(line.len() <= 78, "a line of {} bytes: {line}", line.len());
        }

        // A changed body no longer matches the body hash the signature covers.
        let tampered = signed.replace("SGVsbG8u", "SGVsbG8h");
        let (_, body) = split(&tampered);
        let hash = STANDARD.encode(digest(&SHA256, relaxed_body(body).as_bytes()));
        assert_ne!(tags["bh"], hash);
    }

    /// `h=` names the headers on [`SIGNED`] the message has, in that order, and no other. A
    /// header the message does not have is not named.
    #[test]
    fn the_signed_headers_are_the_fixed_list() {
        let signer = Signer {
            key: Arc::new(Key::of(ED25519_KEY).unwrap()),
            selector: "s".to_owned(),
            domain: "example.com".to_owned(),
        };
        let signed = sign(&signer, &composed(), SystemTime::now(), "test").unwrap();
        assert_eq!(
            tags(&signed)["h"],
            "from:to:cc:subject:date:message-id:mime-version:content-type:reply-to:\
             in-reply-to:references:list-unsubscribe:list-unsubscribe-post"
        );

        let short = "Date: Thu, 26 Nov 2026 10:00:00 +0000\r\nMessage-ID: <a@example.com>\r\n\
                     From: <noreply@example.com>\r\nTo: <ada@example.org>\r\nSubject: Hi\r\n\
                     Auto-Submitted: auto-generated\r\nMIME-Version: 1.0\r\n\
                     Content-Type: text/plain; charset=UTF-8\r\n\r\nSGk=\r\n";
        let signed = sign(&signer, short, SystemTime::now(), "test").unwrap();
        assert_eq!(
            tags(&signed)["h"],
            "from:to:subject:date:message-id:mime-version:content-type"
        );
        assert!(!tags(&signed)["h"].contains("auto-submitted"));
    }
}
