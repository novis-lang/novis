# Handoff

## State

**Goal `webcrypto` — stage 5 is open at the module, and the key parameter's spelling is settled.**
`Jwe\Key` with one static per kind (`shared`, `password`, `recipient`, `own`) and two `Jwe` members;
`rule:security/jwe-compact-subset` is the home and now carries the reason that holds today. Nothing of
`Core\Jwe` is on disk yet: no module, no rows, no cases.

**Why the fallback, checked against this tree rather than read.** A union parameter carries no
qualifier classification, so it refuses every `tainted` argument — `Core\Arr::hasKey`'s
`int|string` refuses a `tainted string`, and a union of two `secret` atoms refuses
`Core\Cli::secret`'s `secret tainted string` — while `Core\Crypto::deriveKey`'s whole-parameter
`secret string` (`crates/nvs-stdlib/src/crypto.rs:703`) takes that same value. A password is tainted,
so the union's password arm would refuse every real password. ADR 0179 reached the same conclusion
from a premise that has since stopped being true (it says no `secret string` atom exists;
`CoreTy::SecretText` landed with `deriveKey` in stage 4). The record stays frozen; the fragment is the
rule.

**The acceptance check the driver reports is stage 7's, not a regression.** `examples/webcrypto.nvs`
opens a JWE token and verifies an ID token, so it cannot be written before `Core\Jwe` and
`Jwt::verifyIssued` exist (`docs/agent/loop-goal.md:222`).

## Next group

**Stage 5: `Core\Jwe`, compact, `A256GCM` alone** — one file set: a new `crates/nvs-stdlib/src/jwe.rs`
over `crypto.rs`'s crate-private primitives, plus its three registration sites. The subset is the
goal's § *Standing decisions*, *JWE, the subset* and *JWE answers like the rest of the roster*, and
`rule:security/jwe-compact-subset` is the rule; `rule:security/algorithm-comes-from-the-key` is what
makes the static's name pick `alg`.

- [ ] **`crates/nvs-stdlib/src/jwe.rs`, new, with its module doc, `Jwe\Key` and the three
      key-management paths.** `dir` is the shared key itself; `PBES2-HS256+A128KW` is
      `crates/nvs-stdlib/src/crypto.rs:1624` (`pbkdf2_sha256`, which is what takes a `p2c` off the
      wire — `:1642`'s `derive_key` is the member-facing one with the `LogicError`) plus `:2412`
      (`wrap_key`); `ECDH-ES` is `:2891` (`agree`) plus `:2473` (`concat_kdf`), `apu`/`apv` empty.
      Content encryption is `:1559`/`:1602` (`gcm_seal_under`/`gcm_open_under`). Register at
      `crates/nvs-stdlib/src/lib.rs:243` (`mod`), `crates/nvs-stdlib/src/lib.rs:404` (the `address`
      chain) and `crates/nvs-stdlib/src/registry.rs:1726` (the `CLASSES` list), which is what
      `crate::signed_cookie` does. `Jwe\Key::password`'s parameter is `CoreTy::SecretText(Qual::Neutral)`,
      the spelling `crates/nvs-stdlib/src/crypto.rs:703` writes, and its four statics are four `Core`
      members with the five edits each (`docs/agent/conventions.md` § *A `Core` member*).
- [ ] **`Jwe::encrypt`, the five edits** (`docs/agent/conventions.md` § *A `Core` member*) — the header
      is written canonically, members sorted and no whitespace at every level, which is what the frozen
      vector set holds it to byte for byte. `crates/nvs-stdlib/src/jwt.rs:364` is the member that
      already writes a canonical header and payload, and `crates/nvs-stdlib/src/signed_cookie.rs:121`
      is the smallest row-plus-card-plus-body to copy the five edits from.
- [ ] **`Jwe::decrypt`, the five edits** — the allowed header parameters are `alg`, `enc`, `epk`,
      `p2s`, `p2c`, `kid`, `typ` and `cty` and everything else is refused, the protected header is
      length-capped before it is parsed, and `p2c` is held to `crates/nvs-stdlib/src/crypto.rs:413`'s
      bounds with a `RuntimeError` rather than `derive_key`'s `LogicError`. The ring is tried in order
      and a ring holding a password key holds exactly one.

## Backlog

- `examples/webcrypto.nvs` — the acceptance fixture, still unwritten; it is stage 7's.
- The five `expect(dead_code)` markers in `crypto.rs` now name `Core\Jwe` as their caller; stage 5
  deletes them as the paths reach the primitives.
- One call site can emit the same `E0401` twice — `Core\Arr::hasKey($a, $taintedKey)` prints
  "expected `string|int`, found `tainted string`" as two errors. No owning doc; `nvs-types`' argument
  check is where it comes from.
- Standard Webhooks' `v1a` and RFC 9421 signatures are package code over `sign`/`verify`, not `Core`
  (goal § *Standing decisions*, *Raw signatures*).
- `Jwt::sign` over a key pair and `Jwt\KeySet` are stage 6's, over these two members' primitives.
