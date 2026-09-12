# Handoff

## State

**Goal `markup-literal`, stage 4 is on disk and both its checks' tests pass.** A hole-free
`` html`…` `` is now constant-pool data: `nvs_ir::ir::InstKind::ConstMarkup` carries the cooked
bytes, and `nvs-codegen` writes the whole `Core\Html\Markup` into the unit's own data section — the
header from `nvs_runtime::immortal_object_bytes`, the class word and the one text slot as two
relocations — so it costs one address and no call per execution. A literal with holes is untouched:
its bytes are not known until it runs, so it stays the join and the lift.

The descriptor route that made the fold possible. `nvs_stdlib::class_descriptors` publishes the one
address the process leaked for a `Core` class with instances (`nvs_stdlib::instance` § *Decision:
the descriptors are one leaked table for the process*), and `nvs-codegen`'s `core_desc_symbols`
mints `class_desc_symbol`'s name for it. Both ends that resolve a descriptor read it: the JIT's own
symbol table, and `Descriptors::resolve`, which is what a warm cache hit relocates a stored payload
against. It could not come from `Classes`, which holds only the classes a unit declares.

Nothing is blocked, and `python tools/verify.py` is green across the tree.

## Next group

**Stage 5: `Core\Html::join`** — one file set: `crates/nvs-stdlib/src/html.rs`, with
`crates/nvs-stdlib/src/registry.rs` for the one spelling question.

- [ ] **What spells `array<Core\Html\Markup>` in a row** — `crates/nvs-stdlib/src/html.rs:160`
      shows `CoreTy::Instance` already works as a parameter, so the open half is the *list*: read
      the `CoreTy` roster at `crates/nvs-stdlib/src/registry.rs:225` for an array-of-instance arm
      before writing the signature, and add one there if it has none.
      `rule:core-classes/html-literal`.
- [ ] **`join`'s row and its helper** — a `CoreMethod` beside `toSource`'s at
      `crates/nvs-stdlib/src/html.rs:157`, its `MethodDoc` beside
      `crates/nvs-stdlib/src/html.rs:301`, and the helper beside the `nvs_helper!` at
      `crates/nvs-stdlib/src/html.rs:645`. It escapes nothing and trusts nothing: every element is
      already a carrier, so it is a walk of the list and the separator between.
      `rule:core-classes/html-literal`.
- [ ] **The three tests the check names** — in `mod tests` at
      `crates/nvs-stdlib/src/html.rs:1633`: `html_join_writes_every_part_in_order_with_its_separator_between`,
      `html_join_over_an_empty_list_answers_an_empty_markup`, and
      `html_join_escapes_nothing_because_every_part_is_already_a_carrier`.

## Backlog

- Stage 5's second check is seven `.nvst` documents, none of them on disk yet —
  `docs/agent/loop-goal.toml:8801`.
- `crates/nvs-stdlib/src/response.rs:12`'s known gap says a carrier is not spellable in a registry
  row, but `toSource` spells one at `crates/nvs-stdlib/src/html.rs:160` — that note is stale or means
  something narrower than it says.
- Two identical string literals are still two data objects, and a folded markup constant now makes
  two of them at a time — `nvs-codegen`'s module doc, known gap 4.
