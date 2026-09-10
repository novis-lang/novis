# Handoff

## State

**Goal `signed-urls`, stage 2: the codec, the token envelope and the shared key ring are on disk and
green, and nothing is registered yet.** `crates/nvs-stdlib/src/keyring.rs` is new and is the home of
what a key ring is — newest first, rotation is prepending, an empty one and an entry that is not 32
octets are the two `LogicError`s, and `KEY` is the `CoreTy` every row that takes a ring declares.
`Core\SignedCookie` reads it now instead of its own `ring_of`/`cipher_at` pair.
`crates/nvs-stdlib/src/signature.rs` gained `mint` and `open`: `tag ‖ document` in unpadded URL-safe
base64, the domain checked inside `open` so a door cannot forget it, and expiry deliberately left to
the caller as the one distinguishable refusal.

**Five of stage 2's eight named tests are green** — the three ring ones, the round trip and the
base64 one — asserted over `mint`/`open` rather than through the registry.
`a_verified_payload_reaching_a_query_text_position_is_refused_as_tainted` needs the registered row.

The module-level `#![allow(dead_code)]` at `crates/nvs-stdlib/src/signature.rs:83` still stands and
still names its own condition: the first caller is `Core\Signature`'s two rows. **The slice that
registers them deletes it** — nothing else may.

## Next group

**Stage 2: `Core\Signature`'s two rows, over the codec and the envelope that landed** — one file
set: `crates/nvs-stdlib/src/signature.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/time.rs`, `crates/nvs-types/src/core_lib.rs`. It is one slice, not three: the
registry's coverage floor is three `.nvst` cases per member, so a row without its cases fails
`cargo test -p nvs-stdlib` on the spot.

- [ ] **`{keys, until}` as a declared shape** — `crates/nvs-stdlib/src/registry.rs:720` is
      `CoreTy::Shape(&[&[CoreField]])` and `crates/nvs-stdlib/src/db/registry.rs:41` is the worked
      one. `until` is a required key holding a nullable value, so `default: None` over
      `CoreTy::Nullable(&CoreTy::Instance(crate::time::INSTANT_NAME))`
      (`crates/nvs-stdlib/src/time.rs:1119`), and `crates/nvs-types/src/core_lib.rs` is where the
      checker reads it. `rule:core-api/shape-parameter`, `rule:core-api/a-lifetime-is-written`.
- [ ] **The two rows, `sign`'s payload contagious** — the class joins
      `crates/nvs-stdlib/src/registry.rs:1371` and its symbols `crates/nvs-stdlib/src/lib.rs:285`;
      the rows, the cards, the bodies and the `address` arm go in
      `crates/nvs-stdlib/src/signature.rs:83`, which is also the `#![allow(dead_code)]` this slice
      deletes. The bodies are thin: `sign` is `mint(Domain::Payload, until, payload, ring, …)` and
      `verify` is `open(...)` plus the expiry judgement plus one refusal — the ring is
      `crate::keyring::borrow(args, 1, WHO)` at both, and `KEY` is the element type.
      `verify` answers `CoreTy::Array(&CoreTy::TaintedStr)` per
      `rule:security/verification-does-not-launder`. `rule:core-classes/signature`.
- [ ] **`until` in and out** — `crates/nvs-stdlib/src/time.rs:3179` (`instant_of`) and
      `crates/nvs-stdlib/src/time.rs:3163` (`instant_built`) are the pair, and both are private:
      make them `pub(crate)` rather than reading the slots again. `Timestamp::as_second()` and
      `subsec_nanosecond()` are what `signature::Until`'s two fields take
      (`crates/nvs-stdlib/src/signature.rs:207`).
- [ ] **Strike `§16 Core\Signature  # 29`** from
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:1`, in the same slice that
      registers the class and neither before nor after it.

## Backlog

- `Core\Uri::sign`/`verifySignature` over `Domain::Uri` — stage 3, and `mint`/`open` already take it.
- `Core\Router`'s signed pair over `Domain::Route` — stage 4.
- `examples/signed-url.nvs`, which the driver's acceptance check names — stage 4, `docs/agent/loop-goal.md`.
- Expiry as the one distinguishable refusal — stage 5, `rule:core-api/one-refusal-except-expiry`.
