//! The password: the finished file encrypted with AES-256 by the standard security handler's
//! revision 6 (`rule:core-classes/pdf-standards-and-passwords`), because `krilla` 0.8 writes no
//! encryption.
//!
//! [`encrypt`] rewrites the file `write` returns. Every string of every object becomes a hex
//! string of its ciphertext, and every stream's data its ciphertext with the stream's `/Length`
//! written again. Each is AES-256-CBC under one 32-byte file key, its 16-byte IV before it and
//! PKCS#7 padding after, as `/V 5` with the `AESV3` crypt filter says. The `/Encrypt` dictionary is
//! a new object after the last one, the trailer names it, and the cross-reference table is
//! written again with every object's new offset. The trailer's `/ID` is not encrypted, as the
//! standard says, and neither is anything in `/Encrypt`.
//!
//! The password is both the user and the owner password, so whoever has it may do everything,
//! and `/P` allows everything. `/U`, `/UE`, `/O`, `/OE` and `/Perms` are Algorithms 8, 9 and 10
//! of ISO 32000-2, over [`hash`], its Algorithm 2.B. The password is its UTF-8 bytes cut to 127;
//! SASLprep is not applied, so a password SASLprep would change opens in a reader only when it
//! is typed in the form the program gave.
//!
//! **Equal input is byte-equal output.** Nothing here draws a random byte. The file key is a
//! SHA-256 of the password and the unencrypted file, and the salts, the `/Perms` filler and every
//! IV are SHA-256s of the file key and what each one is for. They are as hard to guess as the
//! key, and no two strings or streams share an IV.
//!
//! The file is read the way `pdf-writer` writes it: a cross-reference table, not a stream, each
//! object at the offset the table gives, and each stream's `/Length` a direct integer. A file of
//! any other shape returns `Runtime`.
//!
//! Cost: Algorithm 2.B's 64 or more rounds of AES and SHA-2 over the password, four times, and one
//! AES pass over the file. Memory: one more copy of the file, inside the request's cap.

use aes::cipher::{BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Aes256, Block};
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::Error;

/// Every permission: bits 1 and 2 clear, every other bit set.
const PERMISSIONS: i32 = -4;

/// `file` encrypted under `password`.
pub fn encrypt(file: &[u8], password: &str) -> Result<Vec<u8>, Error> {
    let lost = |what: &str| Error::Runtime(format!("the written file has no {what}"));
    let password = &password.as_bytes()[..password.len().min(127)];
    let key = sha256(&[
        b"nvs-pdf file key",
        &(password.len() as u64).to_be_bytes(),
        password,
        file,
    ]);
    let seal = Seal {
        aes: Aes256::new(&key.into()),
        key,
    };

    // The cross-reference table: `xref`, `0 N`, then N entries of 20 bytes each.
    let start = rfind(file, b"startxref\n").ok_or_else(|| lost("startxref"))?;
    let xref =
        number(file, start + b"startxref\n".len()).ok_or_else(|| lost("cross-reference offset"))?;
    if !file
        .get(xref..)
        .is_some_and(|rest| rest.starts_with(b"xref\n0 "))
    {
        return Err(lost("cross-reference table"));
    }
    let count_at = xref + b"xref\n0 ".len();
    let count = number(file, count_at).ok_or_else(|| lost("cross-reference count"))?;
    let entries_at = count_at + count.to_string().len() + 1;
    let trailer_at = entries_at + count * 20;
    let trailer = file
        .get(trailer_at..start)
        .filter(|trailer| trailer.starts_with(b"trailer"))
        .ok_or_else(|| lost("trailer"))?;
    let entries = &file[entries_at..trailer_at];

    // Each object in use, in the order of its offset.
    let mut used: Vec<(usize, usize)> = Vec::new();
    for (index, entry) in entries.chunks(20).enumerate() {
        if entry[17] == b'n' {
            let offset = number(entry, 0).ok_or_else(|| lost("cross-reference entry"))?;
            used.push((offset, index));
        }
    }
    used.sort_unstable();
    let first = used.first().map_or(xref, |&(offset, _)| offset);

    let mut out = Vec::with_capacity(file.len() + file.len() / 8 + 1024);
    out.extend_from_slice(&file[..first]);
    let mut moved = vec![0usize; count];
    for (at, &(offset, index)) in used.iter().enumerate() {
        let end = used.get(at + 1).map_or(xref, |&(next, _)| next);
        let region = file
            .get(offset..end)
            .ok_or_else(|| lost("object in place"))?;
        moved[index] = out.len();
        object(region, index as u64, &seal, &mut out)?;
    }

    let encrypt_number = count;
    let encrypt_at = out.len();
    out.extend_from_slice(dictionary(encrypt_number, password, &seal).as_bytes());

    let new_xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", count + 1).as_bytes());
    for (index, entry) in entries.chunks(20).enumerate() {
        if entry[17] == b'n' {
            out.extend_from_slice(format!("{:010}", moved[index]).as_bytes());
            out.extend_from_slice(&entry[10..]);
        } else {
            out.extend_from_slice(entry);
        }
    }
    out.extend_from_slice(format!("{encrypt_at:010} 00000 n\r\n").as_bytes());

    let size = find(trailer, b"/Size ").ok_or_else(|| lost("trailer size"))? + b"/Size ".len();
    let digits = count_digits(trailer, size);
    let close = rfind(trailer, b">>").ok_or_else(|| lost("trailer end"))?;
    out.extend_from_slice(&trailer[..size]);
    out.extend_from_slice((count + 1).to_string().as_bytes());
    out.extend_from_slice(&trailer[size + digits..close]);
    out.extend_from_slice(format!("  /Encrypt {encrypt_number} 0 R\n").as_bytes());
    out.extend_from_slice(&trailer[close..]);
    out.extend_from_slice(format!("startxref\n{new_xref}\n%%EOF").as_bytes());
    Ok(out)
}

