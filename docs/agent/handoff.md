# Handoff

## State

**M4's Stage 7 is closed: everything ADR 0079 § 24 puts at M4 is on disk**, § 9's
runner half having landed last. `python tools/loop.py --list` reports no named
`.nvst` case owed by any stage, so the frontier is **Stage 8's corpus floor of
750** — the tree is at **742**.

- **Each `#[TestWith]` row is its own reported case.**
  `nvs_cli::runner::invocations` (`crates/nvs-cli/src/runner.rs:556`) is the one
  home of the expansion and of the `method#N` label; ADR 0079 § 9's own body
  states the rule, including why a `skip:` is stated per row.
- **A row's constants are materialized per call and released when it returns** —
  `nvs_runtime::RowValues` (`crates/nvs-runtime/src/dispatch.rs:474`), a
  per-call owner beside `Fixtures`' per-class one. One method per constant
  shape, so the single reference each value carries never leaves that type.
- **`run_case` walks `TestCase::params` positionally** and takes a fixture's
  value borrowed from the class's set or a row's from the owner above; a hole in
  either is reported as an internal inconsistency rather than called around.
- **A fixture is shared, not copied**, until § 2's isolates (M5) — and so is
  nothing about a row, which is built fresh per case. `nvs_cli::runner`'s module
  doc and ADR 0079 §§ 8-9 are the two homes.
- **A `static` method called from native code needs its slot 0 filled with the
  called class** (`nvs_runtime::Value::class_desc`); the playbook bullet under
  *Running things* is that trap's home.
- `tools/leak-check.sh --test` is how a `nvs test` path is valgrinded; a
  `#[Test(retries: N, because: …)]` over a failing assertion is how a per-call
  edge is driven N times.

## Next group

**Conformance depth toward Stage 8's floor of 750, over the thinnest two classes
`python tools/gaps.py` ranks.** The file set is
`crates/nvs-stdlib/src/time.rs` alone, and the cases go under
`tests/conformance/core/`:

- [ ] **`Core\Time\DateTime` — depth 0.06, one case over 17 members**, and three
      of them no case calls at all: `date` (`crates/nvs-stdlib/src/time.rs:2369`),
      `dayOfYear` (`:2410`), `difference` (`:2333`). Reach for *agreement* —
      one question asked of every member that shares a rule, counted — rather
      than another row of the same shape (`docs/agent/conventions.md`).
- [ ] **`Core\Time\Instant` — depth 0.11, one case over 9 members**, with
      `compareTo` (`crates/nvs-stdlib/src/time.rs:1822`), `in` (`:2453`) and
      `minus` (`:1796`) uncalled. A *bound asserted on both sides* is the shape
      the ordering members ask for.
- [ ] **`Core\Time::now`/`monotonic`/`at` have a PHP twin and no oracle case**
      (`crates/nvs-stdlib/src/time.rs:1634`, `:1645`, `:1140`) — those go in
      `tests/differential/`, never in `tests/conformance/`, and PHP computes the
      expectation.

## Backlog

- `Core\Csv::format`'s "column N is not a `string`" throw is still unreachable
  from source (`crates/nvs-stdlib/src/csv.rs:512`) — the playbook bullet owns why.
- `Core\Test::assertEqualsDeep`'s depth-cap throw is unasserted
  (`crates/nvs-stdlib/src/test.rs:692`).
- A `require` whose path is not a string literal runs nothing, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- ADR 0079's `#[TestSource]` (§ 9) and doubles (§§ 10-11) are M4S/M5, not this
  goal.
- `nvs_stdlib::debug`'s known gap 1: an ADR 0036 shape field and an `array<T>`
  element carry no `secret` bit.
