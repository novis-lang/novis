# Handoff

## State

**Spec § 11's first table is six of its seven members**: `crates/mwl-stdlib/src/random.rs` registers
`Random::int`, `float`, `token`, `pick`, `sample` and `shuffle` over the new `rand` dependency. That
module's own docs own the pick against ADR 0051 § 4, what a `ThreadRng` spends *per thread*, and the
three gaps it leaves — so nothing about it is restated here or in the plan.

`python tools/verify.py` green; `mwl test tests/conformance/` is 365 cases, all passing. The two new
ones are `tests/conformance/core/random-draws-inside-its-stated-bounds.mwlt` and
`random-draws-over-an-array.mwlt`. `tools/leak-check.sh` under WSL is clean over the two scratch
fixtures that exercise `pick`/`sample`/`shuffle`'s new retain edges.

**`Random::bytes` is the one row left, and it is blocked on the same thing spec § 7 is**:
`mwl_runtime::Tag` has no `Bytes` variant, so no member can construct a fresh `bytes` value. That
also blocks `Core\Encoding`, `Core\Bytes` and `Hash::of`/`hmac`/`equals`. The next group routes
around it again.

`tools/gen-attribution.py` gained one behaviour change this session, recorded in its own comment: an
SPDX `OR` now declines a branch `PREFERENCE` does not rank as long as another branch *is* ranked,
which is cargo-deny's own reading. `rand` pulls `getrandom`, which pulls `r-efi` for the UEFI target,
offered as `MIT OR Apache-2.0 OR LGPL-2.1-or-later`; MWL takes MIT and the LGPL half never enters the
tree. An `OR` with no ranked branch still fails, which is where the "someone decides" gate belongs.

## Next group — `Core\Uuid`, spec § 11's second table

**Shared file set:** `crates/mwl-stdlib/src/uuid.rs` (new), `crates/mwl-stdlib/src/lib.rs`
(`pub mod random;` at `lib.rs:195`, the `.or_else` at `lib.rs:242`), `crates/mwl-stdlib/src/registry.rs`
(`CLASSES` at `registry.rs:603`), `crates/mwl-stdlib/Cargo.toml` + the root `[workspace.dependencies]`,
and `tests/conformance/core/`. The spec table is `docs/spec/01-core-library.md:726-730`. This is the
same file set the `Core\Random` group just used, plus one shape it did not need: `Uuid` is a
**`Core`-owned instance**, so copy `crate::time::INSTANT` (`time.rs:684`) rather than
`crate::path::CLASS` — the roster is `CoreClass::instance` + `slots`, and `mwl_stdlib::instance`'s own
module doc owns why a `Core` instance is an ordinary MWL object.

- [ ] **`Uuid::v4` and `Uuid::v7`, with the dependency picked and the instance shape stood up.**
      Both return `Uuid`, so the slot layout and `CoreTy::Instance` registration land here. The pick
      is against `docs/adr/0051-standard-library-tiers.md` § 4's two questions and owes the same
      three things `rand` just owed (AGENTS.md): the `[workspace.dependencies]` line saying why that
      crate, `cargo deny check`, and `python tools/gen-attribution.py`. `v4` is 122 random bits —
      draw them through `rand`, which is already a dependency — and `v7` is a millisecond timestamp
      plus randomness, so it is time-ordered for a database key.
- [ ] **`Uuid::parse` and `Uuid::isValid`**, plus whatever instance member § 11 writes for rendering
      one back to a string (read the section's prose at `docs/spec/01-core-library.md:726-742`, not
      just the table). `parse` throws on a malformed input (R4) and `isValid` is its `bool` twin, so
      the two share one parser reached from two entry points.
- [ ] **Conformance cases under `tests/conformance/core/`** — one per group above, *in the same slice
      as the rows*, per `playbook.md` § *Writing a test case*'s first bullet. A generated UUID cannot
      be asserted, so pin the invariants: the version and variant nibbles, `Uuid::isValid` over a
      freshly rendered one, `parse` round-tripping a fixed string, and two `v7`s ordering by
      generation.

## Backlog

- `Random::bytes`, `Core\Encoding`, `Core\Bytes`, `Hash::*` — all wait on a `mwl_runtime::Tag::Bytes`
  variant (`mwl-ir`'s `ty` module doc names the gap).
- `Core\Random\Seeded` — spec § 11's prose; a separate type, not an option (`random.rs` gap 2).
- `Core\Uri` and `Core\Csv` — spec § 12, the rest of `examples/collect.mwl`'s roster; `Uri` reuses the
  instance shape the `Uuid` group stands up.
- `ObjectSet`/`ObjectMap` — additionally need `new Core\X<T>()` to parse (plan, *Open now*).
- Stage 0 catch-up items still open: ADR 0088's registry-wide qualifier classification (plan,
  *Open now*).
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain (`tools/check-migration.py`).

`orient.py` printed everything this session needed. One cost worth avoiding is now a playbook bullet:
`plan.py --get "Open now"` prints ~10,000 tokens.
