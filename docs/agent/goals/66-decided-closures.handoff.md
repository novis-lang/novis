# Handoff

## State

**Goal 66 — every gap the decision sheet answered is built to its answer — has just started; nothing
of it has landed yet.** Goal `cache-shared-dial`'s whole list is this goal's Stage 1 floor.

Everything is settled before the first session. Forty of the forty-eight items carry the user's own
`Decided:` sentence from the sheet of 2026-09-13, and the eight owed to M1, M6, M7 and M8 state in their
own prose what closes them; `grep -rn "owner: decided-closures" crates/` is the whole list. The gate is
`python tools/owners.py --closes decided-closures`, red while any item names the goal, and the driver
asks the same of every goal at its end — so an item leaves only by being built and deleted, struck as a
stated bound, or deferred to M9+ with the plan's scope sentence. Nothing is re-decided; § *Standing
decisions* says what a session does when the chosen option is harder than priced.

## Next group

**Stage 2: the runtime and the lowering** — one file set: `crates/nvs-runtime/src/lib.rs`,
`crates/nvs-runtime/src/decimal.rs`, `crates/nvs-runtime/src/record.rs`, `crates/nvs-ir/src/lib.rs`.

- [ ] **`concat` reuses a solely-owned left operand** — `crates/nvs-runtime/src/lib.rs` gap 2, and the
      test `concat_reuses_a_solely_owned_left_operand` that pins `$s = $s . $x` as linear.
- [ ] **A 128-bit overflow retries at 192 bits** — `crates/nvs-runtime/src/decimal.rs` gap 1; exact
      everywhere, the common case untouched.
- [ ] **`record.rs` gap 2 is struck** — through `mixed` an enum is its integer; the bound becomes the
      module's prose and the item goes.
- [ ] **The collector**, if the group's context allows a third slice — `crates/nvs-ir/src/lib.rs` gap
      14 with `crates/nvs-runtime/src/lib.rs` gaps 5 and 7, one build, run only near the ceiling.

## Backlog

- Stage 2's remaining items: `record.rs` gap 1 (the compile-time refusal lives in `nvs-types`),
  `routes.rs` gap 1 (measure first), `metrics.rs` gap 1 (the `otlp` pusher, `crates/nvs-server`), and
  `nvs-ir/src/lib.rs` gap 18 — the last two share no files with the group above.
- Stage 3 (`nvs-types`, `nvs-hir`, `nvs-syntax`, `nvs-diagnostics`) and stage 4 (`nvs-stdlib`), each
  its own file set; stage 4's prepared-pattern channel is the goal's one ADR.
- When this goal's last check goes green the driver takes goal `gap-zero`.
