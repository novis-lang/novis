// `bun nv webcrypto-vectors`: the frozen WebCrypto vector set goal `webcrypto` replays, written by the
// browser's own API.
//
//     bun nv webcrypto-vectors            rewrite crates/nvs-stdlib/tests/vectors/webcrypto.json
//     bun nv webcrypto-vectors --check    exit 1 if the file on disk is not what this writes
//
// Every value in the file comes out of `globalThis.crypto.subtle` -- the W3C Web Cryptography API, the
// same interface a browser exposes -- so the loop proves browser interop by replaying the file and never
// runs this command. A human fires it; no check runs it. X25519 agreement is the one value taken from
// elsewhere: Bun's `crypto.subtle` imports and exports X25519 keys but will not derive with them, so
// `agree` exports both keys and hands them to `node:crypto`'s `diffieHellman`, the same X25519 function,
// which refuses the same low-order points WebCrypto does.
//
// **Every input is derived from its label** (`fixed` below: SHA-256 of `label#0`, `label#1`, ...), keys
// and nonces included, so running the command twice writes the same bytes and a diff of the file is a
// diff of the vectors. Two things WebCrypto will not derive are handled here rather than drawn: an RSA
// key is built from label-derived primes (`rsaKey`), and an ES256 or PS256 signature, which is
// randomized by its algorithm, is kept from the file on disk for as long as it still verifies over an
// unchanged signing input (`jws`).
//
// WebCrypto has no JWE and no JWS, so the tokens are assembled here from WebCrypto's primitives exactly
// as RFC 7515/7516/7518 describe. `jweOpen` takes each JWE apart again by a separate path, `referee`
// judges every JWS by the rules `Core\Jwt::verifyIssued` holds, and RFC 7518 Appendix C's published
// Concat KDF output is reproduced before anything is written.
//
// The JWE headers are written canonically -- members sorted, no whitespace, at every level -- because
// that is the one form `Core\Jwe::encrypt` can be held to byte for byte.

import { createPrivateKey, createPublicKey, diffieHellman, type webcrypto } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { deflateRawSync, inflateRawSync } from "node:zlib";
import { ROOT } from "../lib/paths.ts";

export const summary = "rewrite the frozen WebCrypto vector set, or --check that it is current: nv webcrypto-vectors [--check]";

type Bytes = Uint8Array<ArrayBuffer>;
type Obj = Record<string, unknown>;
type Jwk = webcrypto.JsonWebKey;
type Algorithm = webcrypto.Algorithm;
type EcKeyImportParams = webcrypto.EcKeyImportParams;
type EcdsaParams = webcrypto.EcdsaParams;
type RsaHashedImportParams = webcrypto.RsaHashedImportParams;
type RsaPssParams = webcrypto.RsaPssParams;

const subtle = globalThis.crypto.subtle;
const OUT = join(ROOT, "crates", "nvs-stdlib", "tests", "vectors", "webcrypto.json");

const utf8 = new TextEncoder();
const hex = (b: Uint8Array | ArrayBuffer): string => Buffer.from(b instanceof ArrayBuffer ? new Uint8Array(b) : b).toString("hex");
const unhex = (s: string): Bytes => new Uint8Array(Buffer.from(s, "hex"));
const b64u = (b: Uint8Array): string => Buffer.from(b).toString("base64url");
const unb64u = (s: string): Bytes => new Uint8Array(Buffer.from(s, "base64url"));
const str = (v: unknown): string => String(v);
const EMPTY: Bytes = new Uint8Array(0);

function cat(...parts: Uint8Array[]): Bytes {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

function u32(n: number): Bytes {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, n);
  return out;
}

// Members sorted at every level, no whitespace.
function canon(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(canon).join(",")}]`;
  if (v && typeof v === "object") {
    const o = v as Obj;
    return `{${Object.keys(o).sort().map((k) => `${JSON.stringify(k)}:${canon(o[k])}`).join(",")}}`;
  }
  return JSON.stringify(v);
}

function assert(ok: boolean, what: string): void {
  if (!ok) throw new Error(`self-check failed: ${what}`);
}

async function fixed(label: string, n: number): Promise<Bytes> {
  const out: number[] = [];
  for (let i = 0; out.length < n; i++) {
    out.push(...new Uint8Array(await subtle.digest("SHA-256", utf8.encode(`${label}#${i}`))));
  }
  return new Uint8Array(out.slice(0, n));
}

// A public JWK in the member order a browser's `exportKey('jwk', publicKey)` writes it, which is the
// order Node writes: `key_ops` and `ext` first, then the key's own members. Bun sorts the members and
// leaves out an Ed25519 key's `alg`, which the Secure Curves specification sets to `Ed25519`.
function browserJwk(jwk: Jwk): string {
  const own: Record<string, string[]> = {
    EC: ["alg", "kty", "x", "y", "crv"],
    OKP: ["alg", "crv", "x", "kty"],
    RSA: ["alg", "kty", "n", "e"],
  };
  const order = ["key_ops", "ext", ...(own[str(jwk.kty)] ?? [])];
  const withAlg: Obj = jwk.crv === "Ed25519" ? { alg: "Ed25519", ...jwk } : { ...jwk };
  const out: Obj = {};
  for (const k of order) if (k in withAlg) out[k] = withAlg[k];
  for (const k of Object.keys(withAlg)) if (!(k in out)) out[k] = withAlg[k];
  return JSON.stringify(out);
}

// ---------------------------------------------------------------------------------------------------
// The primitives, each one WebCrypto call.
// ---------------------------------------------------------------------------------------------------

// WebCrypto answers `ciphertext ‖ tag`; the nonce is the caller's to carry.
async function gcmEncrypt(key: Bytes, nonce: Bytes, plaintext: Bytes, aad?: Bytes): Promise<Bytes> {
  const k = await subtle.importKey("raw", key, "AES-GCM", false, ["encrypt"]);
  const params = { name: "AES-GCM", iv: nonce, tagLength: 128, ...(aad ? { additionalData: aad } : {}) };
  return new Uint8Array(await subtle.encrypt(params, k, plaintext));
}

async function gcmDecrypt(key: Bytes, nonce: Bytes, ciphertextAndTag: Bytes, aad?: Bytes): Promise<Bytes> {
  const k = await subtle.importKey("raw", key, "AES-GCM", false, ["decrypt"]);
  const params = { name: "AES-GCM", iv: nonce, tagLength: 128, ...(aad ? { additionalData: aad } : {}) };
  return new Uint8Array(await subtle.decrypt(params, k, ciphertextAndTag));
}

async function refuses(promise: Promise<unknown>): Promise<boolean> {
  try {
    await promise;
    return false;
  } catch {
    return true;
  }
}

async function pbkdf2(password: Bytes, salt: Bytes, iterations: number, bits: number): Promise<Bytes> {
  const k = await subtle.importKey("raw", password, "PBKDF2", false, ["deriveBits"]);
  return new Uint8Array(await subtle.deriveBits({ name: "PBKDF2", hash: "SHA-256", salt, iterations }, k, bits));
}

async function hkdf(ikm: Bytes, salt: Bytes, info: Bytes, bits: number): Promise<Bytes> {
  const k = await subtle.importKey("raw", ikm, "HKDF", false, ["deriveBits"]);
  return new Uint8Array(await subtle.deriveBits({ name: "HKDF", hash: "SHA-256", salt, info }, k, bits));
}

async function kwWrap(kek: Bytes, keyBytes: Bytes): Promise<Bytes> {
  const k = await subtle.importKey("raw", kek, "AES-KW", false, ["wrapKey"]);
  const wrapped = await subtle.importKey("raw", keyBytes, "AES-GCM", true, ["encrypt"]);
  return new Uint8Array(await subtle.wrapKey("raw", wrapped, k, "AES-KW"));
}

async function kwUnwrap(kek: Bytes, wrapped: Bytes): Promise<Bytes> {
  const k = await subtle.importKey("raw", kek, "AES-KW", false, ["unwrapKey"]);
  const key = await subtle.unwrapKey("raw", wrapped, k, "AES-KW", "AES-GCM", true, ["encrypt"]);
  return new Uint8Array(await subtle.exportKey("raw", key));
}

// ---------------------------------------------------------------------------------------------------
// Key pairs, built from a fixed scalar through PKCS#8 so nothing about them is random.
// ---------------------------------------------------------------------------------------------------

type Curve = "P-256" | "X25519";

const CURVE: Record<Curve, { algorithm: EcKeyImportParams | Algorithm; pkcs8Prefix: Bytes }> = {
  "P-256": {
    algorithm: { name: "ECDH", namedCurve: "P-256" },
    // PKCS#8 around an RFC 5915 ECPrivateKey with no public key in it; WebCrypto computes that half.
    pkcs8Prefix: unhex("3041020100301306072a8648ce3d020106082a8648ce3d030107042730250201010420"),
  },
  X25519: {
    algorithm: { name: "X25519" },
    // PKCS#8 around RFC 8410's CurvePrivateKey.
    pkcs8Prefix: unhex("302e020100300506032b656e04220420"),
  },
};