/// The `/Encrypt` dictionary as object `number`, for `password` and the file key `seal` holds.
fn dictionary(number: usize, password: &[u8], seal: &Seal) -> String {
    let salts = sha256(&[b"nvs-pdf salts", &seal.key]);
    let (user_validation, user_key) = (&salts[0..8], &salts[8..16]);
    let (owner_validation, owner_key) = (&salts[16..24], &salts[24..32]);

    let mut user = hash(password, user_validation, &[]).to_vec();
    user.extend_from_slice(user_validation);
    user.extend_from_slice(user_key);
    let user_encrypted = wrap(&hash(password, user_key, &[]), &seal.key);

    let mut owner = hash(password, owner_validation, &user).to_vec();
    owner.extend_from_slice(owner_validation);
    owner.extend_from_slice(owner_key);
    let owner_encrypted = wrap(&hash(password, owner_key, &user), &seal.key);

    let mut perms = [0u8; 16];
    perms[..4].copy_from_slice(&PERMISSIONS.to_le_bytes());
    perms[4..8].copy_from_slice(&[0xff; 4]);
    perms[8..12].copy_from_slice(b"Tadb");
    perms[12..].copy_from_slice(&sha256(&[b"nvs-pdf perms", &seal.key])[..4]);
    let perms = wrap(&seal.key, &perms);

    format!(
        "{number} 0 obj\n<<\n  /Filter /Standard\n  /V 5\n  /R 6\n  /Length 256\n  \
         /CF << /StdCF << /Type /CryptFilter /CFM /AESV3 /AuthEvent /DocOpen /Length 32 >> >>\n  \
         /StmF /StdCF\n  /StrF /StdCF\n  /O <{}>\n  /U <{}>\n  /OE <{}>\n  /UE <{}>\n  \
         /P {PERMISSIONS}\n  /Perms <{}>\n  /EncryptMetadata true\n>>\nendobj\n\n",
        hex(&owner),
        hex(&user),
        hex(&owner_encrypted),
        hex(&user_encrypted),
        hex(&perms),
    )
}

/// ISO 32000-2's Algorithm 2.B: the hash of `password` with `salt` and, for the owner's
/// entries, the 48-byte `/U` as `user`.
fn hash(password: &[u8], salt: &[u8], user: &[u8]) -> [u8; 32] {
    let mut k: Vec<u8> = sha256(&[password, salt, user]).to_vec();
    let mut round: u32 = 0;
    loop {
        let mut k1 = Vec::with_capacity(64 * (password.len() + k.len() + user.len()));
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(user);
        }
        let aes = Aes128::new(&block(&k[..16]));
        let e = cbc(|block| aes.encrypt_block(block), &k[16..32], &k1);
        // 256 is 1 modulo 3, so the first 16 bytes as one big-endian number are their sum, modulo 3.
        let sum: u32 = e[..16].iter().map(|&byte| u32::from(byte)).sum();
        k = match sum % 3 {
            0 => Sha256::digest(&e).to_vec(),
            1 => Sha384::digest(&e).to_vec(),
            _ => Sha512::digest(&e).to_vec(),
        };
        round += 1;
        let last = e.last().copied().map_or(0, u32::from);
        if round >= 64 && last <= round - 32 {
            break;
        }
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&k[..32]);
    out
}

