# Handoff

## State

**The project is called Novis, and every path, crate and extension is `nvs`.**
The ten crates are `crates/nvs-*`, cases are `.nvst`, sources are `.nvs`, the
binary is `nvs`, the open tag is `<?nvs` and the env prefix is `NVS_`. In prose
the name is **Novis**; `nvs` is the token spelling and the two never swap.
`python tools/rename-project.py --audit` is the standing check, and its module
docstring is the home for why the split exists. Out of scope on purpose and
still to do by hand: the `origin` remote and the `<repo>` checkout directory.

**A tagged operand orders now, and `object` as a declared type is checked at
both ends.** `<`/`<=`/`>`/`>=`/`<=>` over a `mixed`, a union or the `int|float`
a division returns dispatch on the operands' runtime tags
(`nvs_ir::ir::Helper::ValueLt` and its two siblings, whose doc comment is that
design's home), answering ADR 0007 § 4's rows the tags name and throwing
catchably where that closed table names none. The rule's one home is ADR 0007
§ 4's closure paragraph, which now carries the deferral, the object row and the
enum row.

- **Item 25 needed no code**: `erase_checked_ty` already had its `object` arm
  and `object-is-a-declared-type-in-every-position.nvst` already ran, so what
  the slice owed was the descriptor check. Both fields a release or a rendering
  reads are found from the instance, and the `unwind` half is now pinned —
  `an-abandoned-generator-runs-the-finally-it-is-suspended-inside.nvst` grows a
  seventh shape where the last reference goes away through an `object` binding.
- **`emit_binop`'s catch-all has one target left**, a `Ty::Tagged` under an
  arithmetic operator, and its roster comment says so. `nvs-ir`'s known gap 3
  was two thirds stale and is corrected: ADR 0035's truthy table and the tagged
  subscript both left it long ago, the subscript as `E0482` rather than as a
  lowering.
- **`orient.py`'s pack was complete for both items.** The two standing gaps are
  unchanged: `[context] modules` has no `nvs-runtime` entry — this session read
  `crates/nvs-runtime/src/helpers.rs` and `identity.rs` to write the helper —
  and none for `nvs-diagnostics`.

## Next group

**Item 24's other half, over the file set this session already opened.** The
files: `crates/nvs-ir/src/lower/operator.rs`, `crates/nvs-ir/src/ir.rs`,
`crates/nvs-runtime/src/helpers.rs`, `crates/nvs-codegen/src/emit.rs`.
`python tools/holes.py --item N` prints any of these in full.

- [ ] **Item 24's second half, arithmetic on a tagged operand** — the one
      target `emit_binop`'s catch-all has left
      (`crates/nvs-codegen/src/emit.rs:1151`). The shape is this session's:
      a `Helper` per operator dispatching on the two tags, emitted from
      `lower_binary`'s tagged arm at
      `crates/nvs-ir/src/lower/operator.rs:567`, beside
      `crates/nvs-runtime/src/helpers.rs:446` (`value_ordering`), which is the
      model for the tag table and for the throw a pair with no row takes.
      Unlike ordering, § 4's arithmetic rows **throw on overflow** and refuse
      `int ⊕ uint` outright, so the helper owes two error kinds rather than
      one, and `nvs_runtime::arith` already holds both.
- [ ] **`emit.rs`'s six remaining internal panics** — `reinterpret`, the tagged
      widen/narrow pair, the unary catch-all, the refcount one, the terminator
      one and the runtime-helper one. Each is an internal-consistency check
      rather than a hole; what a session owes is the roster comment, in the
      shape `emit_binop`'s now has. `crates/nvs-codegen/src/emit.rs:660`,
      `:690`, `:721`, `:1795`, `:2787`, `:2938`, `:3298`.
- [ ] **Item 26, `bool as int` and `bool as string`** —
      `crates/nvs-ir/src/lower/convert.rs:60`. The playbook records that
      `bool as string` renders `false` as the empty string and that
      `bool as int` does not lower at all; ADR 0007 § 2's grid is what decides
      whether either is a row.

## Backlog

- A `mixed` against a `decimal` still takes `lower_decimal_binary` rather than
  the tagged arm, so a non-numeric tag there answers `false` instead of
  throwing — `crates/nvs-ir/src/lower/operator.rs`, the `decimal` guard that
  runs first.
- Ordering two objects behind two `mixed`s throws; dispatching
  `Comparable::compareTo` from the runtime would be `nvs_runtime::stringify`'s
  shape one method further — ADR 0013, and not this milestone's.
- `docs/agent/loop-goal.toml`'s `[context] modules` has no `nvs-runtime` or
  `nvs-diagnostics` entry.
- `array<T> as array<U>` inside a `.nvst` case still has the shapes the
  playbook names — `docs/agent/playbook.md`.
- `tools/holes.py`'s item 25 anchor points at `lower/mod.rs:2206`, which is
  `aliasing_read` rather than the erasure map — `docs/agent/loop-goal.md`.
