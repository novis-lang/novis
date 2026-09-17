# Handoff

## State

**Goal `decided-closures`, stage 2 — the runtime and the lowering.** Stage 1's floor is the closed goal
`cache-shared-dial`'s checks, and they pass.

`crates/nvs-runtime/src/lib.rs` gap 2 is **built and deleted**: `nvs_str_concat` and `nvs_str_concat_n`
consume one reference to their leading operand and write into its buffer when nothing else holds it, so
`$s = $s . $x` is linear. The protocol's one home is `nvs_ir::ir::InstKind::Concat`'s doc comment; the
runtime body is `crates/nvs-runtime/src/string.rs`'s `concat_onto`. Three of the stage's four items are
left, and nothing is blocked.

## Next group

**Stage 2: the runtime and the lowering, continued** — one file set: `crates/nvs-runtime/src/decimal.rs`,
`crates/nvs-runtime/src/record.rs`, `crates/nvs-runtime/src/lib.rs`, `crates/nvs-ir/src/lib.rs`.

- [ ] **A 128-bit overflow retries at 192 bits** — `crates/nvs-runtime/src/decimal.rs:45`, the gap over
      `Decimal::checked_div`'s scale fold; exact everywhere, the common path untouched, per
      `rule:types/decimal` and ADR 0054 § *Consequences*.
- [ ] **`record.rs` gap 2 is struck** — `crates/nvs-runtime/src/record.rs:55`: through `mixed` an enum is
      its backing integer, so the bound becomes the module's own prose and the numbered item goes
      (`rule:types/conversion`).
- [ ] **The collector**, if the group's context allows a third slice — `crates/nvs-ir/src/lib.rs:614`
      with `crates/nvs-runtime/src/lib.rs:199` and `crates/nvs-runtime/src/lib.rs:206`, one build, run
      only near the memory ceiling (`rule:programs/memory-priority`).

## Backlog

- Stage 2's remaining items: `record.rs` gap 1 (the compile-time refusal lives in `nvs-types`),
  `routes.rs` gap 1 (measure first), `metrics.rs` gap 1 (the `otlp` pusher, `crates/nvs-server`), and
  `nvs-ir/src/lib.rs` gap 18 — the last two share no files with the group above.
- Stage 3 (`nvs-types`, `nvs-hir`, `nvs-syntax`, `nvs-diagnostics`) and stage 4 (`nvs-stdlib`), each
  its own file set; stage 4's prepared-pattern channel is the goal's one ADR.
- When this goal's last check goes green the driver takes goal `gap-zero`.
