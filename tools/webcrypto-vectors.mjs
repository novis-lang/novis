// The frozen WebCrypto vector set goal `webcrypto` replays, written by the browser's own API.
//
//     node tools/webcrypto-vectors.mjs            # rewrite crates/nvs-stdlib/tests/vectors/webcrypto.json
//     node tools/webcrypto-vectors.mjs --check    # exit 1 if the file on disk is not what this writes
//
// Every value in the file comes out of `globalThis.crypto.subtle` -- Node's implementation of the W3C
// Web Cryptography API, the same interface a browser exposes -- so the loop proves browser interop by
// replaying the file and never needs Node itself. A human fires this script; no check runs it.
//
// **Every input is derived from its label** (`fixed` below: SHA-256 of `label#0`, `label#1`, ...), keys
// and nonces included, so running the script twice writes the same bytes and a diff of the file is a
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

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { deflateRawSync, inflateRawSync } from 'node:zlib';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const subtle = globalThis.crypto.subtle;
const OUT = join(dirname(fileURLToPath(import.meta.url)), '..', 'crates', 'nvs-stdlib', 'tests', 'vectors', 'webcrypto.json');

const utf8 = new TextEncoder();
const hex = (b) => Buffer.from(b).toString('hex');
const unhex = (s) => new Uint8Array(Buffer.from(s, 'hex'));
const b64u = (b) => Buffer.from(b).toString('base64url');
const unb64u = (s) => new Uint8Array(Buffer.from(s, 'base64url'));
const EMPTY = new Uint8Array(0);

function cat(...parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

function u32(n) {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, n);
  return out;
}

// Members sorted at every level, no whitespace.
function canon(v) {
  if (Array.isArray(v)) return `[${v.map(canon).join(',')}]`;
  if (v && typeof v === 'object') {
    return `{${Object.keys(v).sort().map((k) => `${JSON.stringify(k)}:${canon(v[k])}`).join(',')}}`;
  }
  return JSON.stringify(v);
}

function assert(ok, what) {
  if (!ok) throw new Error(`self-check failed: ${what}`);
}

async function fixed(label, n) {
  const out = [];
  for (let i = 0; out.length < n; i++) {
    out.push(...new Uint8Array(await subtle.digest('SHA-256', utf8.encode(`${label}#${i}`))));
  }
  return new Uint8Array(out.slice(0, n));
}

// ---------------------------------------------------------------------------------------------------
// The primitives, each one WebCrypto call.
// ---------------------------------------------------------------------------------------------------

// WebCrypto answers `ciphertext ‖ tag`; the nonce is the caller's to carry.
async function gcmEncrypt(key, nonce, plaintext, aad) {
  const k = await subtle.importKey('raw', key, 'AES-GCM', false, ['encrypt']);
  const params = { name: 'AES-GCM', iv: nonce, tagLength: 128 };
  if (aad) params.additionalData = aad;
  return new Uint8Array(await subtle.encrypt(params, k, plaintext));
}

async function gcmDecrypt(key, nonce, ciphertextAndTag, aad) {
  const k = await subtle.importKey('raw', key, 'AES-GCM', false, ['decrypt']);
  const params = { name: 'AES-GCM', iv: nonce, tagLength: 128 };
  if (aad) params.additionalData = aad;
  return new Uint8Array(await subtle.decrypt(params, k, ciphertextAndTag));
}

async function refuses(promise) {
  try {
    await promise;
    return false;
  } catch {
    return true;
  }
}

async function pbkdf2(password, salt, iterations, bits) {
  const k = await subtle.importKey('raw', password, 'PBKDF2', false, ['deriveBits']);
  return new Uint8Array(await subtle.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt, iterations }, k, bits));
}

async function hkdf(ikm, salt, info, bits) {
  const k = await subtle.importKey('raw', ikm, 'HKDF', false, ['deriveBits']);
  return new Uint8Array(await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt, info }, k, bits));
}

async function kwWrap(kek, keyBytes) {
  const k = await subtle.importKey('raw', kek, 'AES-KW', false, ['wrapKey']);
  const wrapped = await subtle.importKey('raw', keyBytes, 'AES-GCM', true, ['encrypt']);
  return new Uint8Array(await subtle.wrapKey('raw', wrapped, k, 'AES-KW'));
}

async function kwUnwrap(kek, wrapped) {
  const k = await subtle.importKey('raw', kek, 'AES-KW', false, ['unwrapKey']);
  const key = await subtle.unwrapKey('raw', wrapped, k, 'AES-KW', 'AES-GCM', true, ['encrypt']);
  return new Uint8Array(await subtle.exportKey('raw', key));
}

// ---------------------------------------------------------------------------------------------------
// Key pairs, built from a fixed scalar through PKCS#8 so nothing about them is random.
// ---------------------------------------------------------------------------------------------------

const CURVE = {
  'P-256': {
    algorithm: { name: 'ECDH', namedCurve: 'P-256' },
    // PKCS#8 around an RFC 5915 ECPrivateKey with no public key in it; WebCrypto computes that half.
    pkcs8Prefix: unhex('3041020100301306072a8648ce3d020106082a8648ce3d030107042730250201010420'),
  },
  X25519: {
    algorithm: { name: 'X25519' },
    // PKCS#8 around RFC 8410's CurvePrivateKey.
    pkcs8Prefix: unhex('302e020100300506032b656e04220420'),
  },
};

async function keyPair(curve, scalar) {
  const { algorithm, pkcs8Prefix } = CURVE[curve];
  const priv = await subtle.importKey('pkcs8', cat(pkcs8Prefix, scalar), algorithm, true, ['deriveBits']);
  const { d, key_ops, ext, ...publicJwk } = await subtle.exportKey('jwk', priv);
  const pub = await subtle.importKey('jwk', publicJwk, algorithm, true, []);
  const minimal = curve === 'P-256'
    ? { crv: publicJwk.crv, kty: publicJwk.kty, x: publicJwk.x, y: publicJwk.y }
    : { crv: publicJwk.crv, kty: publicJwk.kty, x: publicJwk.x };
  return {
    curve,
    priv,
    pub,
    minimal,
    record: {
      scalar: hex(scalar),
      pkcs8: hex(await subtle.exportKey('pkcs8', priv)),
      raw: hex(await subtle.exportKey('raw', pub)),
      spki: hex(await subtle.exportKey('spki', pub)),
      // Exactly what a browser's `exportKey('jwk', publicKey)` hands a program, `ext` and `key_ops` included.
      jwk: JSON.stringify(await subtle.exportKey('jwk', pub)),
      // RFC 7638's required members, sorted: the form `PublicKey::write(KeyFormat::Jwk)` answers.
      jwkMinimal: canon(minimal),
    },
  };
}

async function agree(curve, priv, pub) {
  return new Uint8Array(await subtle.deriveBits({ name: CURVE[curve].algorithm.name, public: pub }, priv, 256));
}