interface KeyPair {
  curve: Curve;
  priv: CryptoKey;
  pub: CryptoKey;
  minimal: Obj;
  record: { scalar: string; pkcs8: string; raw: string; spki: string; jwk: string; jwkMinimal: string };
}

async function keyPair(curve: Curve, scalar: Bytes): Promise<KeyPair> {
  const { algorithm, pkcs8Prefix } = CURVE[curve];
  const priv = await subtle.importKey("pkcs8", cat(pkcs8Prefix, scalar), algorithm, true, ["deriveBits"]);
  const { d: _d, key_ops: _ops, ext: _ext, ...publicJwk } = await subtle.exportKey("jwk", priv);
  const pub = await subtle.importKey("jwk", publicJwk, algorithm, true, []);
  const minimal: Obj = curve === "P-256"
    ? { crv: publicJwk.crv, kty: publicJwk.kty, x: publicJwk.x, y: publicJwk.y }
    : { crv: publicJwk.crv, kty: publicJwk.kty, x: publicJwk.x };
  return {
    curve,
    priv,
    pub,
    minimal,
    record: {
      scalar: hex(scalar),
      pkcs8: hex(await subtle.exportKey("pkcs8", priv)),
      raw: hex(await subtle.exportKey("raw", pub)),
      spki: hex(await subtle.exportKey("spki", pub)),
      // Exactly what a browser's `exportKey('jwk', publicKey)` hands a program, `ext` and `key_ops` included.
      jwk: browserJwk(await subtle.exportKey("jwk", pub)),
      // RFC 7638's required members, sorted: the form `PublicKey::write(KeyFormat::Jwk)` answers.
      jwkMinimal: canon(minimal),
    },
  };
}

async function agree(curve: Curve, priv: CryptoKey, pub: CryptoKey): Promise<Bytes> {
  if (curve === "X25519") {
    const privateKey = createPrivateKey({ key: Buffer.from(await subtle.exportKey("pkcs8", priv)), format: "der", type: "pkcs8" });
    const publicKey = createPublicKey({ key: Buffer.from(await subtle.exportKey("spki", pub)), format: "der", type: "spki" });
    return new Uint8Array(diffieHellman({ privateKey, publicKey }));
  }
  return new Uint8Array(await subtle.deriveBits({ name: "ECDH", public: pub }, priv, 256));
}

// RFC 7518 § 4.6.2: SHA-256 over counter ‖ Z ‖ OtherInfo, one round per 256 bits.
async function concatKdf(z: Bytes, algorithmId: string, apu: Bytes, apv: Bytes, bits: number): Promise<Bytes> {
  const lp = (b: Bytes): Bytes => cat(u32(b.length), b);
  const otherInfo = cat(lp(utf8.encode(algorithmId)), lp(apu), lp(apv), u32(bits));
  const out: number[] = [];
  for (let counter = 1; out.length * 8 < bits; counter++) {
    out.push(...new Uint8Array(await subtle.digest("SHA-256", cat(u32(counter), z, otherInfo))));
  }
  return new Uint8Array(out.slice(0, bits / 8));
}

// ---------------------------------------------------------------------------------------------------
// JWE compact serialization, RFC 7516 § 7.1: the AAD is the ASCII of the encoded protected header.
// ---------------------------------------------------------------------------------------------------

const PBES2 = "PBES2-HS256+A128KW";

function pbes2Salt(p2s: Bytes): Bytes {
  return cat(utf8.encode(PBES2), new Uint8Array([0]), p2s);
}

async function jweSeal(header: Obj, encryptedKey: Bytes, cek: Bytes, iv: Bytes, plaintext: Bytes): Promise<string> {
  const protectedHeader = b64u(utf8.encode(canon(header)));
  const sealed = await gcmEncrypt(cek, iv, plaintext, utf8.encode(protectedHeader));
  return [protectedHeader, b64u(encryptedKey), b64u(iv), b64u(sealed.slice(0, -16)), b64u(sealed.slice(-16))].join(".");
}

interface Opener {
  shared?: Bytes;
  password?: string;
  priv?: CryptoKey;
}

// Takes a token apart by the header it carries and applies no policy at all -- it is how this command
// proves a refusal vector is well-formed except for the one thing `Core\Jwe` must refuse it for.
async function jweOpen(token: string, key: Opener): Promise<Bytes> {
  const [h = "", ek = "", iv = "", ct = "", tag = ""] = token.split(".");
  const header = JSON.parse(Buffer.from(h, "base64url").toString("utf8")) as Obj;
  const bits = header.enc === "A128GCM" ? 128 : 256;
  let cek: Bytes | undefined;
  if (header.alg === "dir" || header.alg === "none") {
    cek = key.shared;
  } else if (header.alg === "ECDH-ES") {
    const epkJwk = header.epk as Jwk;
    const curve = epkJwk.crv as Curve;
    const epk = await subtle.importKey("jwk", epkJwk, CURVE[curve].algorithm, true, []);
    cek = await concatKdf(await agree(curve, key.priv as CryptoKey, epk), str(header.enc), EMPTY, EMPTY, bits);
  } else if (header.alg === PBES2) {
    const kek = await pbkdf2(utf8.encode(str(key.password)), pbes2Salt(unb64u(str(header.p2s))), Number(header.p2c), 128);
    cek = await kwUnwrap(kek, unb64u(ek));
  }
  if (cek === undefined) throw new Error(`no key for alg ${str(header.alg)}`);
  let plaintext = await gcmDecrypt(cek, unb64u(iv), cat(unb64u(ct), unb64u(tag)), utf8.encode(h));
  if (header.zip === "DEF") plaintext = new Uint8Array(inflateRawSync(plaintext));
  return plaintext;
}

function flipFirstBit(b64: string): string {
  const b = unb64u(b64);
  b[0] = (b[0] ?? 0) ^ 0x01;
  return b64u(b);
}

function flipLast(b: Bytes): Bytes {
  const out = b.slice();
  out[out.length - 1] = (out[out.length - 1] ?? 0) ^ 0x01;
  return out;
}

function flipAt(b: Bytes, at: number): Bytes {
  const out = b.slice();
  out[at] = (out[at] ?? 0) ^ 0x01;
  return out;
}

// ---------------------------------------------------------------------------------------------------
// RFC 7518 Appendix C, reproduced before anything is written: the Concat KDF this command uses is the
// one the RFC publishes an answer for.
// ---------------------------------------------------------------------------------------------------

async function rfc7518AppendixC(): Promise<void> {
  const alg = CURVE["P-256"].algorithm;
  const alice = await subtle.importKey("jwk", {
    kty: "EC", crv: "P-256",
    x: "gI0GAILBdu7T53akrFmMyGcsF3n5dO7MmwNBHKW5SV0",
    y: "SLW_xSffzlPWrHEVI30DHM_4egVwt3NQqeUD7nMFpps",
    d: "0_NxaRPUMQoAJt50Gz8YiTr8gRTwyEaCumd-MToTmIo",
  }, alg, false, ["deriveBits"]);
  const bob = await subtle.importKey("jwk", {
    kty: "EC", crv: "P-256",
    x: "weNJy2HscCSM6AEDTDg04biOvhFhyyWvOHQfeF_PxMQ",
    y: "e8lnCO-AlStT-NJVX-crhB7QRYhiix03illJOVAOyck",
  }, alg, false, []);
  const key = await concatKdf(await agree("P-256", alice, bob), "A128GCM", utf8.encode("Alice"), utf8.encode("Bob"), 128);
  assert(b64u(key) === "VqqN6vgjbSBcIijNcacQGg", "RFC 7518 Appendix C Concat KDF output");
}

// ---------------------------------------------------------------------------------------------------
// The set.
// ---------------------------------------------------------------------------------------------------

async function aesGcmVectors(): Promise<Obj> {
  const key = await fixed("aes-gcm key", 32);
  const vectors: { name: string; key: string; nonce: string; plaintext: string; sealed: string }[] = [];
  for (const length of [0, 1, 15, 16, 17, 64, 1000]) {
    const nonce = await fixed(`aes-gcm nonce ${length}`, 12);
    const plaintext = await fixed(`aes-gcm plaintext ${length}`, length);
    const sealed = cat(nonce, await gcmEncrypt(key, nonce, plaintext));
    assert(hex(await gcmDecrypt(key, nonce, sealed.slice(12))) === hex(plaintext), `aes-gcm ${length} round trip`);
    vectors.push({ name: `a ${length}-octet message`, key: hex(key), nonce: hex(nonce), plaintext: hex(plaintext), sealed: hex(sealed) });
  }
  const good = unhex(vectors[4]?.sealed ?? "");
  const refusals = [
    { name: "the last tag octet flipped", key: hex(key), sealed: hex(flipLast(good)) },
    { name: "the first ciphertext octet flipped", key: hex(key), sealed: hex(flipAt(good, 12)) },
    { name: "the first nonce octet flipped", key: hex(key), sealed: hex(flipAt(good, 0)) },
    { name: "one octet short of a nonce and a tag", key: hex(key), sealed: hex(good.slice(0, 27)) },
    { name: "the right bytes under another key", key: hex(await fixed("aes-gcm other key", 32)), sealed: hex(good) },
  ];
  for (const r of refusals) {
    const s = unhex(r.sealed);
    assert(s.length < 28 || await refuses(gcmDecrypt(unhex(r.key), s.slice(0, 12), s.slice(12))), `aes-gcm refusal: ${r.name}`);
  }
  return { vectors, refusals };
}

