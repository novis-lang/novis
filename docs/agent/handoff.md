# Handoff

## State

**Goal `markup-literal`, stage 4: the fold's floor is on disk, the fold itself is not.** A hole-free
`` html`…` `` still lowers to `ConstStr` + `CoreCall{CORE_HTML_MARKUP}` and allocates one carrier per
execution, which is the one stage-4 check still red.

What the floor is. An object header can carry `nvs_runtime::IMMORTAL_REFCOUNT`, at one compare and a
not-taken branch in `bump` and `drop_one` — the same price `crate::string` already pays, and
load-bearing rather than an optimization, because a compiled unit is shared across cores and the word
must never be *written*. `nvs_runtime::object`'s § *Decision: an immortal instance is on no list at
all* is that decision's home and states what it spends; `immortal_object_bytes` and `OBJ_ALIGN` are
what `nvs-codegen` writes such a constant with, leaving the `class` word and each slot payload zero
for the emitting side to relocate.

The blocker that shaped it, and it is settled: a `Core` class's `ClassDesc` was built and leaked once
**per core**, so no one address existed for a shared unit to bake into a data section. That table is
now one per process (`nvs_stdlib::instance` § *Decision: the descriptors are one leaked table for the
process*), which `ClassTable` was already `Send + Sync` for, and which also spends less memory than it
did. `nvs-codegen` already reaches `nvs_stdlib` — `crates/nvs-codegen/src/lib.rs:1656` feeds
`nvs_stdlib::symbols()` to `builder.symbol` — so the descriptor address has a route to a relocation.

## Next group

**Stage 4: the hole-free fold** — one file set: `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-codegen/src/emit.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/expr.rs` and
`crates/nvs-stdlib/src/instance.rs`.

- [ ] **The unit publishes `Core\Html\Markup`'s descriptor** — `crates/nvs-codegen/src/lib.rs:1656`
      is the JIT path where `nvs_stdlib::symbols()` already feeds `builder.symbol`, and
      `crates/nvs-codegen/src/lib.rs:1769` is the object path's twin; both need
      `class_desc_symbol(nvs_runtime::CARRIER_HTML_MARKUP)` bound to the address a new `pub fn` beside
      `crates/nvs-stdlib/src/instance.rs:301` hands out. `rule:core-classes/html-literal`. Note that
      `Classes::desc` answers `None` for it, so `emit`'s `class_desc_const` is the wrong door — the
      symbol is an import this unit never defines.
- [ ] **The instruction that names one** — `crates/nvs-ir/src/ir.rs:604`: an `InstKind::ConstMarkup`
      carrying the joined bytes, `Ty::Object`, sited beside `ConstStr`. A const instruction has exactly
      three homes, which `ConstBytes` is the map of: the enum here, `crates/nvs-ir/src/print.rs:147`,
      and `crates/nvs-codegen/src/emit.rs:560`.
- [ ] **The data object with two relocations** — `crates/nvs-codegen/src/emit.rs:1152`, beside
      `emit_immortal_str`: `nvs_runtime::immortal_object_bytes(&[Tag::Str])` at `OBJ_ALIGN`, then
      `DataDescription::declare_data_in_data` + `write_data_addr` at `nvs_runtime::OBJ_CLASS_OFFSET`
      and at `field_offset(0) + Value::BITS_OFFSET` — the second pointing at the immortal `StrHeader`
      `emit_immortal_str` already builds, which wants a sibling returning its `DataId` rather than a
      materialized address. `crates/nvs-runtime/src/object.rs:3799`'s `immortal_unit` is the same two
      words filled in by hand, and is what the emitted bytes must match.
- [ ] **The fold at the arm that already exists** — `crates/nvs-ir/src/lower/expr.rs:2196`: a `parts`
      holding no `StringPart::Expr` cooks its segments at compile time and emits the one instruction
      instead of the `ConstStr`/`Concat`/`CoreCall` run, with no temporary staged and no release. The
      two red tests go in `crates/nvs-codegen/src/lib.rs:2282`, whose `lower` and `compiled` helpers
      are what they assert on.

## Backlog

- Identical literals each mint their own data object — `nvs-codegen`'s own known gaps.
- `Core\Cli\Text` gets no literal form by decision — `rule:tooling/styling-is-a-value-not-a-grammar`.
- `nvs fmt`, the LSP region and `nvs convert` are rules with no tool to edit — the goal's § *Standing
  decisions*.