// RFC 7518 § 4.6.2: SHA-256 over counter ‖ Z ‖ OtherInfo, one round per 256 bits.
async function concatKdf(z, algorithmId, apu, apv, bits) {
  const lp = (b) => cat(u32(b.length), b);
  const otherInfo = cat(lp(utf8.encode(algorithmId)), lp(apu), lp(apv), u32(bits));
  const out = [];
  for (let counter = 1; out.length * 8 < bits; counter++) {
    out.push(...new Uint8Array(await subtle.digest('SHA-256', cat(u32(counter), z, otherInfo))));
  }
  return new Uint8Array(out.slice(0, bits / 8));
}

// ---------------------------------------------------------------------------------------------------
// JWE compact serialization, RFC 7516 § 7.1: the AAD is the ASCII of the encoded protected header.
// ---------------------------------------------------------------------------------------------------

const PBES2 = 'PBES2-HS256+A128KW';

function pbes2Salt(p2s) {
  return cat(utf8.encode(PBES2), new Uint8Array([0]), p2s);
}

async function jweSeal(header, encryptedKey, cek, iv, plaintext) {
  const protectedHeader = b64u(utf8.encode(canon(header)));
  const sealed = await gcmEncrypt(cek, iv, plaintext, utf8.encode(protectedHeader));
  return [protectedHeader, b64u(encryptedKey), b64u(iv), b64u(sealed.slice(0, -16)), b64u(sealed.slice(-16))].join('.');
}

// Takes a token apart by the header it carries and applies no policy at all -- it is how this script
// proves a refusal vector is well-formed except for the one thing `Core\Jwe` must refuse it for.
async function jweOpen(token, key) {
  const [h, ek, iv, ct, tag] = token.split('.');
  const header = JSON.parse(Buffer.from(h, 'base64url').toString('utf8'));
  const bits = header.enc === 'A128GCM' ? 128 : 256;
  let cek;
  if (header.alg === 'dir' || header.alg === 'none') {
    cek = key.shared;
  } else if (header.alg === 'ECDH-ES') {
    const curve = header.epk.crv;
    const epk = await subtle.importKey('jwk', header.epk, CURVE[curve].algorithm, true, []);
    cek = await concatKdf(await agree(curve, key.priv, epk), header.enc, EMPTY, EMPTY, bits);
  } else if (header.alg === PBES2) {
    const kek = await pbkdf2(utf8.encode(key.password), pbes2Salt(unb64u(header.p2s)), header.p2c, 128);
    cek = await kwUnwrap(kek, unb64u(ek));
  }
  let plaintext = await gcmDecrypt(cek, unb64u(iv), cat(unb64u(ct), unb64u(tag)), utf8.encode(h));
  if (header.zip === 'DEF') plaintext = new Uint8Array(inflateRawSync(plaintext));
  return plaintext;
}

function flipFirstBit(b64) {
  const b = unb64u(b64);
  b[0] ^= 0x01;
  return b64u(b);
}

// ---------------------------------------------------------------------------------------------------
// RFC 7518 Appendix C, reproduced before anything is written: the Concat KDF this script uses is the
// one the RFC publishes an answer for.
// ---------------------------------------------------------------------------------------------------

async function rfc7518AppendixC() {
  const alg = CURVE['P-256'].algorithm;
  const alice = await subtle.importKey('jwk', {
    kty: 'EC', crv: 'P-256',
    x: 'gI0GAILBdu7T53akrFmMyGcsF3n5dO7MmwNBHKW5SV0',
    y: 'SLW_xSffzlPWrHEVI30DHM_4egVwt3NQqeUD7nMFpps',
    d: '0_NxaRPUMQoAJt50Gz8YiTr8gRTwyEaCumd-MToTmIo',
  }, alg, false, ['deriveBits']);
  const bob = await subtle.importKey('jwk', {
    kty: 'EC', crv: 'P-256',
    x: 'weNJy2HscCSM6AEDTDg04biOvhFhyyWvOHQfeF_PxMQ',
    y: 'e8lnCO-AlStT-NJVX-crhB7QRYhiix03illJOVAOyck',
  }, alg, false, []);
  const key = await concatKdf(await agree('P-256', alice, bob), 'A128GCM', utf8.encode('Alice'), utf8.encode('Bob'), 128);
  assert(b64u(key) === 'VqqN6vgjbSBcIijNcacQGg', 'RFC 7518 Appendix C Concat KDF output');
}

// ---------------------------------------------------------------------------------------------------
// The set.
// ---------------------------------------------------------------------------------------------------

async function aesGcmVectors() {
  const key = await fixed('aes-gcm key', 32);
  const vectors = [];
  for (const length of [0, 1, 15, 16, 17, 64, 1000]) {
    const nonce = await fixed(`aes-gcm nonce ${length}`, 12);
    const plaintext = await fixed(`aes-gcm plaintext ${length}`, length);
    const sealed = cat(nonce, await gcmEncrypt(key, nonce, plaintext));
    assert(hex(await gcmDecrypt(key, nonce, sealed.slice(12))) === hex(plaintext), `aes-gcm ${length} round trip`);
    vectors.push({ name: `a ${length}-octet message`, key: hex(key), nonce: hex(nonce), plaintext: hex(plaintext), sealed: hex(sealed) });
  }
  const good = unhex(vectors[4].sealed);
  const tagFlipped = good.slice();
  tagFlipped[tagFlipped.length - 1] ^= 0x01;
  const bodyFlipped = good.slice();
  bodyFlipped[12] ^= 0x01;
  const nonceFlipped = good.slice();
  nonceFlipped[0] ^= 0x01;
  const refusals = [
    { name: 'the last tag octet flipped', key: hex(key), sealed: hex(tagFlipped) },
    { name: 'the first ciphertext octet flipped', key: hex(key), sealed: hex(bodyFlipped) },
    { name: 'the first nonce octet flipped', key: hex(key), sealed: hex(nonceFlipped) },
    { name: 'one octet short of a nonce and a tag', key: hex(key), sealed: hex(good.slice(0, 27)) },
    { name: 'the right bytes under another key', key: hex(await fixed('aes-gcm other key', 32)), sealed: hex(good) },
  ];
  for (const r of refusals) {
    const s = unhex(r.sealed);
    assert(s.length < 28 || await refuses(gcmDecrypt(unhex(r.key), s.slice(0, 12), s.slice(12))), `aes-gcm refusal: ${r.name}`);
  }
  return { vectors, refusals };
}

async function pbkdf2Vectors() {
  const vectors = [];
  for (const [name, password] of [['an ASCII passphrase', 'correct horse battery staple'], ['a non-ASCII password', 'pässwörd 🔑 密码']]) {
    const salt = await fixed(`pbkdf2 salt ${name}`, 16);
    const key = await pbkdf2(utf8.encode(password), salt, 100000, 256);
    vectors.push({ name, password, salt: hex(salt), iterations: 100000, key: hex(key) });
  }
  const salt = await fixed('pbkdf2 refusal salt', 16);
  const refusals = [
    { name: 'one iteration under the floor', password: 'correct horse battery staple', salt: hex(salt), iterations: 99999 },
    { name: 'one iteration over the ceiling', password: 'correct horse battery staple', salt: hex(salt), iterations: 2000001 },
    { name: 'a salt one octet short', password: 'correct horse battery staple', salt: hex(salt.slice(0, 15)), iterations: 100000 },
  ];
  return { vectors, refusals };
}

