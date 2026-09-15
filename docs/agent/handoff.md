# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 0 and stage 2 are done**; stages 3–15 have not started. Stage 2's
three checks are green on disk: `rule:types/arrays` states the covariant read, `cargo test -p
nvs-types` holds its three named tests, and both conformance cases run.

`array<T>`'s covariance was already in the checker (`crates/nvs-types/src/expr/assign.rs:172`) and is
now pinned by tests rather than by a comment. `Core\Arr::flip($stringArray)` compiles — confirmed by
running it, not by reading the relation.

The gap is struck from both its homes: `crates/nvs-stdlib/src/lib.rs`'s known-gaps list is three
items, and `docs/agent/carried-gaps.md` no longer carries the row.

**ADR 0188 is this goal's only record, and the goal's § *Standing decisions* is not contradicted.**
"ADR slots: none" is about design calls, and nothing here was decided this session — 0188 writes down
the call the user's decision sheet already made, because `python tools/rules.py --check` refuses a
rule whose `because` names anything but a record. Its `changes.modifies` carries one rule the pack did
not name: `rule:ide/narrowing-is-a-diff-never-a-save-time-fix` rested its whole argument on the
invariance, so it is re-argued on the write side, retitled, and `docs/plan/m10.md`'s bullet now points
at it instead of restating the premise.

The pack's `[context] shapes` has no *A `.nvst` case* entry, so writing the two cases meant opening a
sibling case to learn the file's sections.

## Next group

**Stage 3: `Core\Decimal`'s roster — `allocate`, `pow`, and the four rounding members** — one file
set: `crates/nvs-stdlib/src/decimal.rs`, the spec row each member needs, and three new cases under
`tests/conformance/core/`. The roster is two rows today (`divExact`, `divRound`) at
`crates/nvs-stdlib/src/decimal.rs:56`, and every item below adds to that one array plus its
`nvs_core_decimal_*` symbol. `crates/nvs-stdlib/src/decimal.rs:38` is a `# Known gaps` item tagged
with a passed milestone (§ *Backlog*) and is worth reading before the first row lands.

- [ ] **`allocate(decimal $amount, array<…> $ratios): array<decimal>`** —
      `crates/nvs-stdlib/src/decimal.rs:56`. Each part is its ratio's share rounded down and the
      remainder goes one smallest unit at a time to the earliest parts, so the answer is
      deterministic and sums exactly; an empty, all-zero or negative ratio list throws. The wording is
      the goal's § *Standing decisions*, `rule:types/arithmetic` is the rule, and the checks name
      `decimal_allocate_parts_add_back_to_the_amount_exactly`,
      `decimal_allocate_refuses_an_empty_zero_or_negative_ratio_list` and
      `tests/conformance/core/decimal-allocate-splits-a-sum-into-parts-that-add-back.nvst`.
- [ ] **The four rounding members, and `Core\RoundMode` beside them** —
      `crates/nvs-stdlib/src/decimal.rs:56`. `floor`, `ceil`, `truncate` and `round` each take a
      target scale defaulting to 0 and answer `decimal`; `round` takes `Core\RoundMode` with **no**
      default, naming the mode being the point. `rule:types/arithmetic`. The checks name
      `decimal_floor_ceil_truncate_and_round_answer_decimal_at_the_scale_asked` and
      `tests/conformance/core/decimal-rounding-members-answer-decimal-and-name-their-mode.nvst`.
- [ ] **`pow`, exact or throwing** — `crates/nvs-stdlib/src/decimal.rs:56`. It takes a `uint`
      exponent and is exact or throws at the mantissa or scale bound; a negative power is written
      `divRound(1, pow(…))`, which says the rounding out loud. `rule:types/arithmetic`. The checks
      name `decimal_pow_is_exact_or_throws_at_the_mantissa_or_scale_bound` and
      `tests/conformance/core/decimal-pow-is-what-a-decimal-base-uses-instead-of-star-star.nvst`.

## Backlog

- 16 `# Known gaps` items are tagged with a milestone the program has passed; 8 are in this goal's
  own files and owned by one of its stages — `crates/nvs-stdlib/src/ast.rs:46`, `cli.rs:137`,
  `csv.rs:128`, `debug.rs:81`, `decimal.rs:38`, `math.rs:33`, `reflect.rs:73`/`:78`/`:83`. Stage 0
  re-pointed `out.rs` alone, because only that one's prose was wrong as well. `python tools/owners.py`.
- CLDR's gap 4 is now gap 3. `docs/agent/goals/59-m8-stdlib-depth.md:202` (stage 11) and its
  `.toml`'s check comment at `:4320` still say "gap 4" and "gaps 2-4".
- `every_language_named_absent_in_the_gap_note_now_has_a_rule` outlives the note it is named for;
  `docs/agent/goals/59-m8-stdlib-depth.toml:4328` names it as a guard, so a rename is a two-file edit.
- Stage 4 and stage 13's guard share `benches/abi-probe/tests/perf_guards.rs`, so a session holding
  that file can take both guards once `spawn` exists. Stages 3 and 4 share nothing else.
- Two retired goal files still describe striking `crates/nvs-stdlib/src/lib.rs` gap 4 as open work —
  `docs/agent/goals/31-unowned-sweep.md:23`/`:79` and `docs/agent/goals/52-plan-truth.md:116`/`:123`.
  Retired goals are frozen, so this is a note rather than an edit.
- When this goal's last check goes green the driver takes goal `unowned-closures`.
