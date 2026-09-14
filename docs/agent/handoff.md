# Handoff

## State

**Goal `m4-refusals` — Stage 5 is closed, both halves; Stage 6 is the next group.**
`python tools/holes.py` reports **6** refusal sites, `UNATTRIBUTED: 0`, **13** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **6** to match.

- `crates/nvs-ir/src/lower/exception.rs:150`'s `split_on_object_tag` is the one tag question
  `throw` and `clone` now share: `InstKind::TagIs` against `Ty::Object`, an `InstKind::Untag`
  on the side where it holds, and the other block handed back for the caller's own refusal.
- `crates/nvs-ir/src/lower/expr.rs:5353`'s `guard_cloneable` fills that block with
  `Helper::CloneOperandNotAnObject`, which renders PHP's own `clone(): Argument #1 ($object)
  must be of type object, <type> given` from the tag in
  `crates/nvs-runtime/src/object.rs:2857`. That helper never returns, so it owns the reference
  it is handed and lowering retains a borrowed operand in front of the call; `leak-check.sh`
  over a fixture running all four ownership paths in a loop reports no leak.
- **The goal's § *Stage 5* prose quotes PHP's pre-8.5 wording** (`__clone method called on
  non-object`). PHP 8.5.9 says the `clone():` message above, and
  `tests/differential/class/cloning-null-matches-phps.nvst` is what the wording answers to
  from here. Nothing is blocked.

## Next group

**Stage 6: labels and the condition** — one file set: `crates/nvs-ir/src/lower/control.rs`,
`crates/nvs-ir/src/lower/expr.rs` and `crates/nvs-ir/src/lower/convert.rs`.
`docs/agent/loop-goal.md` § *Stage 6* is the spec, and `python tools/holes.py --item 901`
lists the three sites as they stand.

- [ ] **A `switch` label at a representation other than the subject's compares through the
      equality lowering** — `crates/nvs-ir/src/lower/control.rs:743`,
      `rule:expressions/switch-match-equality`, which says there is one comparison in the
      language and therefore one here. A pair with no equality row is a `guarded_by!` naming
      `E0466` (`rule:expressions/disjoint-comparison-refused`), never a second table.
- [ ] **A `match` arm label is that same call** — `crates/nvs-ir/src/lower/expr.rs:1929`, whose
      panic is the other half of the one above; `crates/nvs-ir/src/lib.rs`'s gap 19 is what
      claims the equality lowering covers every row of
      `rule:expressions/one-equality-operator`.
- [ ] **The truthy conversion gets an exhaustive match** —
      `crates/nvs-ir/src/lower/convert.rs:609`. `Ty::Void` becomes a `guarded_by!` naming the
      code that keeps `void` out of value position (`rule:types/declaration`), and `Ty::Ref`
      keeps the ordinary engine-invariant panic whose doc comment names `InstKind::RefLoad`.

## Backlog

- Stage 7, `is` against every type — `crates/nvs-ir/src/lower/expr.rs:5006`, goal § *Stage 7*.
- Stage 8, declared types — `crates/nvs-ir/src/lower/mod.rs:3092` and `:3311`, goal § *Stage 8*.
- Stage 9 is the goal's end gate, and `docs/agent/loop-goal.md` § *Stage 9* owns what it asks.