async function hkdfVectors() {
  const cases = [
    ['a salt and an info string', await fixed('hkdf ikm 1', 32), await fixed('hkdf salt 1', 16), 'chat v1'],
    ['an empty salt and an empty info', await fixed('hkdf ikm 2', 32), EMPTY, ''],
    ['long material and a long info', await fixed('hkdf ikm 3', 80), await fixed('hkdf salt 3', 32), 'Novis ⇄ WebCrypto, a longer context string'],
  ];
  const vectors = [];
  for (const [name, ikm, salt, info] of cases) {
    const key = await hkdf(ikm, salt, utf8.encode(info), 256);
    vectors.push({ name, material: hex(ikm), salt: hex(salt), info, key: hex(key) });
  }
  return { vectors };
}

async function ecdhVectors(pairs) {
  const vectors = [];
  for (const curve of ['P-256', 'X25519']) {
    const [a, b] = pairs[curve];
    const ab = await agree(curve, a.priv, b.pub);
    const ba = await agree(curve, b.priv, a.pub);
    assert(hex(ab) === hex(ba), `${curve} agreement is symmetric`);
    vectors.push({ name: `two ${curve} key pairs agree`, curve, a: a.record, b: b.record, secret: hex(ab) });
  }

  const refusals = [];
  const x25519 = CURVE.X25519.algorithm;
  for (const [name, point] of [['the all-zero point', new Uint8Array(32)], ['the point u = 1', cat(new Uint8Array([1]), new Uint8Array(31))]]) {
    const pub = await subtle.importKey('raw', point, x25519, true, []);
    const webcrypto = await refuses(agree('X25519', pairs.X25519[0].priv, pub)) ? 'refused' : 'answered';
    refusals.push({ name: `X25519 ${name}, whose shared secret is all zero`, curve: 'X25519', mine: pairs.X25519[0].record.pkcs8, theirs: hex(point), webcrypto });
  }
  const good = unhex(pairs['P-256'][1].record.raw);
  const offCurve = good.slice();
  offCurve[64] ^= 0x01;
  const p256 = CURVE['P-256'].algorithm;
  for (const [name, point] of [['a point off the curve (the last y octet flipped)', offCurve], ['the point at infinity', new Uint8Array([0])], ['an uncompressed point one octet short', good.slice(0, 64)]]) {
    const webcrypto = await refuses(subtle.importKey('raw', point, p256, true, [])) ? 'refused' : 'answered';
    assert(webcrypto === 'refused', `WebCrypto refuses P-256 ${name}`);
    refusals.push({ name: `P-256 ${name}`, curve: 'P-256', mine: pairs['P-256'][0].record.pkcs8, theirs: hex(point), webcrypto });
  }
  return { vectors, refusals };
}

async function aesKwVectors() {
  const vectors = [];
  for (const [name, kekLength] of [['a 128-bit key-encryption key', 16], ['a 256-bit key-encryption key', 32]]) {
    const kek = await fixed(`aes-kw kek ${kekLength}`, kekLength);
    const key = await fixed(`aes-kw key ${kekLength}`, 32);
    const wrapped = await kwWrap(kek, key);
    assert(hex(await kwUnwrap(kek, wrapped)) === hex(key), `aes-kw ${name} round trip`);
    vectors.push({ name, kek: hex(kek), key: hex(key), wrapped: hex(wrapped) });
  }
  return { vectors };
}

