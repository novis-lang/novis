# Handoff

## State

**Goal 2's Stage 5 is on disk from the walk outwards, and `examples/serialize.nvs` prints
its four frozen lines.** The acceptance check that had been failing (`Core\Serialize` has no
member named `encode`) is closed: ADR 0023 § 2's graph copy exists once, as
`crates/nvs-runtime/src/graph.rs`, and `Core\Serialize::encode`/`decode` are the member pair
that reach it. All five of Stage 5's `nvs-runtime` names are green.

**The walk's three decisions live in `graph.rs`'s module doc** and nowhere else: identity is
*object* identity only (a string is immutable and an array is copy-on-write, so sharing is
unobservable in both); a move at refcount 1 is decided **per node** rather than at the root;
and an object's declared property names are written *before* any of their values, which is
what makes § 3's shape check a refusal rather than a half-filled instance. The `Carrier`
trait is private and has exactly two implementors — that is the item, not an implementation
detail.

**Two seams were added.** `nvs_runtime::Ctx::class_desc(name)` is how a `Core` member reaches
a class the *program* declared, reading the table `set_runtime_error_class` already installed
rather than a second registration. And `graph.rs` refuses a `secret` property on the same
pass as a closure, which is item 18's runtime half; its *compile-time* half is still open.

**What is left of Stage 5**: the two `nvs-types` names in `loop-goal.toml` —
`serialize_decode_refuses_a_tainted_operand` and
`a_secret_value_is_refused_at_the_boundary_unless_revealed` — and joining the live carrier to
the `spawn` boundary, which has nothing to join to until Stage 6 builds one.

**Orientation gaps.** `[context] adrs` gained no new entry this session but wanted one: ADR
0023 § 3 was read by hand (`peek.py` on the heading) because the manifest names § 2 only.
`[context] modules` still has no pattern for `nvs-stdlib/src/instance.rs` or
`nvs-runtime/src/object.rs`, and the second is where `ClassDesc`, `ClassTable` and
`CodecField` all live.

## Next group

**Stage 5's checker half.** File set: `crates/nvs-types/` (the `Core` signature lowering and
its tests), against `crates/nvs-stdlib/src/serialize.rs:57` (the `CoreTy::Blob(Qual::Sink)`
row this group tests) and `crates/nvs-runtime/src/graph.rs:284` (the `field_is_secret`
refusal, which is the runtime half already landed).

- [ ] **`serialize_decode_refuses_a_tainted_operand`**, in `nvs-types`. ADR 0023 § 3's last
      bullet and ADR 0088 § 1: the row is already `Qual::Sink`, so this is a test over
      existing machinery plus whatever `Core\Json::decode`'s own sink test does not cover.
      `crates/nvs-stdlib/src/registry.rs:115` is `Qual::Sink`'s definition.
- [ ] **`a_secret_value_is_refused_at_the_boundary_unless_revealed`**, in `nvs-types`. ADR
      0033, and the *compile-time* half of what `graph.rs:284` already refuses at runtime.
      Decide at the checker whether a `secret`-typed argument to `Core\Serialize::encode` is
      an `E07xx` (next free is E0775) or reuses the qualifier machinery `Qual::Sink` has.
- [ ] **A `.nvst` case for the `secret` refusal**, under `tests/conformance/reject/`, whose
      `--EXPECTF-ERROR--` freezes whichever diagnostic the slice above chose.

## Backlog

- ADR 0072 § 4 row 3 — two children throw, the second is written and never swallowed —
  still has no case; `crates/nvs-host/src/group.rs:279` and `:454` are the two
  `write_diagnostic` calls, and `docs/adr/0072-core-task-structured-concurrency.md` § 4 owns it.
- A `limit` asserted on both sides, and `Core\Task::all` over a field that throws: ADR 0072
  §§ 3 and 1, against `examples/tasks.nvs`.
- `Core\Serialize`'s two names are not in the migration inventory yet —
  `python tools/check-migration.py --report`, and goal 2's Stage 7 owns the rows.
- A `send` on a closed `Core\Task\Channel` is a `Fault::fatal` rather than a throw;
  `crates/nvs-stdlib/src/channel.rs`'s module doc owns why, ADR 0020's ladder owns the fix.
- Stage 6's isolate is the live carrier's only consumer — `docs/agent/goals/2-concurrency.md`
  item 20.
- `graph.rs`'s known gap 2: an encoded `Core` instance is refused as unresolvable, because
  the resolver asks the program's table only.