/// `data`, a whole number of blocks, encrypted with AES-256 under `key`, CBC with a zero IV and no
/// padding, which for one block is ECB.
fn wrap(key: &[u8; 32], data: &[u8]) -> Vec<u8> {
    let aes = Aes256::new(&(*key).into());
    cbc(|block| aes.encrypt_block(block), &[0; 16], data)
}

/// `data`, a whole number of blocks, encrypted block by block with `encrypt` in CBC mode from `iv`.
fn cbc(encrypt: impl Fn(&mut Block), iv: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut previous = block(iv);
    for chunk in data.chunks(16) {
        let mut next = block(chunk);
        for (byte, before) in next.iter_mut().zip(previous.iter()) {
            *byte ^= before;
        }
        encrypt(&mut next);
        out.extend_from_slice(&next);
        previous = next;
    }
    out
}

/// The first 16 bytes of `bytes` as one AES block, zero-filled when there are fewer.
fn block(bytes: &[u8]) -> Block {
    let mut out = Block::default();
    let len = bytes.len().min(16);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

/// The file key, and what encrypts each string and stream under it.
struct Seal {
    key: [u8; 32],
    aes: Aes256,
}

impl Seal {
    /// `data` as the `index`th string or stream of object `number` stores it: its IV, then the
    /// data padded with PKCS#7 and encrypted.
    fn seal(&self, number: u64, index: u64, data: &[u8]) -> Vec<u8> {
        let iv = sha256(&[
            b"nvs-pdf iv",
            &self.key,
            &number.to_be_bytes(),
            &index.to_be_bytes(),
        ]);
        let pad = 16 - data.len() % 16;
        let mut padded = Vec::with_capacity(data.len() + pad);
        padded.extend_from_slice(data);
        padded.resize(data.len() + pad, pad as u8);
        let mut out = Vec::with_capacity(16 + padded.len());
        out.extend_from_slice(&iv[..16]);
        out.extend(cbc(
            |block| self.aes.encrypt_block(block),
            &iv[..16],
            &padded,
        ));
        out
    }
}

/// Object `number`, which `region` holds from its `obj` line to the next object, written to
/// `out` with its strings and its stream encrypted.
fn object(region: &[u8], number: u64, seal: &Seal, out: &mut Vec<u8>) -> Result<(), Error> {
    let bad = || {
        Error::Runtime(format!(
            "object {number} of the written file does not parse"
        ))
    };
    let header = find(region, b" obj").ok_or_else(bad)? + b" obj".len();
    out.extend_from_slice(&region[..header]);
    let mut at = header;
    let mut dicts = 0usize;
    let mut arrays = 0usize;
    let mut length_key = false;
    // Where the stream's `/Length` is in `out`, its digits' count and its value.
    let mut length: Option<(usize, usize, usize)> = None;
    let mut index = 0u64;
    while let Some(&byte) = region.get(at) {
        if is_space(byte) {
            out.push(byte);
            at += 1;
            continue;
        }
        let after_length_key = std::mem::take(&mut length_key);
        match byte {
            b'(' => {
                let (bytes, end) = literal(region, at).ok_or_else(bad)?;
                push_hex(out, &seal.seal(number, index, &bytes));
                index += 1;
                at = end;
            }
            b'<' if region.get(at + 1) == Some(&b'<') => {
                out.extend_from_slice(b"<<");
                dicts += 1;
                at += 2;
            }
            b'<' => {
                let (bytes, end) = hex_string(region, at).ok_or_else(bad)?;
                push_hex(out, &seal.seal(number, index, &bytes));
                index += 1;
                at = end;
            }
            b'>' if region.get(at + 1) == Some(&b'>') => {
                out.extend_from_slice(b">>");
                dicts = dicts.checked_sub(1).ok_or_else(bad)?;
                at += 2;
            }
            b'[' | b']' => {
                if byte == b'[' {
                    arrays += 1;
                } else {
                    arrays = arrays.checked_sub(1).ok_or_else(bad)?;
                }
                out.push(byte);
                at += 1;
            }
            b'%' => {
                while let Some(&byte) = region.get(at).filter(|&&b| b != b'\n' && b != b'\r') {
                    out.push(byte);
                    at += 1;
                }
            }
            _ => {
                let end = at
                    + 1
                    + region[at + 1..]
                        .iter()
                        .take_while(|&&b| is_regular(b))
                        .count();
                let token = &region[at..end];
                if token == b"/Length" && dicts == 1 && arrays == 0 {
                    length_key = true;
                } else if after_length_key {
                    let value = number_of(token).ok_or_else(bad)?;
                    length = Some((out.len(), token.len(), value));
                } else if token == b"stream" {
                    let (place, digits, len) = length.ok_or_else(bad)?;
                    let mut data_at = end;
                    if region.get(data_at) == Some(&b'\r') {
                        data_at += 1;
                    }
                    if region.get(data_at) != Some(&b'\n') {
                        return Err(bad());
                    }
                    data_at += 1;
                    let data = region.get(data_at..data_at + len).ok_or_else(bad)?;
                    let mut rest = data_at + len;
                    while region.get(rest).is_some_and(|&b| is_space(b)) {
                        rest += 1;
                    }
                    if !region[rest..].starts_with(b"endstream") {
                        return Err(bad());
                    }
                    let sealed = seal.seal(number, index, data);
                    index += 1;
                    out.splice(place..place + digits, sealed.len().to_string().into_bytes());
                    out.extend_from_slice(b"stream\n");
                    out.extend_from_slice(&sealed);
                    out.extend_from_slice(b"\nendstream");
                    at = rest + b"endstream".len();
                    continue;
                } else if token == b"endobj" {
                    out.extend_from_slice(&region[at..]);
                    return Ok(());
                }
                out.extend_from_slice(token);
                at = end;
            }
        }
    }
    Err(bad())
}

/// The bytes of the literal string that opens at `start`, and where it ends.
fn literal(src: &[u8], start: usize) -> Option<(Vec<u8>, usize)> {
    let mut at = start + 1;
    let mut depth = 1usize;
    let mut bytes = Vec::new();
    loop {
        let byte = *src.get(at)?;
        at += 1;
        match byte {
            b'(' => {
                depth += 1;
                bytes.push(byte);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((bytes, at));
                }
                bytes.push(byte);
            }
            b'\\' => {
                let next = *src.get(at)?;
                at += 1;
                match next {
                    b'n' => bytes.push(b'\n'),
                    b'r' => bytes.push(b'\r'),
                    b't' => bytes.push(b'\t'),
                    b'b' => bytes.push(0x08),
                    b'f' => bytes.push(0x0c),
                    b'0'..=b'7' => {
                        let mut value = u32::from(next - b'0');
                        for _ in 0..2 {
                            match src.get(at) {
                                Some(&digit @ b'0'..=b'7') => {
                                    value = value * 8 + u32::from(digit - b'0');
                                    at += 1;
                                }
                                _ => break,
                            }
                        }
                        bytes.push(value as u8);
                    }
                    b'\r' => {
                        if src.get(at) == Some(&b'\n') {
                            at += 1;
                        }
                    }
                    b'\n' => {}
                    other => bytes.push(other),
                }
            }
            b'\r' => {
                if src.get(at) == Some(&b'\n') {
                    at += 1;
                }
                bytes.push(b'\n');
            }
            other => bytes.push(other),
        }
    }
}