async function jweVectors(pairs) {
  const shared = await fixed('jwe shared key', 32);
  const password = 'correct horse battery staple';
  const recipients = { 'P-256': pairs['P-256'][1], X25519: pairs.X25519[1] };
  const sharedKey = { kind: 'shared', key: hex(shared) };
  const passwordKey = { kind: 'password', password };
  const pairKey = (curve) => ({ kind: 'keyPair', curve, pkcs8: recipients[curve].record.pkcs8, public: recipients[curve].record.raw });
  const opener = (key) => key.kind === 'shared' ? { shared: unhex(key.key) }
    : key.kind === 'password' ? { password: key.password }
    : { priv: recipients[key.curve].priv };

  const vectors = [];
  const tokens = {};

  async function dir(name, header, payload) {
    const iv = await fixed(`jwe iv ${name}`, 12);
    const token = await jweSeal(header, EMPTY, shared, iv, utf8.encode(payload));
    vectors.push({ name, key: sharedKey, randomness: { iv: hex(iv) }, payload, token });
    return token;
  }

  async function ecdhEs(name, curve, payload) {
    const scalar = await fixed(`jwe ephemeral ${curve}`, 32);
    const ephemeral = await keyPair(curve, scalar);
    const z = await agree(curve, ephemeral.priv, recipients[curve].pub);
    const cek = await concatKdf(z, 'A256GCM', EMPTY, EMPTY, 256);
    const iv = await fixed(`jwe iv ${name}`, 12);
    const token = await jweSeal({ alg: 'ECDH-ES', enc: 'A256GCM', epk: ephemeral.minimal }, EMPTY, cek, iv, utf8.encode(payload));
    vectors.push({ name, key: pairKey(curve), randomness: { iv: hex(iv), ephemeralScalar: hex(scalar), ephemeralPkcs8: ephemeral.record.pkcs8 }, payload, token });
    return { token, ephemeral, cek };
  }

  async function pbes2(name, p2c, p2sLength, payload) {
    const p2s = await fixed(`jwe p2s ${name}`, p2sLength);
    const cek = await fixed(`jwe cek ${name}`, 32);
    const iv = await fixed(`jwe iv ${name}`, 12);
    const kek = await pbkdf2(utf8.encode(password), pbes2Salt(p2s), p2c, 128);
    const header = { alg: PBES2, enc: 'A256GCM', p2c, p2s: b64u(p2s) };
    const token = await jweSeal(header, await kwWrap(kek, cek), cek, iv, utf8.encode(payload));
    return { token, randomness: { iv: hex(iv), cek: hex(cek), p2s: hex(p2s), p2c } };
  }

  tokens.dir = await dir('dir, a JSON payload', { alg: 'dir', enc: 'A256GCM' }, '{"scope":["read","write"],"user":42}');
  await dir('dir, the empty payload', { alg: 'dir', enc: 'A256GCM' }, '');
  await dir('dir with a kid, a non-ASCII payload', { alg: 'dir', enc: 'A256GCM', kid: 'ring-1' }, 'grüße, 世界 🔐');
  const p256 = await ecdhEs('ECDH-ES over P-256', 'P-256', '{"card":"4111 1111 1111 1111"}');
  const x25519 = await ecdhEs('ECDH-ES over X25519', 'X25519', 'a message for the server alone');
  tokens.p256 = p256.token;
  const pb = await pbes2('PBES2 at the iteration floor', 100000, 16, '{"note":"sealed under a passphrase"}');
  vectors.push({ name: 'PBES2 at the iteration floor', key: passwordKey, randomness: pb.randomness, payload: '{"note":"sealed under a passphrase"}', token: pb.token });
  tokens.pbes2 = pb.token;

  for (const v of vectors) {
    const opened = Buffer.from(await jweOpen(v.token, opener(v.key))).toString('utf8');
    assert(opened === v.payload, `jwe opens: ${v.name}`);
  }

  // Each refusal is either well-formed except for the one thing it is refused for (`policy`), or a
  // token whose cryptography does not hold (`authenticity`). `jweOpen` applies no policy, so it opens
  // every policy refusal and fails every authenticity one -- which is checked below.
  const refusals = [];
  const iv = await fixed('jwe iv refusal', 12);
  const payload = utf8.encode('{"user":42}');
  async function policy(name, header, key = sharedKey, cek = shared, plaintext = payload) {
    refusals.push({ name, kind: 'policy', key, token: await jweSeal(header, EMPTY, cek, iv, plaintext) });
  }

  const a128 = await fixed('jwe a128gcm key', 16);
  refusals.push({ name: 'enc A128GCM, which is not the subset', kind: 'policy', key: { kind: 'shared', key: hex(a128) }, token: await jweSeal({ alg: 'dir', enc: 'A128GCM' }, EMPTY, a128, iv, payload) });
  await policy('alg none', { alg: 'none', enc: 'A256GCM' });
  await policy('zip DEF, a decompression bomb waiting to happen', { alg: 'dir', enc: 'A256GCM', zip: 'DEF' }, sharedKey, shared, new Uint8Array(deflateRawSync(payload)));
  await policy('a crit list', { alg: 'dir', crit: ['exp'], enc: 'A256GCM', exp: 1 });
  await policy('jku, a fetch', { alg: 'dir', enc: 'A256GCM', jku: 'https://keys.example/jwks.json' });
  await policy('x5u, a fetch', { alg: 'dir', enc: 'A256GCM', x5u: 'https://keys.example/cert.pem' });
  await policy('a jwk member that is not epk', { alg: 'dir', enc: 'A256GCM', jwk: pairs['P-256'][0].minimal });
  await policy('an unknown header parameter', { alg: 'dir', enc: 'A256GCM', foo: 'bar' });
  refusals.push({ name: 'a dir token handed a password', kind: 'policy', key: passwordKey, token: tokens.dir });
  refusals.push({ name: 'a PBES2 token handed a shared key', kind: 'policy', key: sharedKey, token: tokens.pbes2 });
  refusals.push({ name: 'a P-256 ECDH-ES token handed an X25519 key pair', kind: 'policy', key: pairKey('X25519'), token: tokens.p256 });
  for (const [name, p2c, p2sLength] of [['p2c one over the ceiling', 2000001, 16], ['p2c one under the floor', 99999, 16], ['p2s one octet short', 100000, 15]]) {
    refusals.push({ name, kind: 'policy', key: passwordKey, token: (await pbes2(name, p2c, p2sLength, '{"user":42}')).token });
  }

  const offCurve = { ...p256.ephemeral.minimal, y: flipFirstBit(p256.ephemeral.minimal.y) };
  refusals.push({ name: 'an epk off the P-256 curve', kind: 'authenticity', key: pairKey('P-256'), token: await jweSeal({ alg: 'ECDH-ES', enc: 'A256GCM', epk: offCurve }, EMPTY, p256.cek, iv, payload) });
  const lowOrder = { crv: 'X25519', kty: 'OKP', x: b64u(new Uint8Array(32)) };
  refusals.push({ name: 'an X25519 epk of low order', kind: 'authenticity', key: pairKey('X25519'), token: await jweSeal({ alg: 'ECDH-ES', enc: 'A256GCM', epk: lowOrder }, EMPTY, x25519.cek, iv, payload) });
  const [h, ek, tiv, ct, tag] = tokens.dir.split('.');
  refusals.push({ name: 'the tag flipped', kind: 'authenticity', key: sharedKey, token: [h, ek, tiv, ct, flipFirstBit(tag)].join('.') });
  refusals.push({ name: 'the ciphertext one octet short', kind: 'authenticity', key: sharedKey, token: [h, ek, tiv, b64u(unb64u(ct).slice(0, -1)), tag].join('.') });
  refusals.push({ name: 'the protected header edited after sealing', kind: 'authenticity', key: sharedKey, token: [b64u(utf8.encode(canon({ alg: 'dir', enc: 'A256GCM', kid: 'x' }))), ek, tiv, ct, tag].join('.') });
  refusals.push({ name: 'the right token under another key', kind: 'authenticity', key: { kind: 'shared', key: hex(await fixed('jwe other key', 32)) }, token: tokens.dir });
  refusals.push({ name: 'four segments', kind: 'authenticity', key: sharedKey, token: [h, ek, tiv, ct].join('.') });
  refusals.push({ name: 'a protected header that is not base64url', kind: 'authenticity', key: sharedKey, token: ['!!!', ek, tiv, ct, tag].join('.') });
  refusals.push({ name: 'the JSON serialization, which is not compact', kind: 'authenticity', key: sharedKey, token: JSON.stringify({ protected: h, iv: tiv, ciphertext: ct, tag }) });

  for (const r of refusals) {
    const key = r.key.kind === 'shared' ? { shared: unhex(r.key.key) } : opener(r.key);
    const opens = !(await refuses(jweOpen(r.token, key)));
    // A policy refusal keyed for another kind than its token carries is a mismatch jweOpen cannot see
    // past, so only the ones keyed for their own kind are held to opening.
    const ownKind = !/handed/.test(r.name);
    if (r.kind === 'policy' && ownKind) assert(opens, `policy refusal is otherwise well-formed: ${r.name}`);
    if (r.kind === 'authenticity') assert(!opens, `authenticity refusal does not open: ${r.name}`);
  }
  return { vectors, refusals };
}

// ---------------------------------------------------------------------------------------------------
// JWS, RFC 7515 compact serialization, and the keys and key sets a token is verified against.
// ---------------------------------------------------------------------------------------------------

// Each prime is the first probable prime at or above a label-derived odd number with its top two bits
// set, so a rerun finds the same primes and the modulus has exactly the bits asked for.
const SMALL_PRIMES = (() => {
  const out = [];
  for (let n = 3; out.length < 2000; n += 2) {
    if (out.every((p) => p * p > n || n % p !== 0)) out.push(n);
  }
  return out.map(BigInt);
})();

const WITNESSES = [2n, 3n, 5n, 7n, 11n, 13n, 17n, 19n, 23n, 29n, 31n, 37n, 41n, 43n, 47n, 53n, 59n, 61n, 67n, 71n];

const big = (bytes) => BigInt(`0x${hex(bytes)}`);

function bytesOf(n) {
  const h = n.toString(16);
  return unhex(h.length % 2 ? `0${h}` : h);
}

function modPow(base, exp, mod) {
  let result = 1n;
  base %= mod;
  while (exp > 0n) {
    if (exp & 1n) result = (result * base) % mod;
    base = (base * base) % mod;
    exp >>= 1n;
  }
  return result;
}

