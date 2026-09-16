# Handoff

## State

**Goal `m8-stdlib-depth` is met.** `python tools/loop.py --goal-only` ends `GOAL REACHED: every
acceptance check passes` (787 checks), and `python tools/verify.py --doc` resolves every link — the gate
a goal meets only at its end. Stage 14's last open item is struck and stage 15's three rules report
`shipped`.

Stage 14 item 3 — ADR 0057's cache bullet — is **struck rather than tested**, because a prepared artifact
has no address of its own: the cache holds one file per compiled unit, a prepared entry rides in that
unit's payload, and nothing is prepared at all until `crates/nvs-types/src/intrinsics.rs:57` gap 2's
checker-to-IR channel is built. So `crates/nvs-cli/src/cache.rs:2511`, where a foreign compiler build
misses on the address and again on the header, is the bullet's test, and it is now
`rule:expressions/preparation-preserves-behaviour`'s guard.

Stage 15: `core-classes/process-spawn`, `observability/metrics-three-members` and
`observability/the-exporter-is-a-feature-and-core-metrics-is-not` are `shipped`, each naming the guards
stages 12 and 13 landed. `errors/record-producers` stays `designed` on purpose — two of its five producers
are M10's and `nvs-render`'s.

Two carried floor entries named artifacts this goal's own stage-5 JSON-wire slice renamed: the test
`json_decode_into_a_decimal_field_round_trips_25_significant_digits`, now
`decode_as_fills_a_decimal_field_from_the_numbers_own_digits` (`crates/nvs-stdlib/src/json.rs:2877`), and
the case `json-decode-as-a-decimal-field-keeps-25-significant-digits.nvst`, now
`tests/conformance/core/json-decode-as-round-trips-a-25-digit-decimal-exactly.nvst`. Both toml copies name
the live ones, and each claim is the same 25 digits. No Rust behaviour changed this session. Nothing is
blocked.

## Next group

**Goal `unowned-closures` stage 2: the lowering and the runtime, security first** — one file set:
`crates/nvs-ir/src/lower/` and `crates/nvs-runtime/src/`. A goal switch reseeds this file from
`docs/agent/goals/60-unowned-closures.handoff.md`, so the trio is repeated here only so that a refused
DONE does not lose it. Each item's answer is the `Decided:` sentence already on disk under its gap; build
to it and never re-open it.

- [ ] **A `secret` compared against a `mixed` is constant-time** —
      `crates/nvs-ir/src/lower/operator.rs:896`, with the type side at `crates/nvs-ir/src/lib.rs:405`.
- [ ] **One allocation past the budget, to its decision** — `crates/nvs-runtime/src/budget.rs:89`.
- [ ] **A hooked property is reached through an erased key** — `crates/nvs-runtime/src/object.rs:3403`.

## Backlog

- The rest of stage 2's decided items — `crates/nvs-runtime/src/lib.rs:199`.
- Stage 3, the checker and front end — `crates/nvs-hir/src/requires.rs:84` first.
- Stage 4, the library — `crates/nvs-stdlib/src/compress.rs:47` first.
- Stage 5, server/config/cache/schema — `crates/nvs-config/src/cache.rs:51` first.
- The preparation channel itself, which is what would make ADR 0057 § 4 testable end to end —
  `crates/nvs-types/src/intrinsics.rs:57` gap 2, owner `unowned-closures`.
- What must survive the goal switch is `docs/agent/carried-gaps.md`, not this file.
