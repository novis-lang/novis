# Handoff

## State

**M4's frontier is Stage 5's remainder: items 33 and 36 of `docs/agent/loop-goal.md`.** Item 39 is
closed whole — its last gap, the `foreach` key binding, landed this session, and `nvs-types`'
known-gaps list (`crates/nvs-types/src/lib.rs:185`) now has **no** entry left at all.

- **The mechanism has one home**: `nvs_types::expr::iteration::check_foreach_key`
  (`crates/nvs-types/src/expr/iteration.rs:343`) decides every `foreach` binding-type refusal — a
  cursor's key (`E0444`), and an array's key declared at anything but `string` (`E0723`). ADR 0007 § 5
  is the rule and now states the refusal in its own body.
- **The next group was named as "items 33 and 36" and they do not share a file set with each other or
  with anything loaded here** — 33 is ADR 0092 plus `Core\Debug::dump` (a whole feature, `nvs-stdlib`),
  36 is `nvs-codegen` gap 3. This session stopped after one slice for exactly that reason; the group
  below is one item each, and there is no third to pair either with.
- **`orient.py`'s pack was complete for this item** except that `[context] modules` prints no map line
  for `nvs-types`' `expr/iteration` or `lib`, both of which this session edited, and `[context] adrs`
  had no ADR 0007 § 5 entry — the section the whole slice is written against, and one `peek.py` call to
  fetch.

## Next group

**Two independent items, in this order; neither shares files with the other, so expect one per
session.** Read each item's own line in `docs/agent/loop-goal.md` first — neither is in `[context]`.

- [ ] **Item 36 — a `FATAL` releases the frame's locals**, `nvs-codegen` gap 3, and the smaller of the
      two. A `THROWN` outcome gets a landing block; an outcome no cleanup path and no `catch` can act
      on gets none, and `nvs_ir::ir::Inst::on_error` owns that asymmetry
      (`crates/nvs-ir/src/ir.rs`, `python tools/peek.py --locate on_error`). The file set is
      `crates/nvs-codegen/src/emit.rs` plus that field's own doc.
- [ ] **Item 33 — ADR 0092 and `Core\Debug::dump`**, including ADR 0033's redaction of a
      `secret`-qualified property, which that ADR's *Verification* defers to M4 by name. This is a
      whole `Core` member rather than a hole: read ADR 0092 and ADR 0033 § *Verification* before
      touching `crates/nvs-stdlib/src/registry.rs`, and expect it to want a session to itself.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- `array<K, V>` — narrowing a key type properly — is its own decision and is not on this goal.
- Stage 6's items 37–42 (`docs/agent/loop-goal.md`) are the frontier once Stage 5 closes.
- `[context] modules` in `docs/agent/loop-goal.toml` prints no map line for `nvs-types`'
  `expr/iteration` or `lib`; `[context] adrs` has no ADR 0007 § 5 entry.