function modInverse(a, m) {
  let [r0, r1, s0, s1] = [m, a % m, 0n, 1n];
  while (r1 !== 0n) {
    const q = r0 / r1;
    [r0, r1] = [r1, r0 - q * r1];
    [s0, s1] = [s1, s0 - q * s1];
  }
  assert(r0 === 1n, 'the inverse exists');
  return ((s0 % m) + m) % m;
}

function probablyPrime(n) {
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

async function prime(label, bits, e) {
  const seed = await fixed(label, bits / 8);
  seed[0] |= 0xc0;
  seed[seed.length - 1] |= 0x01;
  for (let p = big(seed); ; p += 2n) {
    if ((p - 1n) % e !== 0n && probablyPrime(p)) return p;
  }
}

// A private JWK, which is the one form WebCrypto imports an RSA key from its numbers in.
async function rsaKey(label, bits) {
  const e = 65537n;
  let p = await prime(`${label} p`, bits / 2, e);
  let q = await prime(`${label} q`, bits / 2, e);
  if (p < q) [p, q] = [q, p];
  const n = p * q;
  assert(n.toString(2).length === bits, `${label} has a ${bits}-bit modulus`);
  const d = modInverse(e, (p - 1n) * (q - 1n));
  const u = (x) => b64u(bytesOf(x));
  return {
    kty: 'RSA', n: u(n), e: u(e), d: u(d), p: u(p), q: u(q),
    dp: u(d % (p - 1n)), dq: u(d % (q - 1n)), qi: u(modInverse(q, p)),
  };
}

// The four JWS algorithms the roster verifies and signs. RS256 and EdDSA are deterministic, so a token
// under either is one `Core\Jwt::sign` must reproduce byte for byte; ES256 and PS256 draw per signature.
const SIGNING = {
  RS256: { algorithm: { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, params: { name: 'RSASSA-PKCS1-v1_5' }, deterministic: true },
  PS256: { algorithm: { name: 'RSA-PSS', hash: 'SHA-256' }, params: { name: 'RSA-PSS', saltLength: 32 }, deterministic: false },
  ES256: { algorithm: { name: 'ECDSA', namedCurve: 'P-256' }, params: { name: 'ECDSA', hash: 'SHA-256' }, deterministic: false },
  EdDSA: { algorithm: { name: 'Ed25519' }, params: { name: 'Ed25519' }, deterministic: true },
};

// PKCS#8 around RFC 8410's CurvePrivateKey, for Ed25519.
const ED25519_PKCS8_PREFIX = unhex('302e020100300506032b657004220420');

function pem(label, der) {
  const body = Buffer.from(der).toString('base64').match(/.{1,64}/g).join('\n');
  return `-----BEGIN ${label}-----\n${body}\n-----END ${label}-----\n`;
}

async function signingKey(kid, alg, source) {
  const { algorithm } = SIGNING[alg];
  const priv = source.jwk
    ? await subtle.importKey('jwk', source.jwk, algorithm, true, ['sign'])
    : await subtle.importKey('pkcs8', source.pkcs8, algorithm, true, ['sign']);
  const privateJwk = await subtle.exportKey('jwk', priv);
  const { d, p, q, dp, dq, qi, key_ops, ext, ...publicJwk } = privateJwk;
  const pub = await subtle.importKey('jwk', publicJwk, algorithm, true, ['verify']);
  const kty = publicJwk.kty;
  const minimal = kty === 'RSA' ? { e: publicJwk.e, kty, n: publicJwk.n }
    : kty === 'EC' ? { crv: publicJwk.crv, kty, x: publicJwk.x, y: publicJwk.y }
    : { crv: publicJwk.crv, kty, x: publicJwk.x };
  const pkcs8 = new Uint8Array(await subtle.exportKey('pkcs8', priv));
  const spki = new Uint8Array(await subtle.exportKey('spki', pub));
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
      pem: pem('PRIVATE KEY', pkcs8),
      spki: hex(spki),
      jwk: JSON.stringify(await subtle.exportKey('jwk', pub)),
      jwkMinimal: canon(minimal),
      thumbprint: b64u(new Uint8Array(await subtle.digest('SHA-256', utf8.encode(canon(minimal))))),
    },
  };
}

function jwsInput(header, payload) {
  return `${b64u(utf8.encode(canon(header)))}.${b64u(utf8.encode(payload))}`;
}

async function verifies(key, token) {
  const [h, p, s] = token.split('.');
  try {
    return await subtle.verify(SIGNING[key.alg].params, key.pub, unb64u(s), utf8.encode(`${h}.${p}`));
  } catch {
    return false;
  }
}

// Every JWS the file on disk already holds, by its signing input, so a randomized signature survives a
// rerun. One input can carry several tokens -- a refusal reuses a vector's input under a mangled
// signature -- so `jws` takes the first of them that verifies under the key it is signing with.
const PREVIOUS = new Map();
try {
  const onDisk = JSON.parse(readFileSync(OUT, 'utf8'));
  for (const section of ['vectors', 'signs', 'refusals']) {
    for (const { token } of onDisk.jws?.[section] ?? []) {
      const parts = token.split('.');
      if (parts.length !== 3) continue;
      const input = `${parts[0]}.${parts[1]}`;
      PREVIOUS.set(input, [...(PREVIOUS.get(input) ?? []), token]);
    }
  }
} catch {
  // No file yet, or one from before this section existed: every signature is drawn fresh.
}

async function jws(key, header, payload) {
  const input = jwsInput(header, payload);
  if (!SIGNING[key.alg].deterministic) {
    for (const old of PREVIOUS.get(input) ?? []) {
      if (await verifies(key, old)) return old;
    }
  }
  const signature = new Uint8Array(await subtle.sign(SIGNING[key.alg].params, key.priv, utf8.encode(input)));
  return `${input}.${b64u(signature)}`;
}

// JWS wants ECDSA's raw r || s; this is the DER form an X.509 toolkit writes instead.
function der(raw) {
  const int = (b) => {
    let i = 0;
    while (i < b.length - 1 && b[i] === 0) i++;
    let out = b.slice(i);
    if (out[0] & 0x80) out = cat(new Uint8Array([0]), out);
    return cat(new Uint8Array([0x02, out.length]), out);
  };
  const body = cat(int(raw.slice(0, 32)), int(raw.slice(32)));
  return cat(new Uint8Array([0x30, body.length]), body);
}

