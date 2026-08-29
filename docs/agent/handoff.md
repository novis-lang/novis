# Handoff

## State

**Goal 1 Stage 2's derive gaps are closed.** ADR 0071 §§ 2 and 7's three refusals are on disk with
`E0756`/`E0757`/`E0758`, and `crates/nvs-types/tests/derive.rs` holds the acceptance check's three
named tests. The codec-reachable test is a **deferred pass** — `derive::resolve_field_types`, run
from `check.rs` after every file has been walked, exactly like `links::resolve` — because "another
class that itself has a codec" is a whole-program question and a field must not answer differently
for a class declared later in the file.

**It refuses the unreachable set, not the undecoded one.** A `decimal`, an `Instant`, an enum, an
`array<T>`, an inline shape and a nested derived class are all § 2-reachable and all still erase to
`CodecTy::Opaque`, so a `decodeAs<T>` over one keeps failing at run time; those decoders are
`nvs_stdlib::json`'s gap. `crates/nvs-types/src/derive.rs`'s gap 2 is that split's only home, and
refusing an `Opaque` here would report a missing decoder as a broken contract.

**ADR 0077 § 4's two link refusals have a case on each side**, one reject file for `E0754`/`E0755`
and one core file for the `{page?}` that is dropped rather than refused.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**,
unchanged and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

## Next group

**Two file sets. Slices 1 and 2 share `crates/nvs-stdlib/src/router.rs`,
`crates/nvs-stdlib/src/uri.rs` and `tests/conformance/`; slice 3 is `tools/` alone and can go last
or by itself.**

- [ ] **A `$params` key naming no capture becomes a percent-encoded query string.** ADR 0102 § 6's
      other half. `substitute` at `crates/nvs-stdlib/src/router.rs:234` is where it lands — the
      prepared pieces name every capture, so what is left over in `$params` is the query — and
      `uri.rs`'s `encode` (`crates/nvs-stdlib/src/uri.rs:603`, with `Form` at `:575`) is the
      encoder; do not write a second one. `crates/nvs-types/src/links.rs`' gap 1 owns why the
      *refusal* half waits on `#[Query]`.
- [ ] **A `.nvst` reject case over the three new derive refusals.** `E0756` (a field typed by a
      class with no codec, and a `bytes` field), `E0757` (both halves hand-written), `E0758` (a
      deriving class with no field). All three are written at
      `crates/nvs-types/src/derive.rs:210`–`:330`; the Rust twins are
      `crates/nvs-types/tests/derive.rs`, so this is the `--EXPECTF-ERROR--` half only. ADR 0071
      §§ 2 and 7.
- [ ] **`goal-switch.py` carries the outgoing goal's unclosed items, not only its checks.** The bug
      item 15's last paragraph names: a carried check whose green depends on an item list arrives
      without its basis, and every goal in `docs/agent/goals/chain.toml` otherwise inherits item 15
      by hand. `tools/goal-switch.py:41` (`LIVE`/`MARKER`) is the insertion point and
      `tools/holes.py:194` (`items`) is the reader whose contract it has to satisfy.

## Backlog

- Item 15's seventeen `nvs-ir` refusals stay open by design — `docs/agent/loop-goal.md` item 15 is
  the inventory and the ratchet is the guard.
- A promoted constructor parameter is still not a derive field (`derive.rs`'s gap 1), and `E0758`
  now makes a promotion-only deriving class a hard error rather than a silent empty codec.
- ADR 0102 § 6 spells the fallback `[app] origin` while ADR 0104 § 1 makes `[[app]]` an array of
  tables keyed on `root`/`entry`; one of the two bodies is stale and the ADR body is the rule.
- `crates/nvs-test/src/case.rs:193` still says `nvs.toml` is not read until M6 — true of the reader,
  no longer true of the file.
- The mount prefix `Core\Router::url` prepends is still empty, and there is no `::match` — goal 6's,
  per this goal's § *Standing decisions*.
- Items 10 and 11 (every class's conformance floor to 3, every `Core` member's error paths asserted)
  are the standing fallback when a group is blocked — `python tools/gaps.py` ranks both.
