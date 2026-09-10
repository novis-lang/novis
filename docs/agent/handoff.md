# Handoff

## State

**Goal `signed-urls`, stage 2 is closed: `Core\Signature` is registered, and both of its
`cargo-named` acceptance checks pass.** `crates/nvs-stdlib/src/signature.rs` now carries the two
rows, their cards, the `address` arm and the two thin bodies over the `mint`/`open` pair that was
already there; the module-level `#![allow(dead_code)]` is gone, as its own condition said it would
be. `crates/nvs-stdlib/src/time.rs`'s `instant_of` is `pub(crate)` for the `until` field;
`instant_built` was left private because nothing outside that module answers an `Instant` yet.

**A payload value is text at both halves**, and that is the one place the tree parts company with
what § 16 used to write. `rule:security/verification-does-not-launder` needs the result qualified,
`tainted` is defined over `string` and `bytes` alone, so `sign` takes `array<string>` and `verify`
answers `array<tainted string>` — `crates/nvs-stdlib/src/jwt.rs`'s *a claim is text* section reached
the same place first and the two roster entries agree rather than each inventing a rule. The spec
row is amended to match; `crates/nvs-stdlib/src/signature.rs`'s *a payload value is text* section is
the home of the reasoning and of what it spends. The codec itself still signs all nine kinds,
because `Domain::Uri` and `Domain::Route` sign typed parameters that never come back to a program.

The driver's failing acceptance check — `examples/signed-url.nvs is missing` — is stage 4 work and
still open, not a regression.

## Next group

**Stage 3: `$uri->sign` and `$uri->verifySignature` over `Domain::Uri`, signing the form
`compareTo` already normalizes** — one file set: `crates/nvs-stdlib/src/uri.rs`,
`crates/nvs-stdlib/src/signature.rs`. It is one slice for stage 2's reason: the coverage floor is
three `.nvst` cases per member, so a row without its cases fails `cargo test -p nvs-stdlib`.

- [ ] **The two rows on `Core\Uri`** — the instance list is `crates/nvs-stdlib/src/uri.rs:466` and
      `compareTo` at `crates/nvs-stdlib/src/uri.rs:616` is the row to sit beside. `sign` takes the
      shared shape `crates/nvs-stdlib/src/signature.rs:179` (`SIGNING`) and nothing else, and
      answers a `Uri`; `verifySignature` takes `array<secret bytes> $keys` and answers `void`,
      throwing on refusal. `rule:core-api/signing-is-over-a-payload`,
      `rule:core-api/each-door-takes-a-different-thing`.
- [ ] **The payload is `equivalent`'s output and not a second normalization** —
      `crates/nvs-stdlib/src/uri.rs:1712` (`equivalent`) is what
      `crates/nvs-stdlib/src/uri.rs:2537` (`nvs_core_uri_compare_to`) calls, and the signing half
      calls the same function rather than growing its own. That is the goal's standing decision and
      not a session's call. `mint`/`open` already take `Domain::Uri`.
- [ ] **The eleven named tests**, in `crates/nvs-stdlib/src/uri.rs:2776`'s test module —
      `docs/agent/loop-goal.toml:6982` names six (appending, removing and reordering a query
      parameter, hex-case rewriting, a fragment never signed, and signing calling the one
      equivalent) and `docs/agent/loop-goal.toml:6996` five more, which are the specification for
      the `_sig` parameter: reserved, single, self-excluding, and refused with one message.
- [ ] **Three `.nvst` cases per new member**, under `tests/conformance/core/uri-sign*`. The shape to
      copy is `tests/conformance/core/signature-verify-refuses-every-forgery-with-one-message.nvst:1`,
      and an `--EXPECTF-ERROR--` case pins its diagnostic by being run once first.

## Backlog

- `Core\Router`'s signed pair over `Domain::Route` — stage 4, `rule:core-classes/router-signed-url`.
- `examples/signed-url.nvs`, which the driver's acceptance check names — stage 4,
  `docs/agent/loop-goal.md`.
- ADR 0146's body still writes the payload as `array<string, mixed>`; § 16 is amended and the record
  is not, per the goal's standing decision that a hole is folded into 0146 rather than opened as
  0147.
- `instant_built` stays private until a door answers an `Instant` — `crates/nvs-stdlib/src/time.rs`.
