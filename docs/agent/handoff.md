# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 3 is closed; stage 4 is the next group.**

The name comparison is gone from the checker. `crates/nvs-types/src/commands.rs`'s `reaches_parses`
(`:817`) is the two-table question `crate::expr::operators`'s `reaches_comparable` already asks of
`Comparable` — `nvs_hir::implements_interface` for a written `implements Parses`, and
`crate::signatures::resolve_interface_args` for `Core\Uuid`, whose edge `crate::core_lib:92` seeds off
`nvs_stdlib::registry::implements_parses`. Both of `commands.rs`'s readings go through it, and
`converts_from_string` is the roster `routes.rs:1724` and `:1818` read, so all four surfaces widened at
once. The three diagnostics and the two doc comments now say "a class implementing `Parses`".

**`ArgConv::Uuid` became `ArgConv::Parses(String)`** — the row carries the class the checker resolved,
which is what stage 4's arm needs. `nvs-cli/src/main.rs:1270` is the bridge: `Core\Uuid` reaches
`nvs_runtime::commands::ArgConv::Uuid`, and every other implementor is `Unconverted`, under the policy
`capture_conv`'s doc (`crates/nvs-cli/src/main.rs:1384`) already states — what the runtime has no arm
for says so rather than converting as something else. Written down as gap 2 of
`crates/nvs-runtime/src/commands.rs`.

**The goal doc's stage 4 quotes two gaps the tree no longer has.** `crates/nvs-runtime/src/routes.rs`'s
gaps 1 and 2 are the linear scan and the reader; `CaptureConv::Decimal` and `CaptureConv::Uuid` both
have arms (`:101`, `:107`, matched at `:403`), and so does `ArgConv::Uuid`
(`crates/nvs-stdlib/src/command.rs:535`). So stage 4's items 1 and 3 are already on disk and what is
actually left is one arm per enum for a `Parses` class that is **not** `Core\Uuid` — and the open
design question is how the runtime reaches a user class's static `parse` from a conversion path that
today calls Rust readers only.

The goal's one record is still unwritten; stage 5 schedules its prose half, and it owes stage 3's
`ArgConv::Uuid` → `ArgConv::Parses` widening in its `changes` block alongside the `Core\Uuid`
reclassification, the `Core\Uri` exclusion, `rule:core-api/reserved-namespace` and stage 2's `E0404`.

## Next group

**Stage 4: the runtime arm, in the two enums that cross** — one file set:
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-runtime/src/commands.rs`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-stdlib/src/command.rs`.

- [ ] **Settle how a conversion path reaches a class's `parse`, then write it once** —
      `crates/nvs-stdlib/src/command.rs:535` is where a command argument converts and
      `crates/nvs-runtime/src/routes.rs:403` is where a segment does; both call Rust readers and
      neither can call a compiled static today. `rule:expressions/try-parse` is the contract the arm
      honours. Prefer the safe answer if the reach is not cheap: leave the arm `Unconverted` and say so
      in the gap rather than converting at the wrong type.
- [ ] **`CaptureConv` gains the parsing arm** — `crates/nvs-runtime/src/routes.rs:107` is the
      `Core\Uuid` variant it generalizes and `crates/nvs-runtime/src/routes.rs:403` its match arm; a
      segment `parse` refuses is **no match**, per
      `rule:security/route-capture-is-laundered-by-its-type`.
- [ ] **The two `nvs-cli` bridges stop naming the class** — `crates/nvs-cli/src/main.rs:1385` is
      `capture_conv`'s `Some(r"Core\Uuid")` arm and `crates/nvs-cli/src/main.rs:1270` is the
      `ArgConv::Parses` arm this session wrote as a two-way `if`; both become the widened variant once
      the runtime has somewhere to send it.

## Backlog

- Stage 5's record and its `.nvst` proofs, including `examples/parses.nvs` — the acceptance check the
  driver reports as failing (`docs/agent/loop-goal.md` § Stage 5).
- `crates/nvs-types/src/commands.rs:189`'s `OneOf` doc still uses `Core\Uuid` as its worked example; it
  is true, not stale, and is the record's business if anything's.
- `crates/nvs-cli/src/openapi.rs`'s `format: uuid` stays a name test on purpose — `docs/agent/loop-goal.md`
  § Standing decisions.
