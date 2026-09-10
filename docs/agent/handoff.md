# Handoff

## State

**Goal `signed-urls`, stage 2: the canonical payload encoding is on disk and green, and nothing is
registered yet.** `crates/nvs-stdlib/src/signature.rs` is the whole of it — `document()` writes the
signed region, `read_document()` reads one back, and its module doc is the home of the wire format,
of why a key is text, and of the domain byte that keeps one key ring from being replayed across the
three doors. Five `-p nvs-stdlib` tests cover it, two of them stage 2's named ones.

The file carries a module-level `#![allow(dead_code)]` with the reason written on it: the codec's
first caller is `Core\Signature`'s two rows, which have not landed. **The slice that registers those
rows deletes that attribute** — nothing else may.

Stage 1's floor and every other goal's work are untouched. Goal `unowned-sweep`'s list still passes.

## Next group

**Stage 2: `Core\Signature`'s two rows, over the codec that landed** — one file set:
`crates/nvs-stdlib/src/signature.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/signed_cookie.rs`, `crates/nvs-types/src/core_lib.rs`.

- [ ] **The key ring, shared not copied** — `crates/nvs-stdlib/src/signed_cookie.rs:252` (`ring_of`,
      the empty-ring `LogicError`) and `crates/nvs-stdlib/src/signed_cookie.rs:282` (`cipher_at`) are
      the walk. A signature keys `crate::hash::hmac_sha256` rather than a cipher, so what is shared is
      the refusal and the 32-octet check at `crates/nvs-stdlib/src/crypto.rs:315`, not `cipher()`
      itself. `rule:core-classes/signature`.
- [ ] **The two rows, `sign`'s payload contagious** — the class joins
      `crates/nvs-stdlib/src/registry.rs:1371` and its symbols `crates/nvs-stdlib/src/lib.rs:285`;
      the bodies, the cards and the `address` arm go in `crates/nvs-stdlib/src/signature.rs:87`, and
      this slice deletes the `#![allow(dead_code)]` at `crates/nvs-stdlib/src/signature.rs:80`.
      `verify` answers `CoreTy::Array(&CoreTy::TaintedStr)`'s shape of answer per
      `rule:security/verification-does-not-launder`. `rule:core-classes/signature`.
- [ ] **`{keys, until}` as a declared shape** — `crates/nvs-stdlib/src/db/registry.rs:41` is the
      worked `CoreTy::Shape`; `until` is a required key holding a nullable value, so `default: None`
      over `CoreTy::Nullable(&CoreTy::Instance(crate::time::INSTANT_NAME))`
      (`crates/nvs-stdlib/src/time.rs:1119`), and `crates/nvs-types/src/core_lib.rs` is where the
      checker reads it. `rule:core-api/shape-parameter`, `rule:core-api/a-lifetime-is-written`.
- [ ] **Strike `§16 Core\Signature  # 29`** from
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:1`, in the same slice that
      registers the class and neither before nor after it.

## Backlog

- Stage 3 — `$uri->sign` over the `equivalent()` `compareTo` already calls: `docs/agent/loop-goal.md`
  § *Stage 3*, and `Domain::Uri` is already the byte it signs under.
- Stage 4 — the router pair over a route's identity: `docs/agent/loop-goal.md` § *Stage 4*,
  `Domain::Route`.
- Stage 5 — one refusal except expiry, and the ordering test that makes it safe:
  `docs/agent/loop-goal.md` § *Stage 5*.
- `crates/nvs-stdlib/src/uri.rs`'s module doc still says the normalization "lives on `compareTo` and
  nowhere else"; stage 3 rewrites that sentence rather than amending it.
