# Handoff

## State

**Stage 4's protocol half has three of its four entries.** `Core\SignedCookie`, `Core\Csrf` and
`Core\Totp` are rows, cards and bodies in `crates/nvs-stdlib/src/signed_cookie.rs`,
`crates/nvs-stdlib/src/csrf.rs` and `crates/nvs-stdlib/src/totp.rs`, each with its named stage-4
check green and three `.nvst` cases — the floor
`every_core_class_has_a_conformance_floor_of_three` enforces, which is per *member* and applies to a
class the day it lands. `Core\Jwt` is the one entry left, and it owes three cases too.

**`Core\Csrf` is `Core\Crypto`'s construction with a domain tag in the plaintext.** `issue` seals
`nvs.csrf.v1\0` followed by the session identifier; `verify` rebuilds exactly that and compares it
with `subtle`. That module's doc is the one home for why the session arrives as an argument, why
there is no key ring here when the cookie has one, why the tag stops a signed cookie under a shared
key from being a valid token, and why a class whose whole point is a *missing* accessor is not ADR
0063 R17's "reachable two ways" against the cookie.

**`Core\Totp` is the one class in the crate not on the near side of the AEAD**, and it is HMAC-SHA-1
through `crates/nvs-stdlib/src/hash.rs:742`'s new `hmac_sha1` — the single named exception to the
strong-digest set, with one caller and no `Digest` argument, so ADR 0060 § 4's "the algorithm never
comes from the token" holds for free. The window is fixed at ±1 step with no widening spelling, and
"no replay" is a counter: `check` answers `?int`, the step the code belonged to, and refuses
anything at or below `$after`. `totp.rs`'s module doc owns all of that, including what the class
deliberately does not do.

**Base32 is the gap `Core\Totp` leaves.** A program can generate and verify codes but cannot build
the `otpauth://` URI a phone scans, because nothing in `Core\Encoding` spells base32. That is
`Core\Encoding`'s slice, not this class's, and it is in the backlog below.

**The driver's acceptance failure is still stage 5's, not a regression.** `examples/http.nvs` names
`Core\Http::allowUrl`, which no stage before 5 lands, and a non-`0` stage's cargo checks run after
the program legs — so that fixture masks stage 4's named checks until `Core\Http` exists. Same shape
`examples/crypto.nvs` had through sessions 0004-0009.

**Manifest gaps, unchanged plus two.** Two dead `[context] modules` selectors
(`crates/nvs-host/src/pool.rs`, `crates/nvs-host/src/stream.rs`); it wants
`crates/nvs-runtime/src/commands.rs`, `crates/nvs-types/src/defaults.rs`, `crates/nvs-test/src/case.rs`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/password.rs`,
`crates/nvs-stdlib/src/crypto.rs`, `crates/nvs-stdlib/src/signed_cookie.rs`,
`crates/nvs-stdlib/src/csrf.rs` and `crates/nvs-stdlib/src/totp.rs` added. **`[context] adrs` still
needs ADR 0060 §§ 1, 4, 5** — none is printed and all three specify what is left. Nothing is blocked.

## Next group

**Stage 4's last entry and the gap it leaves.** They share the file set the last two sessions used:
`crates/nvs-stdlib/src/registry.rs:1168` (the `CLASSES` tail),
`crates/nvs-stdlib/src/lib.rs:250` and `:356` (the `mod` line and the `address` chain), and one new
module beside `crates/nvs-stdlib/src/csrf.rs:100`.

- [ ] **`Core\Jwt`, whose algorithm comes from the key** — ADR 0060 § 4's first three bullets and
      § 5: `alg` is checked against the key and never consulted to choose, `exp` is mandatory with
      no flag, and verified claims come back **`tainted`** — the opposite of
      `crates/nvs-stdlib/src/signed_cookie.rs:136`'s `Qual::Launder`, and the contrast is worth a
      sentence in each module doc. HMAC-SHA-256 through `crates/nvs-stdlib/src/hash.rs:702`'s
      `hmac_of`, registered at `crates/nvs-stdlib/src/registry.rs:1168` and
      `crates/nvs-stdlib/src/lib.rs:356`. The named check is
      `a_jwt_with_an_unexpected_algorithm_is_refused`.
- [ ] **Base32 in `Core\Encoding`, which `Core\Totp` needs and nothing spells** — RFC 4648 § 6, the
      alphabet an `otpauth://` URI carries a shared secret in. A row beside the existing
      `bytes`↔`string` members at `crates/nvs-stdlib/src/encoding.rs`, and one `.nvst` case; the
      reason it is a `Core\Encoding` member and not a `Core\Totp` one is in
      `crates/nvs-stdlib/src/totp.rs:75`'s *what this class does not do*.

## Backlog

- `examples/http.nvs` is stage 5's and masks stage 4's named checks until `Core\Http` lands —
  `docs/agent/loop-goal.toml`'s stage 5 block.
- Two dead `[context] modules` selectors in `docs/agent/loop-goal.toml`, and eight missing ones.
- `[context] adrs` needs ADR 0060 §§ 1, 4, 5 — `docs/agent/loop-goal.toml`.
- `Core\RateLimit` is what a failed TOTP code should cost, and no class calls it —
  `docs/adr/0075-core-ratelimit.md`.
