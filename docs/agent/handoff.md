# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2 are done, and stage 3 is two thirds done**: `allocate` and
`pow` are on disk at `crates/nvs-stdlib/src/decimal.rs:80`, each with its named `#[test]` and three
conformance cases. What is left of the stage is the four rounding members.

Stage 3's `cargo-named` check stays red until they land — it names four tests and three of them now
run (`decimal_allocate_parts_add_back_to_the_amount_exactly`,
`decimal_allocate_refuses_an_empty_zero_or_negative_ratio_list`,
`decimal_pow_is_exact_or_throws_at_the_mantissa_or_scale_bound`); the fourth,
`decimal_floor_ceil_truncate_and_round_answer_decimal_at_the_scale_asked`, does not exist yet. Its
`nvs-suite` check is two cases of three, missing
`tests/conformance/core/decimal-rounding-members-answer-decimal-and-name-their-mode.nvst`.

**The calls this session settled are in the module doc, not in a record** — the goal's § *Standing
decisions* pre-authorized each. `allocate` splits its refusals: an empty, all-zero or negative ratio
list is a `LogicError` (a bad argument), and a share no `decimal` holds is an `ArithmeticError` (the
bound the operators have). The remainder goes by **position**, so a zero ratio written first takes a
unit; keys are the ratios', so a named split answers under those names. `pow` agrees with repeated
`*` on the scale as well as the value, and squares rather than walks so an exponent at `uint`'s
ceiling terminates.

`docs/spec/01-core-library.md` has **no `Core\Decimal` member table** — the hand-written home is
`docs/reference/core/Decimal.md`, whose example blocks `tools/verify.py`'s reference step runs.

## Next group

**Stage 3: the four rounding members, and `Core\RoundMode` beside them** — one file set:
`crates/nvs-stdlib/src/decimal.rs` (the roster at `:80`, the five edits per member), the enum's
reader in `crates/nvs-stdlib/src/math.rs`, and cases under `tests/conformance/core/`. Each member is
a row, a card, a body, an `address()` arm and cases; `rule:types/arithmetic` is the rule and the
goal's § *Standing decisions* the wording.

- [ ] **`floor`, `ceil` and `truncate`, each taking a target scale that defaults to 0** —
      `crates/nvs-stdlib/src/decimal.rs:80`. They answer `decimal` at that scale, never `int`:
      `rule:types/conversion` makes the scale observable, so `floor($x, 2)` renders its two places.
      A `defaults:` entry is what carries the 0 — `crates/nvs-stdlib/src/decimal.rs:88` is a row
      with none. The check names
      `decimal_floor_ceil_truncate_and_round_answer_decimal_at_the_scale_asked`.
- [ ] **`round`, taking `Core\RoundMode` with no default** —
      `crates/nvs-stdlib/src/decimal.rs:290`'s `rounds_away` is already the six modes over a
      truncation and a `Discard`, and `crates/nvs-stdlib/src/math.rs:2203`'s `round_mode` reads the
      argument. Naming the mode is the point, so no default (the `divRound` doc at
      `crates/nvs-stdlib/src/decimal.rs:236` argues it).
- [ ] **Three cases, one of them named by the check** —
      `tests/conformance/core/decimal-rounding-members-answer-decimal-and-name-their-mode.nvst`
      plus two more: `crates/nvs-stdlib/tests/conformance_coverage.rs:155` is a floor of three
      *per member*, and one case naming all four members counts for each of them.

## Backlog

- A leftover unit can land on a zero ratio that comes first; pinned by
  `tests/conformance/core/decimal-allocate-keeps-the-ratios-keys-and-the-amounts-sign.nvst`, and a
  decision to reopen only if skipping zero ratios is wanted.
- The pack's `[context] playbook` is filtered to the item's own paths, so the trap at
  `docs/agent/playbook.md:3889` — a negative `decimal` literal does not compile, and `0 - 1` does
  not either; it needs `decimal $one = 1; 0 - $one` — did not print and cost two runs.
- `crates/nvs-stdlib/src/decimal.rs`'s `# Known gaps` item is now the four rounding members alone,
  and closes with the group above.
