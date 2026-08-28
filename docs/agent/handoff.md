# Handoff

## State

**M4's Stage 6 is closed and Stage 7 is the frontier.** Every named `.nvst` case
`python tools/loop.py --list` reports is on disk except one, and that one is owed
rather than misnamed.

- The two stale names in `docs/agent/loop-goal.toml` now point at the files that
  landed: ADR 0046 §§ 4-5's retrieval is
  `core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst` (structural, not
  nominal), and ADR 0092's dump is its two cases — § 3's plaintext view and § 5's
  property redaction — which one `.nvst` cannot hold.
- **Nothing of ADR 0079 exists**: `#[Test]` is not on `nvs_types::derive::ATTRIBUTES`
  (`crates/nvs-types/src/derive.rs:72`), `Core\Test` has no row in
  `nvs_stdlib::registry`, and no table is built. Stage 7 is unstarted work, not a
  gap in a landed feature; the toml says so at the case rather than leaving it to be
  re-derived.
- The conformance corpus is at **725** against Stage 8's floor of 750.

## Next group

**ADR 0079 § 1 — `#[Test]` and the table the compiler builds from it**, which is
Stage 7's first item and the three `nvs-types` guards `loop-goal.toml:897` names.
One file set: `crates/nvs-types/src/derive.rs`, `attributes.rs`, `check.rs`,
`expr_table.rs`, plus `docs/adr/0079-testing-is-a-language-feature.md`.

- [ ] **`#[Test]` joins the compiler-recognized roster and its payload is checked**
      (ADR 0079 § 1; ADR 0071 § 1 owns the roster's closure) —
      `crates/nvs-types/src/derive.rs:72` is `ATTRIBUTES`, `:75` the two existing
      names; the payload shape is `{skip?: string, at?: string, seed?: int,
      db?: string, server?: bool, retries?: int, because?: string}`, validated as an
      ADR 0046 shape literal, and `crates/nvs-types/src/attributes.rs:90`'s
      `check_attribute` is where a recognized name is already exempted from the
      shape-alias resolution.
- [ ] **The table is collected while checking, and rides in `ExprTypeTable`** —
      `crates/nvs-types/src/check.rs:226` is the per-class call
      `derive::check_class_derive` already takes, `derive.rs:148` its shape, and
      `crates/nvs-types/src/expr_table.rs:599` the table it is recorded into, for
      `DerivedCodec`'s reason. Guard: `a_test_attribute_builds_a_table_of_its_cases`.
- [ ] **§ 1's five compile errors** — a duplicate name in one class, a `static` /
      non-`void` / non-`public` `#[Test]`, an unfilled parameter, and
      `#[Test(skip: true)]`. Next free type code is **E0733** (the `E04xx` band is
      full at `E0499`).

## Backlog

- ADR 0079 §§ 8-9's `#[Fixture]`/`#[TestWith]` — the other two `nvs-types` guards
  (`docs/agent/loop-goal.toml:897`).
- ADR 0079 §§ 4-6's `Core\Test` roster and ledger, and § 22's three reporters — the
  `nvs-stdlib` guards at `docs/agent/loop-goal.toml:906`.
- `tests/conformance/lang/a-test-attribute-builds-a-table-the-runner-reports.nvst`,
  written by item 43 once the table above exists.
- The conformance floor: 725 on disk against Stage 8's 750
  (`docs/agent/loop-goal.md` § *Stage 8*).
- `nvs_types::derive`'s known gap 1 (`crates/nvs-types/src/derive.rs:36`) says a
  promoted constructor parameter is not a field; the plan records promotion as
  landed in `layout` and `signatures`, so that sentence is likely stale — check
  before trusting it.
- A `require` whose path is not a string literal runs nothing at all, silently, in
  both forms (`nvs_hir::requires`' own known gap).

**The orientation pack was missing Stage 7 entirely.** `[context] adrs` names no
section of ADR 0079 and `[context] modules` no `nvs-types/src/{derive,attributes}.rs`,
so this session read both by hand; the next group needs `0079 §§ 1, 8, 9, 24` and
those two module lines added.
