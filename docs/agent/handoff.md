# Handoff

## State

**Item 1 of the group is closed, and it was five checks plus one hole.**
`emit.rs`'s six remaining internal panics each carry a roster comment in the
shape `emit_binop`'s has, naming what subtracts to nothing and why; the plan's
`Open now` holds that roster. The sixth was not a check: a `mixed` operand under
unary `-`/`~` reached the catch-all, and it is closed by `Helper::ValueNeg` and
`Helper::ValueBitNot`, the tag-decided pair `nvs_runtime::helpers::value_neg`
and `value_bit_not` are the home of.

- **`emit.rs:1310`'s `the binary operator {other:?}` is the group's item 3 and is
  deliberately untouched** — that is static `float` `%`, refused at both ends
  because PHP's `%` converts to an integer where ADR 0007 § 4's float row would
  not. It is the one refusal site in this file with no roster comment, on
  purpose.
- **`orient.py`'s pack was complete for this item.** The two standing gaps are
  unchanged: `[context] modules` has no `nvs-runtime` entry — this session read
  `crates/nvs-runtime/src/helpers.rs` for the `value_arith` table it added the
  unary twin beside — and none for `nvs-diagnostics`.
- `MSYS_NO_PATHCONV=1` is needed in front of `wsl.exe -- bash /mnt/d/...` from
  the Bash tool; the playbook already records the trap in its other spelling.

## Next group

**`nvs-codegen` and `nvs-ir`'s remaining M4 holes, over the two files this
session already opened.** The files: `crates/nvs-codegen/src/emit.rs` and
`crates/nvs-ir/src/lower/convert.rs`. `python tools/holes.py --item N` prints any
of these in full.

- [ ] **Item 26, `bool as int` and `bool as string`** — ADR 0007 § 2's grid rows
      for a `bool` source, which `convert` has no row for today.
      `crates/nvs-ir/src/lower/convert.rs:84` is the row table's head.
- [ ] **`float` `%`, at both ends at once** — `crates/nvs-codegen/src/emit.rs:1310`
      is the static refusal and `nvs_runtime::helpers::value_arith`'s `float_arith`
      guard the tagged one. ADR 0007 § 4 has no row saying whether PHP's
      convert-to-integer reading applies; taking the decision is the slice.
- [ ] **A `.nvst` case for the five roster comments' claims that are testable** —
      only the refcount one has an observable shape (a widened operand released
      as a scalar); the other four subtract to nothing a program can reach.
      Low value, listed so it is not re-derived.

## Backlog

- `float` `%` behind a `mixed` is refused rather than answered — `nvs_runtime::helpers::value_arith`'s doc.
- ADR 0007 § 4's closure names `Helper::ValueLt` as "its one home"; three families answer it now — `docs/adr/0007-explicit-type-system.md` § 4.
- `[context] modules` gains `nvs-runtime` and `nvs-diagnostics` — `docs/agent/loop-goal.toml`.
- Virtual dispatch by slot, `br_table` for a dense `switch` — M12, `docs/plan/m12.md`.
- String-literal deduplication in a unit's data section — `nvs-codegen` gap 4.
