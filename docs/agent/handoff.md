# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2–10 are done**; stage 11 (CLDR) is the next group below.
Nothing is blocked.

Stage 10 closed this session. A `Core` class that declares `rule:classes/comparable`'s member now
carries its address on its **own descriptor field** — `ClassDesc::comparer`, beside `renderer` — and
`nvs_runtime::call_compare_to` is the one entry point that knows which of the two calling conventions
a class answers under: a program's own class is a method row and is called through `call_method`,
which transfers a reference per slot, while a registered `Core` member borrows its arguments and so
would leak one reference per operand per comparison as a row. `crates/nvs-stdlib/src/instance.rs`'s
§ *Decision: a registered member the engine reaches by name gets a descriptor field* is that argument
in full, and it now covers `toString` and `compareTo` together.

Both addresses are **derived from the registry**, never written down: `registry::compare_symbol` is
asked of `implements_comparable`, which is the same question the compiler answers `$a < $b` with, so
a class the checker lets a program order cannot be one a `Core\Heap` finds no ordering for. The spelling
`"compareTo"` has one home now, `nvs_runtime::COMPARE_TO`. `crates/nvs-stdlib/src/heap.rs`'s known gap
is gone with it — step 2 of its three orderings reaches a `Core\Time\Instant` as readily as a declared
class.

## Next group

**Stage 11: CLDR** — one file set: `crates/nvs-stdlib/src/cldr.rs`, whose `# Known gaps` 2 and 3 are
this whole stage. Each item is a `Core\Cldr`/`Core\Time::format` question and touches no other module.

- [ ] **Every pattern letter once refused now formats as ICU does** — `crates/nvs-stdlib/src/cldr.rs:381`
      is `compile`, the letter-by-letter pattern compiler, and `crates/nvs-stdlib/src/cldr.rs:168` is
      gap 2's own list of what it still refuses: `U`/`r` (the year), `B`/`b` (day periods), `A`, `g`, and
      the zone spellings `z`, `Z`, `O`, `v`. The goal's § *Standing decisions* fixes every answer —
      each letter matches ICU under the Gregorian calendar, `z`/`v` fall back to the localized GMT
      format where the transcribed data names no zone, `B`/`b` read day periods for the languages the
      cardinal roster carries and take ICU's root fallback otherwise, and **no letter stays refused**.
      `rule:core-api/shape-rules` R11 is why none of them is a mode string. Closes
      `every_pattern_letter_once_refused_now_formats_as_icu_does`.
- [ ] **`Y`, `e` and `c` read the territory's week data** — the same `compile` at
      `crates/nvs-stdlib/src/cldr.rs:381`; gap 2 names `Y` as the one with a caller waiting, because a
      week-based year beside `w` is the pair ISO 8601 writes. The week data (first day, minimal days)
      is per territory, so it lands beside `RULES` (`crates/nvs-stdlib/src/cldr.rs:1802`) as a table of
      its own rather than inside the compiler. Closes
      `a_week_based_year_and_local_weekday_read_the_territorys_week_data` and the case
      `tests/conformance/core/cldr-formats-a-week-based-year-beside-its-week.nvst`.
- [ ] **The ordinal roster is CLDR's** — `crates/nvs-stdlib/src/cldr.rs:2205` is `ORDINALS`, and gap 3
      at `crates/nvs-stdlib/src/cldr.rs:178` states its failure mode: a language that marks a form but
      is missing from the table answers `Other` silently. Widening it is a row per language and an arm
      only where the published rule is a shape no arm has; `crates/nvs-stdlib/src/cldr.rs:2265`'s
      `rules_for` is the boundary both members share and stays the refusal for a language neither table
      carries. Closes `every_language_cldr_gives_an_ordinal_rule_is_on_the_ordinal_roster`.

## Backlog

- `crates/nvs-stdlib/src/ordering.rs:36-46` still says reaching an instance method from a helper "is
  the thing that is not built yet", so `Core\Arr::sort` over `Comparable` objects throws where a
  `Core\Heap` now orders. `rule:classes/comparable` names no member, so which of the two is right is a
  question for the goal that owns `Core\Arr::sort`'s natural order.
- Stages 12–15 are `Core\Metrics`, `Core\Process::spawn`, the verification remainder and the rulebook,
  in `docs/agent/loop-goal.toml`'s own order.
