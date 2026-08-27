# Handoff

## State

**Item 13 is landed whole: an abandoned generator runs the `finally` it is
suspended inside, and it agrees with PHP.** `mwl_runtime::object::dismantle`
calls `{name}$gen::gen#unwind` before it sweeps the field slots; the mechanism's
one home is `mwl_ir::lower::generator`'s `lower_generator` doc, the release end
is `dismantle`'s own § *An abandoned generator runs its `finally`*, and the
plan's *Open now* carries the four decisions this session took (an unspellable
method name, a `ClassDesc` field rather than a per-dismantle probe, a
membership test over the states that actually owe a `finally`, and the
resurrection to one that keeps the borrow from crossing zero).

- **One divergence from PHP is deliberate and is the only thing item 13 leaves
  open.** A throw escaping such a `finally` is discarded rather than reported
  uncaught: a release has no error edge, and the exception a landing pad is
  already carrying is what would otherwise be replaced.
  `mwl_runtime::Ctx::with_pending_set_aside` owns the argument, ADR 0028 § 2 and
  `mwl-ir`'s gap 18 both now say it, and it is a backlog item behind ADR 0020's
  ladder — not a hole.
- **`mwl_runtime::ctx::CurrentCtx` is new and is the first thread-local context
  in the tree.** `abi::call` installs it, `object::dismantle` is its only
  reader, and its doc comment states the cost (two word stores per Rust-to-
  compiled boundary, none per compiled call) and the soundness argument (a
  release performs no other context access, so the reborrow it hands out is the
  only live one). Anything else that ever wants a context off the release path
  goes through it rather than adding a second one.
- **`orient.py`'s pack was complete**, with the one gap the previous handoff
  already named: `[context] modules` still has no `mwl-runtime` entry, and this
  session read `crates/mwl-runtime/src/object.rs`, `ctx.rs`, `dispatch.rs`,
  `release.rs` and `abi.rs`. Add it before the next runtime slice.

## Next group

**The promotion table's remaining refusal sites, which is `mwl-codegen`'s
`emit.rs` twice over.** The file set: `crates/mwl-codegen/src/emit.rs`, then
`crates/mwl-ir/src/lower/`. `python tools/holes.py --item N` prints any of these
in full; the anchors below are that tool's own.

- [ ] **Item 4, the bitwise operators** — one site,
      `crates/mwl-ir/src/lower/stmt.rs:269`, with the operator table itself at
      `crates/mwl-ir/src/lower/operator.rs:470` and `crates/mwl-ir/src/ir.rs:1470`.
      ADR 0007 § 4's `& | ^ ~ << >>` row is over `int` and `uint` alone, and
      `E0706` already refuses every other operand, so what is left is the
      lowering.
- [ ] **Item 1, ADR 0007 § 4's promotion table** — the biggest remaining group,
      nine sites all in `crates/mwl-codegen/src/emit.rs` (`:660`, `:690`, `:721`,
      `:1125`, `:1251`, `:1786`, `:2778`, `:2929`, `:3286`), lowered from
      `crates/mwl-ir/src/lower/operator.rs:470` and checked at
      `crates/mwl-types/src/expr/operators.rs:403`. Take the `emit_binop` rows
      before the `Reinterpret`/tag ones.
- [ ] **Item 25, `object` as a declared type** — two sites, both in
      `crates/mwl-ir/src/lower/mod.rs` (`:2495`, `:2578`), anchored at `:2206`.
      The representation is already settled (the goal's standing decisions say
      `object` erases to the same pointer a named class does), so this is the
      "does anything below read a class label" check and nothing more.

## Backlog

- A throw escaping an abandoned generator's `finally` is dropped — surfacing it
  wants ADR 0020's escalation ladder (`mwl-ir` gap 18).
- `[context] modules` in `docs/agent/loop-goal.toml` names no `mwl-runtime`
  pattern, and two consecutive sessions have needed one.
- Two unattributed refusal sites, `crates/mwl-codegen/src/ty.rs:116` and `:121`
  — `python tools/holes.py` lists them under no item.
- Seven named `.mwlt` cases are still unwritten (`python tools/holes.py --cases`).
- `cargo-insta` is not installed on this box; snapshots are accepted by hand
  (`docs/agent/playbook.md` § *Tooling*).