async function pbkdf2Vectors(): Promise<Obj> {
  const vectors: Obj[] = [];
  for (const [name, password] of [["an ASCII passphrase", "correct horse battery staple"], ["a non-ASCII password", "pässwörd 🔑 密码"]] as const) {
    const salt = await fixed(`pbkdf2 salt ${name}`, 16);
    const key = await pbkdf2(utf8.encode(password), salt, 100000, 256);
    vectors.push({ name, password, salt: hex(salt), iterations: 100000, key: hex(key) });
  }
  const salt = await fixed("pbkdf2 refusal salt", 16);
  const refusals = [
    { name: "one iteration under the floor", password: "correct horse battery staple", salt: hex(salt), iterations: 99999 },
    { name: "one iteration over the ceiling", password: "correct horse battery staple", salt: hex(salt), iterations: 2000001 },
    { name: "a salt one octet short", password: "correct horse battery staple", salt: hex(salt.slice(0, 15)), iterations: 100000 },
  ];
  return { vectors, refusals };
}

async function hkdfVectors(): Promise<Obj> {
  const cases: [string, Bytes, Bytes, string][] = [
    ["a salt and an info string", await fixed("hkdf ikm 1", 32), await fixed("hkdf salt 1", 16), "chat v1"],
    ["an empty salt and an empty info", await fixed("hkdf ikm 2", 32), EMPTY, ""],
    ["long material and a long info", await fixed("hkdf ikm 3", 80), await fixed("hkdf salt 3", 32), "Novis ⇄ WebCrypto, a longer context string"],
  ];
  const vectors: Obj[] = [];
  for (const [name, ikm, salt, info] of cases) {
    const key = await hkdf(ikm, salt, utf8.encode(info), 256);
    vectors.push({ name, material: hex(ikm), salt: hex(salt), info, key: hex(key) });
  }
  return { vectors };
}

type Pairs = Record<Curve, [KeyPair, KeyPair]>;

async function ecdhVectors(pairs: Pairs): Promise<Obj> {
  const vectors: Obj[] = [];
  for (const curve of ["P-256", "X25519"] as const) {
    const [a, b] = pairs[curve];
    const ab = await agree(curve, a.priv, b.pub);
    const ba = await agree(curve, b.priv, a.pub);
    assert(hex(ab) === hex(ba), `${curve} agreement is symmetric`);
    vectors.push({ name: `two ${curve} key pairs agree`, curve, a: a.record, b: b.record, secret: hex(ab) });
  }

  const refusals: Obj[] = [];
  const x25519 = CURVE.X25519.algorithm;
  const lowOrder: [string, Bytes][] = [["the all-zero point", new Uint8Array(32)], ["the point u = 1", cat(new Uint8Array([1]), new Uint8Array(31))]];
  for (const [name, point] of lowOrder) {
    const pub = await subtle.importKey("raw", point, x25519, true, []);
    const webcrypto = await refuses(agree("X25519", pairs.X25519[0].priv, pub)) ? "refused" : "answered";
    refusals.push({ name: `X25519 ${name}, whose shared secret is all zero`, curve: "X25519", mine: pairs.X25519[0].record.pkcs8, theirs: hex(point), webcrypto });
  }
  const good = unhex(pairs["P-256"][1].record.raw);
  const p256 = CURVE["P-256"].algorithm;
  const bad: [string, Bytes][] = [["a point off the curve (the last y octet flipped)", flipAt(good, 64)], ["the point at infinity", new Uint8Array([0])], ["an uncompressed point one octet short", good.slice(0, 64)]];
  for (const [name, point] of bad) {
    const webcrypto = await refuses(subtle.importKey("raw", point, p256, true, [])) ? "refused" : "answered";
    assert(webcrypto === "refused", `WebCrypto refuses P-256 ${name}`);
    refusals.push({ name: `P-256 ${name}`, curve: "P-256", mine: pairs["P-256"][0].record.pkcs8, theirs: hex(point), webcrypto });
  }
  return { vectors, refusals };
}

async function aesKwVectors(): Promise<Obj> {
  const vectors: Obj[] = [];
  for (const [name, kekLength] of [["a 128-bit key-encryption key", 16], ["a 256-bit key-encryption key", 32]] as const) {
    const kek = await fixed(`aes-kw kek ${kekLength}`, kekLength);
    const key = await fixed(`aes-kw key ${kekLength}`, 32);
    const wrapped = await kwWrap(kek, key);
    assert(hex(await kwUnwrap(kek, wrapped)) === hex(key), `aes-kw ${name} round trip`);
    vectors.push({ name, kek: hex(kek), key: hex(key), wrapped: hex(wrapped) });
  }
  return { vectors };
}

type JweKey =
  | { kind: "shared"; key: string }
  | { kind: "password"; password: string }
  | { kind: "keyPair"; curve: Curve; pkcs8: string; public: string };

interface JweVector {
  name: string;
  key: JweKey;
  randomness: Obj;
  payload: string;
  token: string;
}

interface JweRefusal {
  name: string;
  kind: "policy" | "authenticity";
  key: JweKey;
  token: string;
}