// `Jwt\KeySet::read`'s rules: a key not meant for signing, or under an algorithm outside the roster, is
// skipped; a key that claims to sign and cannot be honoured refuses the whole set. `null` is a refusal.
function admit(jwks, rsaScheme) {
  let doc;
  try {
    doc = JSON.parse(jwks);
  } catch {
    return null;
  }
  const out = new Map();
  for (const k of doc.keys) {
    if (k.use === 'enc') continue;
    if (['d', 'p', 'q', 'dp', 'dq', 'qi', 'k'].some((m) => m in k)) return null;
    if (k.alg !== undefined && !(k.alg in SIGNING)) continue;
    let alg = k.alg;
    if (k.kty === 'RSA') {
      if (unb64u(k.n).length * 8 < 2048) return null;
      alg ??= rsaScheme ?? undefined;
      if (alg !== 'RS256' && alg !== 'PS256') return null;
    } else if (k.kty === 'EC' && k.crv === 'P-256') {
      alg ??= 'ES256';
      if (alg !== 'ES256') return null;
    } else if (k.kty === 'OKP' && k.crv === 'Ed25519') {
      alg ??= 'EdDSA';
      if (alg !== 'EdDSA') return null;
    } else {
      continue;
    }
    if (out.has(k.kid)) return null;
    out.set(k.kid, alg);
  }
  return out;
}

