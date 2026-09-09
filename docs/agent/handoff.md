# Handoff

## State

**Goal 39 — a record names the line it came from, and a repeat is bounded at the sink that suffers.
Stage 0 is closed; nothing of stages 1–3 has landed.** Goal 38's whole list is still this goal's
Stage 1 floor. The design is settled by [0165](../decisions/0165.md) and its § *Standing decisions*
are not a session's to re-open.

Stage 0's question is answered, and the answer is on disk in the one doc that owns it —
`Lowering::frame_label` (`crates/nvs-ir/src/lower/mod.rs:2127`). The existing call-site constant
**does** carry file, one-based line and `Class::method`, but pre-rendered into one `String` and
built only where a landing block is. So `rule:errors/a-record-names-where-it-was-produced`'s datum
reuses that *derivation* — `fn_label`, the source name and `cur_stmt_span`, no second position
table — and never the string itself. Three reasons, spelled out in that doc comment: the string is
presentation, which `rule:errors/diagnostic-record`'s envelope does not carry; its leading half is
a *frame* name, `script` or `file#<id>$script`, exactly where `Source::member` is `None`; and it
reaches compiled code through `Terminator::Propagate` alone, so a producer on a statement needing
no error path has no constant there at all.

That resolves the goal's standing "reuse it / add a sibling" branch to both halves at once: one
derivation, and a constant emitted at the producer's own call the way
`crates/nvs-codegen/src/emit.rs:3381` already does with `emit_bytes`. The only other per-statement
mechanism in codegen, `emit_stmt_probe` (`crates/nvs-codegen/src/emit.rs:1040`), sits behind the
debug-flags branch and is not a standing current location — reaching for it would build the `Ctx`
word 0165 refuses.

## Next group

**Stage 2: one derivation, and the constant at the producer's call** — one file set:
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/emit.rs`.

- [ ] **A sibling of `frame_label` returns the three parts** — `crates/nvs-ir/src/lower/mod.rs:2127`
      — in `nvs_render::Source`'s `file`/`line`/`member` shape
      (`crates/nvs-render/src/lib.rs:414`), with `frame_label` rendering *from* it so one
      derivation still feeds both readers. `member` is `None` for the two script spellings at
      `crates/nvs-ir/src/lower/mod.rs:517`. `rule:errors/a-record-names-where-it-was-produced`.
- [ ] **Decide the IR carrier, then add it** — `crates/nvs-ir/src/ir.rs:490`'s `InstKind::Call` is
      the variant a producer's call lowers to and it carries no source today;
      `Terminator::Propagate` (`crates/nvs-ir/src/ir.rs:2533`) is the shape that already works,
      a plain field on the node that codegen materialises.
      `rule:errors/a-record-names-where-it-was-produced`.
- [ ] **Emit it** — `crates/nvs-codegen/src/emit.rs:3381` is the pattern to copy, and `emit_bytes`
      (`crates/nvs-codegen/src/emit.rs:1108`) is the primitive: bytes into the data section,
      address and length into the call. `rule:errors/propagation` fixes the signature any extra
      operand has to live beside.

## Backlog

- `crates/nvs-stdlib/src/debug.rs:220` — `record_of` fills `Envelope.source`; blocked on the
  carrier above. (goal 39 stage 3)
- `crates/nvs-stdlib/src/log.rs` — the same on `Core\Log::write`'s record. (goal 39 stage 3)
- `crates/nvs-runtime/src/throwable.rs:56` — `LOCATION_SLOT` stores that datum instead of the
  empty string. (goal 39 stage 3)
- The `count` half: the log target's small fixed window per
  `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`; `crates/nvs-runtime/src/floor.rs`
  holds the single-slot precedent the rule says is too narrow here. (goal 39, later stage)
- `[context] modules` did not name `crates/nvs-ir/src/lower/**`, where stage 0's whole answer and
  all of stage 2's first slice live; this session's commit touches it, so the driver's sweep should
  close it — check the next pack prints it.