async function jweVectors(pairs: Pairs): Promise<Obj> {
  const shared = await fixed("jwe shared key", 32);
  const password = "correct horse battery staple";
  const recipients: Record<Curve, KeyPair> = { "P-256": pairs["P-256"][1], X25519: pairs.X25519[1] };
  const sharedKey: JweKey = { kind: "shared", key: hex(shared) };
  const passwordKey: JweKey = { kind: "password", password };
  const pairKey = (curve: Curve): JweKey => ({ kind: "keyPair", curve, pkcs8: recipients[curve].record.pkcs8, public: recipients[curve].record.raw });
  const opener = (key: JweKey): Opener => key.kind === "shared" ? { shared: unhex(key.key) }
    : key.kind === "password" ? { password: key.password }
    : { priv: recipients[key.curve].priv };

  const vectors: JweVector[] = [];

  async function dir(name: string, header: Obj, payload: string): Promise<string> {
    const iv = await fixed(`jwe iv ${name}`, 12);
    const token = await jweSeal(header, EMPTY, shared, iv, utf8.encode(payload));
    vectors.push({ name, key: sharedKey, randomness: { iv: hex(iv) }, payload, token });
    return token;
  }

  async function ecdhEs(name: string, curve: Curve, payload: string): Promise<{ token: string; ephemeral: KeyPair; cek: Bytes }> {
    const scalar = await fixed(`jwe ephemeral ${curve}`, 32);
    const ephemeral = await keyPair(curve, scalar);
    const z = await agree(curve, ephemeral.priv, recipients[curve].pub);
    const cek = await concatKdf(z, "A256GCM", EMPTY, EMPTY, 256);
    const iv = await fixed(`jwe iv ${name}`, 12);
    const token = await jweSeal({ alg: "ECDH-ES", enc: "A256GCM", epk: ephemeral.minimal }, EMPTY, cek, iv, utf8.encode(payload));
    vectors.push({ name, key: pairKey(curve), randomness: { iv: hex(iv), ephemeralScalar: hex(scalar), ephemeralPkcs8: ephemeral.record.pkcs8 }, payload, token });
    return { token, ephemeral, cek };
  }

  async function pbes2(name: string, p2c: number, p2sLength: number, payload: string): Promise<{ token: string; randomness: Obj }> {
    const p2s = await fixed(`jwe p2s ${name}`, p2sLength);
    const cek = await fixed(`jwe cek ${name}`, 32);
    const iv = await fixed(`jwe iv ${name}`, 12);
    const kek = await pbkdf2(utf8.encode(password), pbes2Salt(p2s), p2c, 128);
    const header = { alg: PBES2, enc: "A256GCM", p2c, p2s: b64u(p2s) };
    const token = await jweSeal(header, await kwWrap(kek, cek), cek, iv, utf8.encode(payload));
    return { token, randomness: { iv: hex(iv), cek: hex(cek), p2s: hex(p2s), p2c } };
  }

  const dirToken = await dir("dir, a JSON payload", { alg: "dir", enc: "A256GCM" }, '{"scope":["read","write"],"user":42}');
  await dir("dir, the empty payload", { alg: "dir", enc: "A256GCM" }, "");
  await dir("dir with a kid, a non-ASCII payload", { alg: "dir", enc: "A256GCM", kid: "ring-1" }, "grüße, 世界 🔐");
  const p256 = await ecdhEs("ECDH-ES over P-256", "P-256", '{"card":"4111 1111 1111 1111"}');
  const x25519 = await ecdhEs("ECDH-ES over X25519", "X25519", "a message for the server alone");
  const pb = await pbes2("PBES2 at the iteration floor", 100000, 16, '{"note":"sealed under a passphrase"}');
  vectors.push({ name: "PBES2 at the iteration floor", key: passwordKey, randomness: pb.randomness, payload: '{"note":"sealed under a passphrase"}', token: pb.token });

  for (const v of vectors) {
    const opened = Buffer.from(await jweOpen(v.token, opener(v.key))).toString("utf8");
    assert(opened === v.payload, `jwe opens: ${v.name}`);
  }

  // Each refusal is either well-formed except for the one thing it is refused for (`policy`), or a
  // token whose cryptography does not hold (`authenticity`). `jweOpen` applies no policy, so it opens
  // every policy refusal and fails every authenticity one -- which is checked below.
  const refusals: JweRefusal[] = [];
  const iv = await fixed("jwe iv refusal", 12);
  const payload = utf8.encode('{"user":42}');
  async function policy(name: string, header: Obj, key: JweKey = sharedKey, cek: Bytes = shared, plaintext: Bytes = payload): Promise<void> {
    refusals.push({ name, kind: "policy", key, token: await jweSeal(header, EMPTY, cek, iv, plaintext) });
  }

  const a128 = await fixed("jwe a128gcm key", 16);
  refusals.push({ name: "enc A128GCM, which is not the subset", kind: "policy", key: { kind: "shared", key: hex(a128) }, token: await jweSeal({ alg: "dir", enc: "A128GCM" }, EMPTY, a128, iv, payload) });
  await policy("alg none", { alg: "none", enc: "A256GCM" });
  await policy("zip DEF, a decompression bomb waiting to happen", { alg: "dir", enc: "A256GCM", zip: "DEF" }, sharedKey, shared, new Uint8Array(deflateRawSync(payload)));
  await policy("a crit list", { alg: "dir", crit: ["exp"], enc: "A256GCM", exp: 1 });
  await policy("jku, a fetch", { alg: "dir", enc: "A256GCM", jku: "https://keys.example/jwks.json" });
  await policy("x5u, a fetch", { alg: "dir", enc: "A256GCM", x5u: "https://keys.example/cert.pem" });
  await policy("a jwk member that is not epk", { alg: "dir", enc: "A256GCM", jwk: pairs["P-256"][0].minimal });
  await policy("an unknown header parameter", { alg: "dir", enc: "A256GCM", foo: "bar" });
  refusals.push({ name: "a dir token handed a password", kind: "policy", key: passwordKey, token: dirToken });
  refusals.push({ name: "a PBES2 token handed a shared key", kind: "policy", key: sharedKey, token: pb.token });
  refusals.push({ name: "a P-256 ECDH-ES token handed an X25519 key pair", kind: "policy", key: pairKey("X25519"), token: p256.token });
  const pbesRefusals: [string, number, number][] = [["p2c one over the ceiling", 2000001, 16], ["p2c one under the floor", 99999, 16], ["p2s one octet short", 100000, 15]];
  for (const [name, p2c, p2sLength] of pbesRefusals) {
    refusals.push({ name, kind: "policy", key: passwordKey, token: (await pbes2(name, p2c, p2sLength, '{"user":42}')).token });
  }

  const offCurve = { ...p256.ephemeral.minimal, y: flipFirstBit(str(p256.ephemeral.minimal.y)) };
  refusals.push({ name: "an epk off the P-256 curve", kind: "authenticity", key: pairKey("P-256"), token: await jweSeal({ alg: "ECDH-ES", enc: "A256GCM", epk: offCurve }, EMPTY, p256.cek, iv, payload) });
  const lowOrder = { crv: "X25519", kty: "OKP", x: b64u(new Uint8Array(32)) };
  refusals.push({ name: "an X25519 epk of low order", kind: "authenticity", key: pairKey("X25519"), token: await jweSeal({ alg: "ECDH-ES", enc: "A256GCM", epk: lowOrder }, EMPTY, x25519.cek, iv, payload) });
  const [h = "", ek = "", tiv = "", ct = "", tag = ""] = dirToken.split(".");
  refusals.push({ name: "the tag flipped", kind: "authenticity", key: sharedKey, token: [h, ek, tiv, ct, flipFirstBit(tag)].join(".") });
  refusals.push({ name: "the ciphertext one octet short", kind: "authenticity", key: sharedKey, token: [h, ek, tiv, b64u(unb64u(ct).slice(0, -1)), tag].join(".") });
  refusals.push({ name: "the protected header edited after sealing", kind: "authenticity", key: sharedKey, token: [b64u(utf8.encode(canon({ alg: "dir", enc: "A256GCM", kid: "x" }))), ek, tiv, ct, tag].join(".") });
  refusals.push({ name: "the right token under another key", kind: "authenticity", key: { kind: "shared", key: hex(await fixed("jwe other key", 32)) }, token: dirToken });
  refusals.push({ name: "four segments", kind: "authenticity", key: sharedKey, token: [h, ek, tiv, ct].join(".") });
  refusals.push({ name: "a protected header that is not base64url", kind: "authenticity", key: sharedKey, token: ["!!!", ek, tiv, ct, tag].join(".") });
  refusals.push({ name: "the JSON serialization, which is not compact", kind: "authenticity", key: sharedKey, token: JSON.stringify({ protected: h, iv: tiv, ciphertext: ct, tag }) });

  for (const r of refusals) {
    const opens = !(await refuses(jweOpen(r.token, opener(r.key))));
    // A policy refusal keyed for another kind than its token carries is a mismatch jweOpen cannot see
    // past, so only the ones keyed for their own kind are held to opening.
    const ownKind = !/handed/.test(r.name);
    if (r.kind === "policy" && ownKind) assert(opens, `policy refusal is otherwise well-formed: ${r.name}`);
    if (r.kind === "authenticity") assert(!opens, `authenticity refusal does not open: ${r.name}`);
  }
  return { vectors, refusals };
}

// ---------------------------------------------------------------------------------------------------
// JWS, RFC 7515 compact serialization, and the keys and key sets a token is verified against.
// ---------------------------------------------------------------------------------------------------

// Each prime is the first probable prime at or above a label-derived odd number with its top two bits
// set, so a rerun finds the same primes and the modulus has exactly the bits asked for.
const SMALL_PRIMES = ((): bigint[] => {
  const out: number[] = [];
  for (let n = 3; out.length < 2000; n += 2) {
    if (out.every((p) => p * p > n || n % p !== 0)) out.push(n);
  }
  return out.map(BigInt);
})();

const WITNESSES = [2n, 3n, 5n, 7n, 11n, 13n, 17n, 19n, 23n, 29n, 31n, 37n, 41n, 43n, 47n, 53n, 59n, 61n, 67n, 71n];

const big = (bytes: Uint8Array): bigint => BigInt(`0x${hex(bytes)}`);

function bytesOf(n: bigint): Bytes {
  const h = n.toString(16);
  return unhex(h.length % 2 ? `0${h}` : h);
}

function modPow(base: bigint, exp: bigint, mod: bigint): bigint {
  let result = 1n;
  base %= mod;
  while (exp > 0n) {
    if (exp & 1n) result = (result * base) % mod;
    base = (base * base) % mod;
    exp >>= 1n;
  }
  return result;
}

function modInverse(a: bigint, m: bigint): bigint {
  let [r0, r1, s0, s1] = [m, a % m, 0n, 1n];
  while (r1 !== 0n) {
    const q = r0 / r1;
    [r0, r1] = [r1, r0 - q * r1];
    [s0, s1] = [s1, s0 - q * s1];
  }
  assert(r0 === 1n, "the inverse exists");
  return ((s0 % m) + m) % m;
}

function probablyPrime(n: bigint): boolean {
  for (const p of SMALL_PRIMES) {
    if (n % p === 0n) return n === p;
  }
  let d = n - 1n;
  let s = 0;
  while ((d & 1n) === 0n) {
    d >>= 1n;
    s++;
  }
  witness: for (const a of WITNESSES) {
    let x = modPow(a, d, n);
    if (x === 1n || x === n - 1n) continue;
    for (let i = 1; i < s; i++) {
      x = (x * x) % n;
      if (x === n - 1n) continue witness;
    }
    return false;
  }
  return true;
}

async function prime(label: string, bits: number, e: bigint): Promise<bigint> {
  const seed = await fixed(label, bits / 8);
  seed[0] = (seed[0] ?? 0) | 0xc0;
  seed[seed.length - 1] = (seed[seed.length - 1] ?? 0) | 0x01;
  for (let p = big(seed); ; p += 2n) {
    if ((p - 1n) % e !== 0n && probablyPrime(p)) return p;
  }
}

// A private JWK, which is the one form WebCrypto imports an RSA key from its numbers in.
async function rsaKey(label: string, bits: number): Promise<Jwk> {
  const e = 65537n;
  let p = await prime(`${label} p`, bits / 2, e);
  let q = await prime(`${label} q`, bits / 2, e);
  if (p < q) [p, q] = [q, p];
  const n = p * q;
  assert(n.toString(2).length === bits, `${label} has a ${bits}-bit modulus`);
  const d = modInverse(e, (p - 1n) * (q - 1n));
  const u = (x: bigint): string => b64u(bytesOf(x));
  return {
    kty: "RSA", n: u(n), e: u(e), d: u(d), p: u(p), q: u(q),
    dp: u(d % (p - 1n)), dq: u(d % (q - 1n)), qi: u(modInverse(q, p)),
  };
}

