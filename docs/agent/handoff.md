# Handoff

## State

**Goal 39 — stage 0 is closed and stage 2's first slice is on disk; nothing else of stages 1–3 has
landed.** Goal 38's whole list is still this goal's Stage 1 floor. The design is settled by
[0165](../decisions/0165.md) and its § *Standing decisions* are not a session's to re-open.

`Lowering::source` (`crates/nvs-ir/src/lower/mod.rs:2170`) is the one derivation of
`rule:errors/a-record-names-where-it-was-produced`'s datum — `nvs_render::Source`'s file, one-based
line and `Option<Class::member>` — and `frame_label` (`crates/nvs-ir/src/lower/mod.rs:2152`) now
renders its backtrace string from it rather than deriving a position of its own. `member` is
`Some(fn_label)` for every frame but the script one, closures and generators included, because their
labels are `Class::member` shaped too. `nvs-ir` depends on `nvs-render` directly now.

**Stage 2's remaining anchor was wrong, and the corrected one is below.** A producer's call is
`InstKind::CoreCall`, not `InstKind::Call`, and it goes through `rule:errors/propagation`'s fixed
`(ctx, args, out) -> i32` signature — so no extra machine operand is available at one, and the
constant has to arrive as one more `Value` in `args`. That is a solved problem in this tree, not a
new one: `Core\Json::decodeAs` already takes three compiler-supplied operands ahead of its declared
parameters.

## Next group

**Stage 2: the carrier, and the constant at the producer's call** — one file set:
`crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-codegen/src/emit.rs`,
`crates/nvs-stdlib/src/debug.rs`, `crates/nvs-stdlib/src/log.rs`.

- [ ] **Add the carrier as a `ShapeCodecConst` sibling** — `crates/nvs-ir/src/ir.rs:553` is the
      variant to copy, holding the `Source` `crates/nvs-ir/src/lower/mod.rs:2170` returns; codegen
      bakes it with `emit_bytes` (`crates/nvs-codegen/src/emit.rs:1108`) and materialises one
      address the way `crates/nvs-codegen/src/emit.rs:2243` does.
      `rule:errors/a-record-names-where-it-was-produced`.
- [ ] **Append it at the producer's own call, and decode it there** — one slice, because the arity
      moves on both sides at once: lowering appends the operand as
      `crates/nvs-ir/src/lower/mod.rs:2001` appends a codec, and `nvs_core_debug_dump`
      (`crates/nvs-stdlib/src/debug.rs:174`), `nvs_core_debug_render`
      (`crates/nvs-stdlib/src/debug.rs:200`) and `nvs_core_log_write`
      (`crates/nvs-stdlib/src/log.rs:214`) read it off `args` the way
      `crates/nvs-stdlib/src/json.rs:1128` reads its codec — a `Tag::Null` payload carrying an
      address, decoded by `Value::as_shape_codec` (`crates/nvs-runtime/src/value.rs:588`), which is
      the shape a `*const` source constant travels by. Closes the stage's first check.
- [ ] **A throw's location is that same datum** — `crates/nvs-runtime/src/throwable.rs:347`'s
      `new_as` stores the empty string into `LOCATION_SLOT`
      (`crates/nvs-runtime/src/throwable.rs:56`); it takes the constant the raising site carries
      instead. Closes the stage's second check.
      `rule:errors/a-record-names-where-it-was-produced`.

## Backlog

- Stage 1's floor is goal 38's whole list, still unlanded — `docs/agent/loop-goal.toml` stage 1.
- Stage 3 bounds a repeat at the log sink only — `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.
- The stage's conformance cases under `tests/conformance/` are unwritten — stage 2's third check.
- `Core\Debug::render` may not want the operand at all if it renders a record it was handed rather
  than producing one — decide it with the slice above, `rule:errors/debug-dump`.