/// The bytes of the hex string that opens at `start`, and where it ends.
fn hex_string(src: &[u8], start: usize) -> Option<(Vec<u8>, usize)> {
    let mut at = start + 1;
    let mut digits = Vec::new();
    loop {
        let byte = *src.get(at)?;
        at += 1;
        match byte {
            b'>' => break,
            _ if is_space(byte) => {}
            _ => digits.push(char::from(byte).to_digit(16)? as u8),
        }
    }
    if digits.len() % 2 == 1 {
        digits.push(0);
    }
    let bytes = digits
        .chunks(2)
        .map(|pair| pair[0] << 4 | pair[1])
        .collect();
    Some((bytes, at))
}

fn is_space(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

fn is_regular(byte: u8) -> bool {
    !is_space(byte) && !b"()<>[]{}/%".contains(&byte)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn push_hex(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'<');
    out.extend_from_slice(hex(bytes).as_bytes());
    out.push(b'>');
}

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// The decimal number at `at` in `bytes`.
fn number(bytes: &[u8], at: usize) -> Option<usize> {
    let digits = count_digits(bytes, at);
    number_of(bytes.get(at..at + digits)?)
}

fn number_of(token: &[u8]) -> Option<usize> {
    std::str::from_utf8(token).ok()?.parse().ok()
}

fn count_digits(bytes: &[u8], at: usize) -> usize {
    bytes.get(at..).map_or(0, |rest| {
        rest.iter().take_while(|b| b.is_ascii_digit()).count()
    })
}

/// Where `needle` first occurs in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Where `needle` last occurs in `haystack`.
fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}