type Alg = "RS256" | "PS256" | "ES256" | "EdDSA";

// The four JWS algorithms the roster verifies and signs. RS256 and EdDSA are deterministic, so a token
// under either is one `Core\Jwt::sign` must reproduce byte for byte; ES256 and PS256 draw per signature.
const SIGNING: Record<Alg, { algorithm: RsaHashedImportParams | EcKeyImportParams | Algorithm; params: Algorithm | RsaPssParams | EcdsaParams; deterministic: boolean }> = {
  RS256: { algorithm: { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" }, params: { name: "RSASSA-PKCS1-v1_5" }, deterministic: true },
  PS256: { algorithm: { name: "RSA-PSS", hash: "SHA-256" }, params: { name: "RSA-PSS", saltLength: 32 }, deterministic: false },
  ES256: { algorithm: { name: "ECDSA", namedCurve: "P-256" }, params: { name: "ECDSA", hash: "SHA-256" }, deterministic: false },
  EdDSA: { algorithm: { name: "Ed25519" }, params: { name: "Ed25519" }, deterministic: true },
};

const isAlg = (a: unknown): a is Alg => typeof a === "string" && a in SIGNING;

// PKCS#8 around RFC 8410's CurvePrivateKey, for Ed25519.
const ED25519_PKCS8_PREFIX = unhex("302e020100300506032b657004220420");

function pem(label: string, der: Uint8Array): string {
  const body = (Buffer.from(der).toString("base64").match(/.{1,64}/g) ?? []).join("\n");
  return `-----BEGIN ${label}-----\n${body}\n-----END ${label}-----\n`;
}

interface SigningKey {
  kid: string;
  alg: Alg;
  priv: CryptoKey;
  pub: CryptoKey;
  minimal: Obj;
  spki: Bytes;
  privateJwk: Jwk;
  record: Obj;
}

async function signingKey(kid: string, alg: Alg, source: { jwk: Jwk } | { pkcs8: Bytes }): Promise<SigningKey> {
  const { algorithm } = SIGNING[alg];
  const priv = "jwk" in source
    ? await subtle.importKey("jwk", source.jwk, algorithm, true, ["sign"])
    : await subtle.importKey("pkcs8", source.pkcs8, algorithm, true, ["sign"]);
  const privateJwk = await subtle.exportKey("jwk", priv);
  const { d: _d, p: _p, q: _q, dp: _dp, dq: _dq, qi: _qi, key_ops: _ops, ext: _ext, ...publicJwk } = privateJwk;
  const pub = await subtle.importKey("jwk", publicJwk, algorithm, true, ["verify"]);
  const kty = publicJwk.kty;
  const minimal: Obj = kty === "RSA" ? { e: publicJwk.e, kty, n: publicJwk.n }
    : kty === "EC" ? { crv: publicJwk.crv, kty, x: publicJwk.x, y: publicJwk.y }
    : { crv: publicJwk.crv, kty, x: publicJwk.x };
  const pkcs8 = new Uint8Array(await subtle.exportKey("pkcs8", priv));
  const spki = new Uint8Array(await subtle.exportKey("spki", pub));
  return {
    kid,
    alg,
    priv,
    pub,
    minimal,
    spki,
    privateJwk,
    record: {
      alg,
      pkcs8: hex(pkcs8),
      // The form a provider hands out -- a Google service-account file carries exactly this block.
      pem: pem("PRIVATE KEY", pkcs8),
      spki: hex(spki),
      jwk: browserJwk(await subtle.exportKey("jwk", pub)),
      jwkMinimal: canon(minimal),
      thumbprint: b64u(new Uint8Array(await subtle.digest("SHA-256", utf8.encode(canon(minimal))))),
    },
  };
}

function jwsInput(header: Obj, payload: string): string {
  return `${b64u(utf8.encode(canon(header)))}.${b64u(utf8.encode(payload))}`;
}

async function verifies(key: SigningKey, token: string): Promise<boolean> {
  const [h = "", p = "", s = ""] = token.split(".");
  try {
    return await subtle.verify(SIGNING[key.alg].params, key.pub, unb64u(s), utf8.encode(`${h}.${p}`));
  } catch {
    return false;
  }
}

// Every JWS the file on disk already holds, by its signing input, so a randomized signature survives a
// rerun. One input can carry several tokens -- a refusal reuses a vector's input under a mangled
// signature -- so `jws` takes the first of them that verifies under the key it is signing with.
const PREVIOUS = new Map<string, string[]>();
// The same, for a raw signature: by the key that made it and the message it is over.
const RAW_PREVIOUS = new Map<string, string[]>();

function rememberOnDisk(): void {
  try {
    const onDisk = JSON.parse(readFileSync(OUT, "utf8")) as { jws?: Record<string, { token: string }[]>; signatures?: Record<string, { key: string; message: string; signature: string }[]> };
    for (const section of ["vectors", "signs", "refusals"]) {
      for (const { token } of onDisk.jws?.[section] ?? []) {
        const parts = token.split(".");
        if (parts.length !== 3) continue;
        const input = `${parts[0]}.${parts[1]}`;
        PREVIOUS.set(input, [...(PREVIOUS.get(input) ?? []), token]);
      }
    }
    for (const section of ["vectors", "refusals"]) {
      for (const { key, message, signature } of onDisk.signatures?.[section] ?? []) {
        const at = `${key}:${message}`;
        RAW_PREVIOUS.set(at, [...(RAW_PREVIOUS.get(at) ?? []), signature]);
      }
    }
  } catch {
    // No file yet, or one from before this section existed: every signature is drawn fresh.
  }
}

async function jws(key: SigningKey, header: Obj, payload: string): Promise<string> {
  const input = jwsInput(header, payload);
  if (!SIGNING[key.alg].deterministic) {
    for (const old of PREVIOUS.get(input) ?? []) {
      if (await verifies(key, old)) return old;
    }
  }
  const signature = new Uint8Array(await subtle.sign(SIGNING[key.alg].params, key.priv, utf8.encode(input)));
  const token = `${input}.${b64u(signature)}`;
  // Remembered for the rest of this run, for `rawSign`'s reason.
  PREVIOUS.set(input, [token, ...(PREVIOUS.get(input) ?? [])]);
  return token;
}

// JWS wants ECDSA's raw r || s; this is the DER form an X.509 toolkit writes instead.
function der(raw: Bytes): Bytes {
  const int = (b: Bytes): Bytes => {
    let i = 0;
    while (i < b.length - 1 && b[i] === 0) i++;
    let out = b.slice(i);
    if ((out[0] ?? 0) & 0x80) out = cat(new Uint8Array([0]), out);
    return cat(new Uint8Array([0x02, out.length]), out);
  };
  const body = cat(int(raw.slice(0, 32)), int(raw.slice(32)));
  return cat(new Uint8Array([0x30, body.length]), body);
}

// `Jwt\KeySet::read`'s rules: a key not meant for signing, or under an algorithm outside the roster, is
// skipped; a key that claims to sign and cannot be honoured refuses the whole set. `null` is a refusal.
function admit(jwks: string, rsaScheme: Alg | null): Map<string | undefined, Alg> | null {
  let doc: { keys: Obj[] };
  try {
    doc = JSON.parse(jwks) as { keys: Obj[] };
  } catch {
    return null;
  }
  const out = new Map<string | undefined, Alg>();
  for (const k of doc.keys) {
    if (k.use === "enc") continue;
    if (["d", "p", "q", "dp", "dq", "qi", "k"].some((m) => m in k)) return null;
    if (k.alg !== undefined && !isAlg(k.alg)) continue;
    let alg = k.alg as Alg | undefined;
    if (k.kty === "RSA") {
      if (unb64u(str(k.n)).length * 8 < 2048) return null;
      alg ??= rsaScheme ?? undefined;
      if (alg !== "RS256" && alg !== "PS256") return null;
    } else if (k.kty === "EC" && k.crv === "P-256") {
      alg ??= "ES256";
      if (alg !== "ES256") return null;
    } else if (k.kty === "OKP" && k.crv === "Ed25519") {
      alg ??= "EdDSA";
      if (alg !== "EdDSA") return null;
    } else {
      continue;
    }
    const kid = k.kid as string | undefined;
    if (out.has(kid)) return null;
    out.set(kid, alg);
  }
  return out;
}

interface Options {
  issuer: string;
  audience: string;
  now: number;
  leeway: number;
  typ?: string;
  nonce?: string;
  maxAge?: number;
}

type Verdict = "ok" | "policy" | "authenticity" | "time" | "claims";

// `Core\Jwt::verifyIssued`'s verdict, in the order it asks: the shape, the header's policy, the key, the
// signature, then the clock and the claims -- which are only ever reached under a signature that held.
async function referee(token: string, admitted: Map<string | undefined, Alg>, keys: Map<string | undefined, SigningKey>, o: Options): Promise<Verdict> {
  const parts = token.split(".");
  if (parts.length !== 3 || !parts.every((s) => /^[A-Za-z0-9_-]*$/.test(s)) || parts[0] === "") return "authenticity";
  let header: Obj;
  try {
    header = JSON.parse(Buffer.from(parts[0] ?? "", "base64url").toString("utf8")) as Obj;
  } catch {
    return "authenticity";
  }
  if (["jku", "x5u", "x5c", "jwk", "crit", "b64", "zip", "cty"].some((m) => m in header)) return "policy";
  const kid = header.kid !== undefined ? (header.kid as string) : admitted.size === 1 ? [...admitted.keys()][0] : null;
  if (kid === null || !admitted.has(kid)) return "policy";
  if (header.alg !== admitted.get(kid)) return "policy";
  const key = keys.get(kid);
  if (key === undefined || !(await verifies(key, token))) return "authenticity";
  const c = JSON.parse(Buffer.from(parts[1] ?? "", "base64url").toString("utf8")) as Obj;
  if (typeof c.exp !== "number") return "claims";
  if (c.nbf !== undefined && typeof c.nbf !== "number") return "claims";
  if (o.now >= c.exp + o.leeway) return "time";
  if (typeof c.nbf === "number" && o.now < c.nbf - o.leeway) return "time";
  if (c.iss !== o.issuer) return "claims";
  const aud: unknown[] = Array.isArray(c.aud) ? c.aud : [c.aud];
  if (!aud.includes(o.audience)) return "claims";
  if (aud.length > 1 && c.azp !== o.audience) return "claims";
  const typ = (t: unknown): string => String(t ?? "").replace(/^application\//i, "").toLowerCase();
  if (o.typ !== undefined && typ(header.typ) !== typ(o.typ)) return "claims";
  if (o.nonce !== undefined && c.nonce !== o.nonce) return "claims";
  if (o.maxAge !== undefined && (typeof c.auth_time !== "number" || o.now > c.auth_time + o.maxAge + o.leeway)) return "claims";
  return "ok";
}

// `Crypto::sign`'s and `Crypto::verify`'s primitive: the scheme is the key's, the message is raw bytes.
async function rawVerifies(key: SigningKey, message: Bytes, signature: Bytes): Promise<boolean> {
  try {
    return await subtle.verify(SIGNING[key.alg].params, key.pub, signature, message);
  } catch {
    return false;
  }
}

async function rawSign(key: SigningKey, message: Bytes): Promise<Bytes> {
  if (!SIGNING[key.alg].deterministic) {
    for (const old of RAW_PREVIOUS.get(`${key.kid}:${hex(message)}`) ?? []) {
      if (await rawVerifies(key, message, unhex(old))) return unhex(old);
    }
  }
  const signature = new Uint8Array(await subtle.sign(SIGNING[key.alg].params, key.priv, message));
  // Remembered for the rest of this run too, so asking twice for one signature answers the same bytes
  // on the first run as on every later one.
  const at = `${key.kid}:${hex(message)}`;
  RAW_PREVIOUS.set(at, [hex(signature), ...(RAW_PREVIOUS.get(at) ?? [])]);
  return signature;
}

interface JwsCase {
  name: string;
  kind?: Verdict;
  token: string;
  keySet?: number;
  now?: number;
  options?: Partial<Options>;
  payload?: string;
}

async function jwsVectors(pairs: Pairs): Promise<Obj> {
  const keys: Record<string, SigningKey> = {
    "rsa-1": await signingKey("rsa-1", "RS256", { jwk: await rsaKey("rsa 2048 a", 2048) }),
    "rsa-2": await signingKey("rsa-2", "PS256", { jwk: await rsaKey("rsa 2048 b", 2048) }),
    "ec-1": await signingKey("ec-1", "ES256", { pkcs8: cat(CURVE["P-256"].pkcs8Prefix, await fixed("es256 scalar", 32)) }),
    "ed-1": await signingKey("ed-1", "EdDSA", { pkcs8: cat(ED25519_PKCS8_PREFIX, await fixed("ed25519 seed", 32)) }),
  };
  const key = (kid: string): SigningKey => {
    const k = keys[kid];
    if (k === undefined) throw new Error(`no signing key ${kid}`);
    return k;
  };
  const rsa1Key = key("rsa-1");
  const rsa2Key = key("rsa-2");
  const ec1Key = key("ec-1");
  const ed1Key = key("ed-1");
  // Keys that sign a refusal and are never admitted under their own name.
  const other = await signingKey("rsa-1", "RS256", { jwk: await rsaKey("rsa 2048 c", 2048) });
  const weak = await signingKey("rsa-weak", "RS256", { jwk: await rsaKey("rsa 1024", 1024) });
  // `rsa-2`'s own numbers under the other RSA scheme; its exported `alg` would make WebCrypto refuse this.
  const { alg: _alg, key_ops: _ops, ...rsa2Numbers } = rsa2Key.privateJwk;
  const rsa2AsRs256 = await signingKey("rsa-2", "RS256", { jwk: rsa2Numbers });
  const attacker = await signingKey("attacker", "ES256", { pkcs8: cat(CURVE["P-256"].pkcs8Prefix, await fixed("attacker scalar", 32)) });

  const defaults = { issuer: "https://issuer.example", audience: "client-1", now: 1767225660, leeway: 60 };
  const IAT = 1767225600;
  const EXP = IAT + 3600;

  const entry = (k: SigningKey, extra: Obj = {}): Obj => ({ ...k.minimal, kid: k.kid, use: "sig", ...extra });
  const everySigningKey = JSON.stringify({ keys: [
    entry(rsa1Key, { alg: "RS256" }),
    // No alg: the scheme is the caller's to name, and without one the set is refused.
    entry(rsa2Key),
    entry(ec1Key, { alg: "ES256" }),
    entry(ed1Key, { alg: "EdDSA" }),
    { ...pairs.X25519[0].minimal, kid: "enc-1", use: "enc" },
  ] });
  const keySets: { name: string; jwks: string; rsaScheme: Alg | null; outcome: "admits" | "refused"; kids?: (string | null)[] }[] = [
    { name: "every signing key admitted and the encryption key skipped", jwks: everySigningKey, rsaScheme: "PS256", outcome: "admits", kids: ["rsa-1", "rsa-2", "ec-1", "ed-1"] },
    { name: "the sole key, carrying no kid", jwks: JSON.stringify({ keys: [{ ...ec1Key.minimal, alg: "ES256", use: "sig" }] }), rsaScheme: null, outcome: "admits", kids: [null] },
    { name: "an RSA key with no alg and no scheme named", jwks: everySigningKey, rsaScheme: null, outcome: "refused" },
    { name: "an RSA key under 2048 bits", jwks: JSON.stringify({ keys: [entry(rsa1Key, { alg: "RS256" }), entry(weak, { alg: "RS256" })] }), rsaScheme: null, outcome: "refused" },
    { name: "a key carrying its private half", jwks: JSON.stringify({ keys: [entry(ec1Key, { alg: "ES256", d: ec1Key.privateJwk.d })] }), rsaScheme: null, outcome: "refused" },
    { name: "two keys under one kid", jwks: JSON.stringify({ keys: [entry(rsa1Key, { alg: "RS256" }), entry(ec1Key, { alg: "ES256", kid: "rsa-1" })] }), rsaScheme: null, outcome: "refused" },
    { name: "a key under an algorithm outside the roster is skipped", jwks: JSON.stringify({ keys: [entry(rsa1Key, { alg: "RS256" }), entry(rsa1Key, { alg: "RS384", kid: "rsa-384" })] }), rsaScheme: null, outcome: "admits", kids: ["rsa-1"] },
    { name: "a key set that is not JSON", jwks: '{"keys":[', rsaScheme: null, outcome: "refused" },
  ];
  const admittedBy = keySets.map((s) => admit(s.jwks, s.rsaScheme));
  keySets.forEach((s, i) => {
    const a = admittedBy[i] ?? null;
    assert(s.outcome === "refused" ? a === null : a !== null && canon([...a.keys()].map((kid) => kid ?? null)) === canon(s.kids), `key set: ${s.name}`);
  });
  const keysOf = (i: number): Map<string | undefined, SigningKey> => (i === 1 ? new Map([[undefined, ec1Key]]) : new Map(Object.entries(keys)));
  const judge = (v: JwsCase): Promise<Verdict> => {
    const i = v.keySet ?? 0;
    const admitted = admittedBy[i];
    if (!admitted) throw new Error(`key set ${i} admits nothing`);
    return referee(v.token, admitted, keysOf(i), { ...defaults, ...v.options, now: v.now ?? defaults.now } as Options);
  };

  const header = (k: SigningKey, extra: Obj = {}): Obj => ({ alg: k.alg, kid: k.kid, typ: "JWT", ...extra });
  const idToken = (extra: Obj = {}): Obj => ({ iss: defaults.issuer, sub: "248289761001", aud: defaults.audience, exp: EXP, iat: IAT, ...extra });

  const vectors: JwsCase[] = [];
  async function verified(name: string, k: SigningKey, claims: Obj, extra: { header?: Obj; options?: Partial<Options>; now?: number; keySet?: number } = {}): Promise<void> {
    const payload = JSON.stringify(claims);
    const token = await jws(k, extra.header ?? header(k), payload);
    const { header: _, ...rest } = extra;
    vectors.push({ name, ...rest, payload, token });
  }
  const nonce = "n-0S6_WzA2Mj";
  await verified("an RS256 ID token, its nonce and auth_time checked", rsa1Key, idToken({ nonce, auth_time: IAT - 30 }), { options: { nonce, maxAge: 300 } });
  await verified("a PS256 ID token under the key whose scheme the caller named", rsa2Key, idToken());
  await verified("an ES256 ID token", ec1Key, idToken());
  await verified("an EdDSA ID token", ed1Key, idToken());
  await verified("structured claims: an audience list with azp, roles, amr and an address", rsa1Key, idToken({
    aud: ["client-1", "api-2"], azp: "client-1", roles: ["admin", "billing"], amr: ["pwd", "otp"],
    address: { country: "AT", locality: "Wien" }, email_verified: true,
  }));
  await verified("an access token of type at+jwt, asked for by type", ec1Key, idToken({ client_id: "client-1", scope: "invoices:read", jti: hex(await fixed("at jti", 16)) }), {
    header: header(ec1Key, { typ: "at+jwt" }), options: { typ: "application/at+jwt" },
  });
  await verified("an x5t header member, which is a hint and is ignored", rsa1Key, idToken(), { header: header(rsa1Key, { x5t: b64u(await fixed("x5t", 20)) }) });
  await verified("expiry one second inside the leeway", rsa1Key, idToken(), { now: EXP + 59 });
  await verified("not-before at the edge of the leeway", rsa1Key, idToken({ nbf: defaults.now + 60 }));
  await verified("no kid, verified against a set of one", ec1Key, idToken(), { header: { alg: "ES256", typ: "JWT" }, keySet: 1 });
  for (const v of vectors) assert(await judge(v) === "ok", `jws vector verifies: ${v.name}`);

  // `Core\Jwt::sign`'s own layout: the caller's claims in order, then iat, then exp.
  const signs: Obj[] = [];
  async function signed(name: string, k: SigningKey, options: { kid?: string; typ?: string; embedKey?: boolean }, claims: [string, string][], lifetime: number): Promise<void> {
    const h: Obj = { alg: k.alg, typ: options.typ ?? "JWT" };
    if (options.kid) h.kid = options.kid;
    if (options.embedKey) h.jwk = k.minimal;
    const payload = `{${claims.map(([n, v]) => `${JSON.stringify(n)}:${JSON.stringify(v)}`).join(",")},"iat":${IAT},"exp":${IAT + lifetime}}`;
    const token = await jws(k, h, payload);
    assert(await verifies(k, token), `jws sign verifies: ${name}`);
    signs.push({ name, key: k.kid, options, claims, clock: IAT, lifetime, deterministic: SIGNING[k.alg].deterministic, header: canon(h), token });
  }
  const assertion = async (kid: string): Promise<[string, string][]> => [["iss", "client-1"], ["sub", "client-1"], ["aud", "https://issuer.example/token"], ["jti", hex(await fixed(`assertion jti ${kid}`, 16))]];
  await signed("a client assertion for private_key_jwt under RS256", rsa1Key, { kid: "rsa-1" }, await assertion("rsa-1"), 300);
  await signed("a client assertion under EdDSA", ed1Key, { kid: "ed-1" }, await assertion("ed-1"), 300);
  const ath = b64u(new Uint8Array(await subtle.digest("SHA-256", utf8.encode("an access token"))));
  await signed("a DPoP proof under ES256, its public key embedded", ec1Key, { typ: "dpop+jwt", embedKey: true },
    [["htm", "POST"], ["htu", "https://issuer.example/token"], ["jti", hex(await fixed("dpop jti", 16))], ["ath", ath]], 60);

  // Structured claims, which `Jwt::sign` writes only under a key pair: the value as `Core\Json::encode`
  // writes it -- fields in declared order, no whitespace, no escaped slash -- with iat and exp appended.
  async function signedStructured(name: string, k: SigningKey, options: { kid?: string; typ?: string }, claims: Obj, lifetime: number): Promise<void> {
    const h: Obj = { alg: k.alg, typ: options.typ ?? "JWT" };
    if (options.kid) h.kid = options.kid;
    const payload = `${JSON.stringify(claims).slice(0, -1)},"iat":${IAT},"exp":${IAT + lifetime}}`;
    const token = await jws(k, h, payload);
    assert(await verifies(k, token), `jws structured sign verifies: ${name}`);
    signs.push({ name, key: k.kid, options, structured: claims, clock: IAT, lifetime, deterministic: SIGNING[k.alg].deterministic, header: canon(h), token });
  }
  await signedStructured("a signed request object carrying authorization_details, under RS256", rsa1Key, { kid: "rsa-1", typ: "oauth-authz-req+jwt" }, {
    iss: "client-1",
    aud: "https://issuer.example",
    response_type: "code",
    client_id: "client-1",
    redirect_uri: "https://client.example/callback",
    scope: "openid payments",
    state: hex(await fixed("request object state", 16)),
    authorization_details: [{ type: "payment_initiation", instructedAmount: { currency: "EUR", amount: "123.50" }, creditorName: "Merchant A" }],
  }, 300);

  const refusals: JwsCase[] = [];
  async function refused(name: string, kind: Verdict, token: string, extra: { now?: number; options?: Partial<Options> } = {}): Promise<void> {
    const r: JwsCase = { name, kind, ...extra, token };
    assert(await judge(r) === kind, `jws refusal is refused as ${kind}: ${name}`);
    refusals.push(r);
  }
  const sign = (k: SigningKey, h: Obj, claims: Obj): Promise<string> => jws(k, h, JSON.stringify(claims));
  const rsa1 = header(rsa1Key);
  async function policyHeader(name: string, extra: Obj): Promise<void> {
    await refused(name, "policy", await sign(rsa1Key, { ...rsa1, ...extra }, idToken()));
  }

  await refused("alg none", "policy", `${jwsInput({ alg: "none", kid: "rsa-1", typ: "JWT" }, JSON.stringify(idToken()))}.`);
  const confusion = await subtle.importKey("raw", utf8.encode(pem("PUBLIC KEY", rsa1Key.spki)), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const confusionInput = jwsInput({ alg: "HS256", kid: "rsa-1", typ: "JWT" }, JSON.stringify(idToken()));
  const mac = new Uint8Array(await subtle.sign("HMAC", confusion, utf8.encode(confusionInput)));
  await refused("HS256 keyed with the RSA public key as PEM text, the classic confusion", "policy", `${confusionInput}.${b64u(mac)}`);
  await refused("an RS256 header on the key bound to PS256", "policy", await sign(rsa2AsRs256, header(rsa2AsRs256), idToken()));
  await refused("a kid no admitted key carries", "policy", await sign(rsa1Key, { ...rsa1, kid: "rsa-9" }, idToken()));
  await refused("no kid against a set of several keys", "policy", await sign(rsa1Key, { alg: "RS256", typ: "JWT" }, idToken()));
  await refused("a token under the 1024-bit key, which no set admits", "policy", await sign(weak, header(weak), idToken()));
  await policyHeader("jku, a fetch", { jku: "https://keys.example/jwks.json" });
  await policyHeader("x5u, a fetch", { x5u: "https://keys.example/cert.pem" });
  await policyHeader("x5c, a key the token brings", { x5c: [Buffer.from(rsa1Key.spki).toString("base64")] });
  await refused("a jwk header, the attacker's own key embedded", "policy", await sign(attacker, { alg: "ES256", jwk: attacker.minimal, typ: "JWT" }, idToken()));
  await policyHeader("an unencoded payload, b64 false under crit", { b64: false, crit: ["b64"] });
  await policyHeader("a crit list naming an extension", { crit: ["https://ext.example/x"], "https://ext.example/x": true });
  await policyHeader("zip DEF, which JWS never defined", { zip: "DEF" });
  await policyHeader("cty JWT, a nested token", { cty: "JWT" });

  const good = vectors[0]?.token ?? "";
  const [gh = "", gp = "", gs = ""] = good.split(".");
  await refused("the payload edited after signing", "authenticity", [gh, b64u(utf8.encode(JSON.stringify(idToken({ nonce, auth_time: IAT - 30, sub: "1" })))), gs].join("."), { options: { nonce } });
  await refused("another RSA key signing under the admitted kid", "authenticity", await sign(other, rsa1, idToken()));
  const [e0 = "", e1 = "", e2 = ""] = (vectors[2]?.token ?? "").split(".");
  await refused("an ES256 signature in DER form", "authenticity", [e0, e1, b64u(der(unb64u(e2)))].join("."));
  await refused("an ES256 signature one octet short", "authenticity", [e0, e1, b64u(unb64u(e2).slice(0, 63))].join("."));
  await refused("an empty signature", "authenticity", [gh, gp, ""].join("."));
  await refused("two segments", "authenticity", [gh, gp].join("."));
  await refused("a header that is not base64url", "authenticity", ["!!!", gp, gs].join("."));
  await refused("the JSON serialization, which is not compact", "authenticity", JSON.stringify({ payload: gp, protected: gh, signature: gs }));

  await refused("expired at the edge of the leeway", "time", vectors[1]?.token ?? "", { now: EXP + 60 });
  await refused("not yet valid, one second past the leeway", "time", await sign(rsa1Key, rsa1, idToken({ nbf: defaults.now + 61 })));

  const { exp: _exp, ...noExp } = idToken();
  await refused("no exp at all", "claims", await sign(rsa1Key, rsa1, noExp));
  await refused("exp written as a string", "claims", await sign(rsa1Key, rsa1, idToken({ exp: String(EXP) })));
  await refused("another issuer", "claims", await sign(rsa1Key, rsa1, idToken({ iss: "https://issuer.example.evil" })));
  await refused("another audience", "claims", await sign(rsa1Key, rsa1, idToken({ aud: "client-2" })));
  await refused("an audience list without ours", "claims", await sign(rsa1Key, rsa1, idToken({ aud: ["client-2", "api-2"] })));
  await refused("an audience list whose azp is another party", "claims", await sign(rsa1Key, rsa1, idToken({ aud: ["client-1", "api-2"], azp: "api-2" })));
  await refused("a nonce that does not match", "claims", await sign(rsa1Key, rsa1, idToken({ nonce: "n-other" })), { options: { nonce } });
  await refused("a nonce asked for and absent", "claims", await sign(rsa1Key, rsa1, idToken()), { options: { nonce } });
  await refused("a typ other than the one asked for", "claims", await sign(rsa1Key, rsa1, idToken()), { options: { typ: "at+jwt" } });
  await refused("auth_time older than maxAge and the leeway", "claims", await sign(rsa1Key, rsa1, idToken({ auth_time: IAT - 400 })), { options: { maxAge: 300 } });

  // Raw signatures over the same keys: what `Crypto::sign` answers and `Crypto::verify` checks.
  const signatureVectors: Obj[] = [];
  const messages: [string, Bytes][] = [
    ["the empty message", EMPTY],
    ["a short text", utf8.encode("a message a peer signed")],
    ["a thousand octets", await fixed("raw message 1000", 1000)],
  ];
  for (const kid of ["rsa-1", "rsa-2", "ec-1", "ed-1"]) {
    const k = key(kid);
    for (const [what, message] of messages) {
      const signature = await rawSign(k, message);
      assert(await rawVerifies(k, message, signature), `raw signature verifies: ${kid}, ${what}`);
      signatureVectors.push({ name: `${k.alg} over ${what}`, key: kid, message: hex(message), signature: hex(signature), deterministic: SIGNING[k.alg].deterministic });
    }
  }
  // Standard Webhooks' `v1a`: Ed25519 over `id.timestamp.body`, which a receiver checks with `verify`.
  const webhook = utf8.encode(`msg_2KWPBgLlAfxdpx2AI54pPJ85f4W.${IAT}.{"type":"invoice.paid","data":{"id":"in_1"}}`);
  const webhookSignature = await rawSign(ed1Key, webhook);
  assert(await rawVerifies(ed1Key, webhook, webhookSignature), "the webhook signature verifies");
  signatureVectors.push({ name: "a Standard Webhooks v1a signature", key: "ed-1", message: hex(webhook), signature: hex(webhookSignature), deterministic: true });

  const signatureRefusals: Obj[] = [];
  async function rawRefused(name: string, k: SigningKey, message: Bytes, signature: Bytes): Promise<void> {
    assert(!(await rawVerifies(k, message, signature)), `raw refusal does not verify: ${name}`);
    signatureRefusals.push({ name, key: k.kid, message: hex(message), signature: hex(signature) });
  }
  const text = utf8.encode("a message a peer signed");
  const rs = await rawSign(rsa1Key, text);
  const esRaw = await rawSign(ec1Key, text);
  await rawRefused("an RS256 signature with its last octet flipped", rsa1Key, text, flipLast(rs));
  await rawRefused("another RSA key's signature over the same message", rsa1Key, text, await rawSign(other, text));
  await rawRefused("the message edited after signing", ed1Key, utf8.encode("a message a peer signeD"), await rawSign(ed1Key, text));
  await rawRefused("an ES256 signature in DER form", ec1Key, text, der(esRaw));
  await rawRefused("an ES256 signature one octet short", ec1Key, text, esRaw.slice(0, 63));
  await rawRefused("an empty signature", ed1Key, text, EMPTY);
  await rawRefused("a PKCS#1 v1.5 signature under the key bound to PSS", rsa2Key, text, await rawSign(rsa2AsRs256, text));

  const record = Object.fromEntries(Object.entries({ ...keys, "rsa-weak": weak }).map(([kid, k]) => [kid, k.record]));
  return { defaults, keys: record, keySets, vectors, signs, refusals, raw: { vectors: signatureVectors, refusals: signatureRefusals } };
}

async function build(): Promise<string> {
  await rfc7518AppendixC();
  const pairs: Pairs = {
    "P-256": [await keyPair("P-256", await fixed("p256 a", 32)), await keyPair("P-256", await fixed("p256 b", 32))],
    X25519: [await keyPair("X25519", await fixed("x25519 a", 32)), await keyPair("X25519", await fixed("x25519 b", 32))],
  };
  const { raw, ...jwsSet } = await jwsVectors(pairs);
  const set = {
    about: [
      "Written by `bun nv webcrypto-vectors` from crypto.subtle, the W3C Web Cryptography API a browser exposes; regenerate it only by running that command.",
      "Every input is derived from a label, so the command writes these bytes every time. Octets are lowercase hex; tokens, passwords, info strings and payloads are text.",
      "aesGcm.sealed is nonce(12) || ciphertext || tag(16), the layout Core\\Crypto::seal answers under Cipher::Aes256Gcm.",
      "A jwe vector proves both directions: Core\\Jwe::decrypt must answer its payload, and Core\\Jwe::encrypt given the same randomness must answer its token byte for byte.",
      'A refusal of kind "policy" is well-formed except for the one thing it is refused for; one of kind "authenticity" fails its cryptography. Core\\Jwe refuses both with one RuntimeError.',
      "jws.keys are the signing keys by kid. An RSA key is built from label-derived primes, because WebCrypto cannot derive one from a seed. jwkMinimal is RFC 7638's required members sorted, the form PublicKey::write(KeyFormat::Jwk) answers, and thumbprint is base64url(SHA-256(jwkMinimal)).",
      "An ES256 or PS256 signature is randomized by its algorithm, so the command keeps the one on disk while it still verifies over an unchanged signing input; every other byte is derived from a label.",
      "jws.keySets: Jwt\\KeySet::read(jwks) with rsaScheme naming the scheme of an RSA key that carries no alg admits exactly kids (null is a key with no kid), or refuses the whole set.",
      "jws.vectors: Jwt::verifyIssued against keySets[keySet ?? 0] at now ?? defaults.now, with defaults.issuer, defaults.audience, defaults.leeway and options, answers payload's claims.",
      "jws.signs: Jwt::sign(claims, lifetime, keys[key], options) at clock answers token byte for byte when deterministic, and otherwise its header and payload segments byte for byte and a signature that verifies.",
      'jws.refusals are judged like jws.vectors. "policy" and "authenticity" are the one RuntimeError; "time" is the expiry error, which is only reached under a signature that held; "claims" is refused after the signature holds.',
      "A jws.signs entry carrying structured rather than claims is Jwt::sign under a key pair over that value as Core\\Json::encode writes it, with iat and exp appended.",
      "signatures.vectors: Crypto::verify accepts signature over message under jws.keys[key], and Crypto::sign reproduces it byte for byte when deterministic. ES256 is the 64-octet r || s, never DER.",
      "signatures.refusals: each refused by Crypto::verify under jws.keys[key] with the one RuntimeError.",
    ],
    aesGcm: await aesGcmVectors(),
    pbkdf2: await pbkdf2Vectors(),
    hkdf: await hkdfVectors(),
    ecdh: await ecdhVectors(pairs),
    aesKw: await aesKwVectors(),
    jwe: await jweVectors(pairs),
    jws: jwsSet,
    signatures: raw,
  };
  return `${JSON.stringify(set, null, 2)}\n`;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  rememberOnDisk();
  const text = await build();
  if (args.includes("--check")) {
    let onDisk = "";
    try {
      onDisk = readFileSync(OUT, "utf8");
    } catch {
      // A missing file is a difference like any other.
    }
    if (onDisk !== text) {
      console.error(`nv webcrypto-vectors: ${OUT} is not what this command writes; run it without --check`);
      return 1;
    }
    console.log("nv webcrypto-vectors: the file on disk is current");
    return 0;
  }
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, text);
  console.log(`nv webcrypto-vectors: wrote ${OUT}`);
  return 0;
}