// `Core\Jwt::verifyIssued`'s verdict, in the order it asks: the shape, the header's policy, the key, the
// signature, then the clock and the claims -- which are only ever reached under a signature that held.
async function referee(token, admitted, keys, o) {
  const parts = token.split('.');
  if (parts.length !== 3 || !parts.every((s) => /^[A-Za-z0-9_-]*$/.test(s)) || parts[0] === '') return 'authenticity';
  let header;
  try {
    header = JSON.parse(Buffer.from(parts[0], 'base64url').toString('utf8'));
  } catch {
    return 'authenticity';
  }
  if (['jku', 'x5u', 'x5c', 'jwk', 'crit', 'b64', 'zip', 'cty'].some((m) => m in header)) return 'policy';
  const kid = header.kid ?? (admitted.size === 1 ? [...admitted.keys()][0] : null);
  if (kid === null || !admitted.has(kid)) return 'policy';
  if (header.alg !== admitted.get(kid)) return 'policy';
  if (!(await verifies(keys[kid], token))) return 'authenticity';
  const c = JSON.parse(Buffer.from(parts[1], 'base64url').toString('utf8'));
  if (typeof c.exp !== 'number') return 'claims';
  if (c.nbf !== undefined && typeof c.nbf !== 'number') return 'claims';
  if (o.now >= c.exp + o.leeway) return 'time';
  if (c.nbf !== undefined && o.now < c.nbf - o.leeway) return 'time';
  if (c.iss !== o.issuer) return 'claims';
  const aud = Array.isArray(c.aud) ? c.aud : [c.aud];
  if (!aud.includes(o.audience)) return 'claims';
  if (aud.length > 1 && c.azp !== o.audience) return 'claims';
  const typ = (t) => String(t ?? '').replace(/^application\//i, '').toLowerCase();
  if (o.typ !== undefined && typ(header.typ) !== typ(o.typ)) return 'claims';
  if (o.nonce !== undefined && c.nonce !== o.nonce) return 'claims';
  if (o.maxAge !== undefined && (typeof c.auth_time !== 'number' || o.now > c.auth_time + o.maxAge + o.leeway)) return 'claims';
  return 'ok';
}

async function jwsVectors(pairs) {
  const keys = {
    'rsa-1': await signingKey('rsa-1', 'RS256', { jwk: await rsaKey('rsa 2048 a', 2048) }),
    'rsa-2': await signingKey('rsa-2', 'PS256', { jwk: await rsaKey('rsa 2048 b', 2048) }),
    'ec-1': await signingKey('ec-1', 'ES256', { pkcs8: cat(CURVE['P-256'].pkcs8Prefix, await fixed('es256 scalar', 32)) }),
    'ed-1': await signingKey('ed-1', 'EdDSA', { pkcs8: cat(ED25519_PKCS8_PREFIX, await fixed('ed25519 seed', 32)) }),
  };
  // Keys that sign a refusal and are never admitted under their own name.
  const other = await signingKey('rsa-1', 'RS256', { jwk: await rsaKey('rsa 2048 c', 2048) });
  const weak = await signingKey('rsa-weak', 'RS256', { jwk: await rsaKey('rsa 1024', 1024) });
  // `rsa-2`'s own numbers under the other RSA scheme; its exported `alg` would make WebCrypto refuse this.
  const { alg: _alg, key_ops: _ops, ...rsa2Numbers } = keys['rsa-2'].privateJwk;
  const rsa2AsRs256 = await signingKey('rsa-2', 'RS256', { jwk: rsa2Numbers });
  const attacker = await signingKey('attacker', 'ES256', { pkcs8: cat(CURVE['P-256'].pkcs8Prefix, await fixed('attacker scalar', 32)) });

  const defaults = { issuer: 'https://issuer.example', audience: 'client-1', now: 1767225660, leeway: 60 };
  const IAT = 1767225600;
  const EXP = IAT + 3600;

  const entry = (k, extra = {}) => ({ ...k.minimal, kid: k.kid, use: 'sig', ...extra });
  const everySigningKey = JSON.stringify({ keys: [
    entry(keys['rsa-1'], { alg: 'RS256' }),
    // No alg: the scheme is the caller's to name, and without one the set is refused.
    entry(keys['rsa-2']),
    entry(keys['ec-1'], { alg: 'ES256' }),
    entry(keys['ed-1'], { alg: 'EdDSA' }),
    { ...pairs.X25519[0].minimal, kid: 'enc-1', use: 'enc' },
  ] });
  const keySets = [
    { name: 'every signing key admitted and the encryption key skipped', jwks: everySigningKey, rsaScheme: 'PS256', outcome: 'admits', kids: ['rsa-1', 'rsa-2', 'ec-1', 'ed-1'] },
    { name: 'the sole key, carrying no kid', jwks: JSON.stringify({ keys: [{ ...keys['ec-1'].minimal, alg: 'ES256', use: 'sig' }] }), rsaScheme: null, outcome: 'admits', kids: [null] },
    { name: 'an RSA key with no alg and no scheme named', jwks: everySigningKey, rsaScheme: null, outcome: 'refused' },
    { name: 'an RSA key under 2048 bits', jwks: JSON.stringify({ keys: [entry(keys['rsa-1'], { alg: 'RS256' }), entry(weak, { alg: 'RS256' })] }), rsaScheme: null, outcome: 'refused' },
    { name: 'a key carrying its private half', jwks: JSON.stringify({ keys: [entry(keys['ec-1'], { alg: 'ES256', d: keys['ec-1'].privateJwk.d })] }), rsaScheme: null, outcome: 'refused' },
    { name: 'two keys under one kid', jwks: JSON.stringify({ keys: [entry(keys['rsa-1'], { alg: 'RS256' }), entry(keys['ec-1'], { alg: 'ES256', kid: 'rsa-1' })] }), rsaScheme: null, outcome: 'refused' },
    { name: 'a key under an algorithm outside the roster is skipped', jwks: JSON.stringify({ keys: [entry(keys['rsa-1'], { alg: 'RS256' }), entry(keys['rsa-1'], { alg: 'RS384', kid: 'rsa-384' })] }), rsaScheme: null, outcome: 'admits', kids: ['rsa-1'] },
    { name: 'a key set that is not JSON', jwks: '{"keys":[', rsaScheme: null, outcome: 'refused' },
  ];
  const admittedBy = keySets.map((s) => admit(s.jwks, s.rsaScheme));
  keySets.forEach((s, i) => {
    const a = admittedBy[i];
    assert(s.outcome === 'refused' ? a === null : a !== null && canon([...a.keys()].map((kid) => kid ?? null)) === canon(s.kids), `key set: ${s.name}`);
  });
  const keysOf = (i) => (i === 1 ? { undefined: keys['ec-1'] } : keys);
  const judge = (v) => referee(v.token, admittedBy[v.keySet ?? 0], keysOf(v.keySet ?? 0), { ...defaults, ...v.options, now: v.now ?? defaults.now });

  const header = (key, extra = {}) => ({ alg: key.alg, kid: key.kid, typ: 'JWT', ...extra });
  const idToken = (extra = {}) => ({ iss: defaults.issuer, sub: '248289761001', aud: defaults.audience, exp: EXP, iat: IAT, ...extra });

  const vectors = [];
  async function verified(name, key, claims, extra = {}) {
    const payload = JSON.stringify(claims);
    const token = await jws(key, extra.header ?? header(key), payload, `vectors:${name}`);
    const { header: _, ...rest } = extra;
    vectors.push({ name, ...rest, payload, token });
  }
  const nonce = 'n-0S6_WzA2Mj';
  await verified('an RS256 ID token, its nonce and auth_time checked', keys['rsa-1'], idToken({ nonce, auth_time: IAT - 30 }), { options: { nonce, maxAge: 300 } });
  await verified('a PS256 ID token under the key whose scheme the caller named', keys['rsa-2'], idToken());
  await verified('an ES256 ID token', keys['ec-1'], idToken());
  await verified('an EdDSA ID token', keys['ed-1'], idToken());
  await verified('structured claims: an audience list with azp, roles, amr and an address', keys['rsa-1'], idToken({
    aud: ['client-1', 'api-2'], azp: 'client-1', roles: ['admin', 'billing'], amr: ['pwd', 'otp'],
    address: { country: 'AT', locality: 'Wien' }, email_verified: true,
  }));
  await verified('an access token of type at+jwt, asked for by type', keys['ec-1'], idToken({ client_id: 'client-1', scope: 'invoices:read', jti: hex(await fixed('at jti', 16)) }), {
    header: header(keys['ec-1'], { typ: 'at+jwt' }), options: { typ: 'application/at+jwt' },
  });
  await verified('an x5t header member, which is a hint and is ignored', keys['rsa-1'], idToken(), { header: header(keys['rsa-1'], { x5t: b64u(await fixed('x5t', 20)) }) });
  await verified('expiry one second inside the leeway', keys['rsa-1'], idToken(), { now: EXP + 59 });
  await verified('not-before at the edge of the leeway', keys['rsa-1'], idToken({ nbf: defaults.now + 60 }));
  await verified('no kid, verified against a set of one', keys['ec-1'], idToken(), { header: { alg: 'ES256', typ: 'JWT' }, keySet: 1 });
  for (const v of vectors) assert(await judge(v) === 'ok', `jws vector verifies: ${v.name}`);

  // `Core\Jwt::sign`'s own layout: the caller's claims in order, then iat, then exp.
  const signs = [];
  async function signed(name, key, options, claims, lifetime) {
    const h = { alg: key.alg, typ: options.typ ?? 'JWT' };
    if (options.kid) h.kid = options.kid;
    if (options.embedKey) h.jwk = key.minimal;
    const payload = `{${claims.map(([k, v]) => `${JSON.stringify(k)}:${JSON.stringify(v)}`).join(',')},"iat":${IAT},"exp":${IAT + lifetime}}`;
    const token = await jws(key, h, payload, `signs:${name}`);
    assert(await verifies(key, token), `jws sign verifies: ${name}`);
    signs.push({ name, key: key.kid, options, claims, clock: IAT, lifetime, deterministic: SIGNING[key.alg].deterministic, header: canon(h), token });
  }
  const assertion = async (kid) => [['iss', 'client-1'], ['sub', 'client-1'], ['aud', 'https://issuer.example/token'], ['jti', hex(await fixed(`assertion jti ${kid}`, 16))]];
  await signed('a client assertion for private_key_jwt under RS256', keys['rsa-1'], { kid: 'rsa-1' }, await assertion('rsa-1'), 300);
  await signed('a client assertion under EdDSA', keys['ed-1'], { kid: 'ed-1' }, await assertion('ed-1'), 300);
  const ath = b64u(new Uint8Array(await subtle.digest('SHA-256', utf8.encode('an access token'))));
  await signed('a DPoP proof under ES256, its public key embedded', keys['ec-1'], { typ: 'dpop+jwt', embedKey: true },
    [['htm', 'POST'], ['htu', 'https://issuer.example/token'], ['jti', hex(await fixed('dpop jti', 16))], ['ath', ath]], 60);

  const refusals = [];
  async function refused(name, kind, token, extra = {}) {
    const r = { name, kind, ...extra, token };
    assert(await judge(r) === kind, `jws refusal is refused as ${kind}: ${name}`);
    refusals.push(r);
  }
  const sign = (key, h, claims, name) => jws(key, h, JSON.stringify(claims), `refusals:${name}`);
  const rsa1 = header(keys['rsa-1']);
  async function policyHeader(name, extra) {
    await refused(name, 'policy', await sign(keys['rsa-1'], { ...rsa1, ...extra }, idToken(), name));
  }

  await refused('alg none', 'policy', `${jwsInput({ alg: 'none', kid: 'rsa-1', typ: 'JWT' }, JSON.stringify(idToken()))}.`);
  const confusion = await subtle.importKey('raw', utf8.encode(pem('PUBLIC KEY', keys['rsa-1'].spki)), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  const confusionInput = jwsInput({ alg: 'HS256', kid: 'rsa-1', typ: 'JWT' }, JSON.stringify(idToken()));
  const mac = new Uint8Array(await subtle.sign('HMAC', confusion, utf8.encode(confusionInput)));
  await refused('HS256 keyed with the RSA public key as PEM text, the classic confusion', 'policy', `${confusionInput}.${b64u(mac)}`);
  await refused('an RS256 header on the key bound to PS256', 'policy', await sign(rsa2AsRs256, header(rsa2AsRs256), idToken(), 'rs256 on ps256'));
  await refused('a kid no admitted key carries', 'policy', await sign(keys['rsa-1'], { ...rsa1, kid: 'rsa-9' }, idToken(), 'unknown kid'));
  await refused('no kid against a set of several keys', 'policy', await sign(keys['rsa-1'], { alg: 'RS256', typ: 'JWT' }, idToken(), 'no kid'));
  await refused('a token under the 1024-bit key, which no set admits', 'policy', await sign(weak, header(weak), idToken(), 'weak'));
  await policyHeader('jku, a fetch', { jku: 'https://keys.example/jwks.json' });
  await policyHeader('x5u, a fetch', { x5u: 'https://keys.example/cert.pem' });
  await policyHeader('x5c, a key the token brings', { x5c: [Buffer.from(keys['rsa-1'].spki).toString('base64')] });
  await refused('a jwk header, the attacker\'s own key embedded', 'policy', await sign(attacker, { alg: 'ES256', jwk: attacker.minimal, typ: 'JWT' }, idToken(), 'embedded jwk'));
  await policyHeader('an unencoded payload, b64 false under crit', { b64: false, crit: ['b64'] });
  await policyHeader('a crit list naming an extension', { crit: ['https://ext.example/x'], 'https://ext.example/x': true });
  await policyHeader('zip DEF, which JWS never defined', { zip: 'DEF' });
  await policyHeader('cty JWT, a nested token', { cty: 'JWT' });

  const good = vectors[0].token;
  const [gh, , gs] = good.split('.');
  await refused('the payload edited after signing', 'authenticity', [gh, b64u(utf8.encode(JSON.stringify(idToken({ nonce, auth_time: IAT - 30, sub: '1' })))), gs].join('.'), { options: { nonce } });
  await refused('another RSA key signing under the admitted kid', 'authenticity', await sign(other, rsa1, idToken(), 'other key'));
  const es = vectors[2].token.split('.');
  await refused('an ES256 signature in DER form', 'authenticity', [es[0], es[1], b64u(der(unb64u(es[2])))].join('.'));
  await refused('an ES256 signature one octet short', 'authenticity', [es[0], es[1], b64u(unb64u(es[2]).slice(0, 63))].join('.'));
  await refused('an empty signature', 'authenticity', [gh, good.split('.')[1], ''].join('.'));
  await refused('two segments', 'authenticity', good.split('.').slice(0, 2).join('.'));
  await refused('a header that is not base64url', 'authenticity', ['!!!', good.split('.')[1], gs].join('.'));
  await refused('the JSON serialization, which is not compact', 'authenticity', JSON.stringify({ payload: good.split('.')[1], protected: gh, signature: gs }));

  await refused('expired at the edge of the leeway', 'time', vectors[1].token, { now: EXP + 60 });
  await refused('not yet valid, one second past the leeway', 'time', await sign(keys['rsa-1'], rsa1, idToken({ nbf: defaults.now + 61 }), 'early'));

  const { exp, ...noExp } = idToken();
  await refused('no exp at all', 'claims', await sign(keys['rsa-1'], rsa1, noExp, 'no exp'));
  await refused('exp written as a string', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ exp: String(EXP) }), 'string exp'));
  await refused('another issuer', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ iss: 'https://issuer.example.evil' }), 'issuer'));
  await refused('another audience', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ aud: 'client-2' }), 'audience'));
  await refused('an audience list without ours', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ aud: ['client-2', 'api-2'] }), 'audience list'));
  await refused('an audience list whose azp is another party', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ aud: ['client-1', 'api-2'], azp: 'api-2' }), 'azp'));
  await refused('a nonce that does not match', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ nonce: 'n-other' }), 'nonce'), { options: { nonce } });
  await refused('a nonce asked for and absent', 'claims', await sign(keys['rsa-1'], rsa1, idToken(), 'no nonce'), { options: { nonce } });
  await refused('a typ other than the one asked for', 'claims', await sign(keys['rsa-1'], rsa1, idToken(), 'typ'), { options: { typ: 'at+jwt' } });
  await refused('auth_time older than maxAge and the leeway', 'claims', await sign(keys['rsa-1'], rsa1, idToken({ auth_time: IAT - 400 }), 'max age'), { options: { maxAge: 300 } });

  const record = Object.fromEntries(Object.entries({ ...keys, 'rsa-weak': weak }).map(([kid, k]) => [kid, k.record]));
  return { defaults, keys: record, keySets, vectors, signs, refusals };
}

