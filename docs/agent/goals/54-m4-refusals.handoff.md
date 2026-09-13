# Handoff

## State

**Goal `m4-refusals` — every shape the checker admits lowers, or a diagnostic naming its rule refuses it — has just started; nothing of it has landed yet.** Goal `gap-register`'s whole list is this goal's Stage 1 floor.

**Settled before the first session; do not re-decide these.**
- `python tools/holes.py --item 901` lists sixteen sites and `CEILING` is `16`.
- A site closes in one of two ways. It lowers. Or it becomes `guarded_by!(code, …)`, naming a diagnostic
  that a conformance case expects.
- Rewording a panic past `REFUSAL` is not a close.
- `ALLOWLIST` stays empty.
- No new decision record: every design is already a rule (§ *Standing decisions*).
- M4's ADR fixtures and its valgrind item are met, so no stage is owed for them.

## Next group

**Stage 2: the keystone** — one file set: `crates/nvs-ir/src/lower/mod.rs`, `tools/holes.py`,
`crates/nvs-ir/tests/refusals.rs`, `crates/nvs-ir/src/lib.rs`.

- [ ] **`guarded_by!`** — `crates/nvs-ir/src/lower/mod.rs`. It takes an `nvs_diagnostics::code`
      constant and a message, and panics with the code at the head of the message. Rewrite
      `crates/nvs-ir/src/lib.rs:187`'s preamble whole, so it says which of the two spellings means what.
- [ ] **`holes.py --guarded`** — `tools/holes.py:166` (`sites`) is the shape to follow. It lists by
      file, then line, and each line carries the site's code and the first
      `tests/conformance/**/*.nvst` expecting `error[<code>]`, or `NO CASE`. `CONSTRUCT`'s comment
      (`:66-77`) says the source now declares its kind.
- [ ] **`every_guarded_site_names_a_code_a_conformance_case_expects`** — `crates/nvs-ir/tests/refusals.rs`,
      following `every_refusal_is_a_diagnostic_or_decided` (`:156`). It fails on any `NO CASE` line
      and names the site.

## Backlog

- Stage 3 — the seven guarded sites, `call.rs:1170`, `control.rs:974`/`:985`, `expr.rs:5229`,
  `mod.rs:2615`, `stmt.rs:263`/`:1483`. Each is a one-site edit, cheap once stage 2 is loaded. Probe
  the `foreach` subject first.
- Stage 4 — `$f(...)`, `call.rs:789`. A session of its own, or with stage 3's `call.rs` site.
- Stage 5 — tagged `throw` and `clone`, `exception.rs:28`, `expr.rs:5289`, and the runtime's throw path
  for a non-object receiver.
- Stage 6 — the labels and the condition, `control.rs:741`, `expr.rs:1927`, `convert.rs:608`.
- Stage 7 — `is` for every row, `expr.rs:5004` and `test_shape`. The largest stage.
- Stage 8 — declared types, `mod.rs:3069`, `:3288`, `:3473`.
- Stage 9 — `CEILING` to `0`, gaps 1 and 6 of `lib.rs`, and the retirement of entry 901. `CEILING`
  falls in every slice before this, so this stage is its doc comment and the retirement.
- When this goal's last check goes green the driver takes goal `m5-proofs`.
