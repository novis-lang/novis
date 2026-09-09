# Handoff

## State

**Goal 39 — A record names the line it came from, and a repeat is bounded at the sink that suffers — has just started; nothing of it has landed yet.** Goal 38's whole list is this goal's Stage 1 floor.

The design is settled by [0165](../decisions/0165.md) and nothing in it is a session's to
re-decide. Four facts out of the tree, each the reason a stage exists. `nvs_render::Source` is
defined and named by the rule and **constructed nowhere** — every `Source {` in the workspace belongs
to `nvs-db`'s unrelated bind-source type. `Throwable::$location` is a declared, readable property
written as the empty string when the object is built. `Envelope.count` is filled only by
`crates/nvs-runtime/src/floor.rs`, so a request that logs in a loop is unbounded where one that
faults in a loop is not. And the floor's window is a single slot, which its own comment argues is
enough *there* because a fault loop repeats one record — an argument that does not survive
application code interleaving two messages.

Three things a session must not re-decide: the call site is a **constant read at the call**, never a
current-location word maintained in `Ctx`; **no stack is captured at any producer**, because a record
already carries `span_id` under a trace and the trace already has the call events; and the **debug
stream is not coalesced**, because its bound is an index rather than a disk and grouping at read time
is what keeps a run expandable.

## Next group

**Stage 0: how the datum reaches a producer** — read-only, and it decides stage 2's shape. File set:
`crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/emit.rs`, `crates/nvs-runtime/src/throwable.rs`.

- [ ] **Read the existing call-site constant** — `Terminator::Propagate`'s `frame: String` in
      `crates/nvs-ir/src/ir.rs`, materialised as static bytes in `crates/nvs-codegen/src/emit.rs`
      and received by `nvs_trace_push`. Once ticked, one question is answered in writing: does that
      label already carry file, line and member, or does a producer need a sibling constant?
- [ ] **Write the answer down** in the module doc that owns it, either way. A session that guesses
      here writes stage 2 twice.

Then **Stage 2: one construction, two readers** — file set
`crates/nvs-stdlib/src/{log,debug}.rs` and `crates/nvs-runtime/src/throwable.rs`, cheap to take in
the same session because stage 0 writes almost no code.

- [ ] **`crates/nvs-stdlib/src/debug.rs:@record_of`** — fill `Envelope.source`.
- [ ] **`crates/nvs-stdlib/src/log.rs`** — the same on `Core\Log::write`'s record.
- [ ] **`crates/nvs-runtime/src/throwable.rs`** — the `LOCATION_SLOT` store takes that datum instead
      of `""`.

## Backlog

- **Stage 3 — the log target's window**, file set `crates/nvs-runtime/src/floor.rs` and
  `crates/nvs-stdlib/src/log.rs`. It shares `log.rs` with stage 2, so it is cheap to take alongside
  if the session is still under the ceiling. The floor's single slot and `COALESCING_WINDOW` are left
  exactly as they are; what is new is a small fixed table for the log target.
- **Stage 4 — the rulebook**, file set `docs/rules/errors.json` plus the two fragments. It cannot run
  before stages 2 and 3 have landed the cases its `guardedBy` names.
- When this goal's last check goes green the driver takes goal 26.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
