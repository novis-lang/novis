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
// diff of the vectors. WebCrypto has no JWE, so the tokens are assembled here from WebCrypto's
// primitives exactly as RFC 7516/7518 describe; `jweOpen` takes each one apart again by a separate
// path, and RFC 7518 Appendix C's published Concat KDF output is reproduced before anything is written.
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
    ],
    aesGcm: await aesGcmVectors(),
    pbkdf2: await pbkdf2Vectors(),
    hkdf: await hkdfVectors(),
    ecdh: await ecdhVectors(pairs),
    aesKw: await aesKwVectors(),
    jwe: await jweVectors(pairs),
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