async function build() {
  await rfc7518AppendixC();
  const pairs = {
    'P-256': [await keyPair('P-256', await fixed('p256 a', 32)), await keyPair('P-256', await fixed('p256 b', 32))],
    X25519: [await keyPair('X25519', await fixed('x25519 a', 32)), await keyPair('X25519', await fixed('x25519 b', 32))],
  };
  const set = {
    about: [
      'Written by tools/webcrypto-vectors.mjs from Node\'s crypto.subtle, the W3C Web Cryptography API a browser exposes; regenerate it only by running that script.',
      'Every input is derived from a label, so the script writes these bytes every time. Octets are lowercase hex; tokens, passwords, info strings and payloads are text.',
      'aesGcm.sealed is nonce(12) || ciphertext || tag(16), the layout Core\\Crypto::seal answers under Cipher::Aes256Gcm.',
      'A jwe vector proves both directions: Core\\Jwe::decrypt must answer its payload, and Core\\Jwe::encrypt given the same randomness must answer its token byte for byte.',
      'A refusal of kind "policy" is well-formed except for the one thing it is refused for; one of kind "authenticity" fails its cryptography. Core\\Jwe refuses both with one RuntimeError.',
      'jws.keys are the signing keys by kid. An RSA key is built from label-derived primes, because WebCrypto cannot derive one from a seed. jwkMinimal is RFC 7638\'s required members sorted, the form PublicKey::write(KeyFormat::Jwk) answers, and thumbprint is base64url(SHA-256(jwkMinimal)).',
      'An ES256 or PS256 signature is randomized by its algorithm, so the script keeps the one on disk while it still verifies over an unchanged signing input; every other byte is derived from a label.',
      'jws.keySets: Jwt\\KeySet::read(jwks) with rsaScheme naming the scheme of an RSA key that carries no alg admits exactly kids (null is a key with no kid), or refuses the whole set.',
      'jws.vectors: Jwt::verifyIssued against keySets[keySet ?? 0] at now ?? defaults.now, with defaults.issuer, defaults.audience, defaults.leeway and options, answers payload\'s claims.',
      'jws.signs: Jwt::sign(claims, lifetime, keys[key], options) at clock answers token byte for byte when deterministic, and otherwise its header and payload segments byte for byte and a signature that verifies.',
      'jws.refusals are judged like jws.vectors. "policy" and "authenticity" are the one RuntimeError; "time" is the expiry error, which is only reached under a signature that held; "claims" is refused after the signature holds.',
    ],
    aesGcm: await aesGcmVectors(),
    pbkdf2: await pbkdf2Vectors(),
    hkdf: await hkdfVectors(),
    ecdh: await ecdhVectors(pairs),
    aesKw: await aesKwVectors(),
    jwe: await jweVectors(pairs),
    jws: await jwsVectors(pairs),
  };
  return `${JSON.stringify(set, null, 2)}\n`;
}

const text = await build();
if (process.argv.includes('--check')) {
  let onDisk = '';
  try {
    onDisk = readFileSync(OUT, 'utf8');
  } catch {
    // A missing file is a difference like any other.
  }
  if (onDisk !== text) {
    console.error(`webcrypto-vectors: ${OUT} is not what this script writes; run it without --check`);
    process.exit(1);
  }
  console.log('webcrypto-vectors: the file on disk is current');
} else {
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, text);
  console.log(`webcrypto-vectors: wrote ${OUT}`);
}
