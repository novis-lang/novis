# Handoff

## State

**Stage 6's item 20 is closed: both of ADR 0006's constructs lower, and `native examples/isolate.nvs
[6 isolates]` prints all five frozen lines.** `spawn script` and `await` are two
`InstKind::CoreCall`s against `nvs_stdlib::script`'s two rowless symbols; the handle is a
`Core\Script\Handle` whose one slot holds the key `Ctx::hold_started_script` filed the running
isolate under; `await` answers ADR 0036's shape with one field per `Completion` member. `E0703` and
`E0776` are retired, `E0777` is new for the three options this compiler parses and does not enforce.

**The seam is a start and a join now, and that is the load-bearing change.**
`nvs_runtime::host::Host::start_isolate` answers a `Box<dyn Running>`; `nvs_host::Isolate::run` is
`start` then `join`, so every existing test over it is untouched. `Output`, `Failure` and
`Completion` moved down to `nvs_runtime::host` (the seam names them) and `nvs_host::isolate`
re-exports them. Each module doc is the one home of its own half: why the start is eager
(`nvs_runtime::host::Host::start_isolate`), why the handle holds a key rather than a table entry
(`Ctx::hold_started_script`), and why the two symbols have no rows (`nvs_stdlib::script`).

**Orientation gaps, unchanged from last session and still real:** `[context] adrs` prints ADR 0023
§ 2 and ADR 0072 §§ 4-5 only — ADR 0006's `## Decision` is what a Stage 6 slice is written against
and had to be sliced by hand. `[context] modules` has no pattern for `nvs-types/src/expr/`,
`nvs-stdlib/src/instance.rs`, `nvs-stdlib/src/registry.rs` or `nvs-ir/src/lower/`.

## Next group

**The four Stage 6 `cargo-named` tests with no function on disk.** They share one file:
`crates/nvs-host/src/isolate.rs`, whose test module opens at `:420` and whose four existing siblings
(`:494`, `:641`) are the shapes to copy. `crates/nvs-host/src/scheduler.rs:1102` is `cancel_task`,
which the last two need.

- [ ] **`an_unresolvable_class_is_refused_at_the_boundary`** — ADR 0023 § 2's third bullet. A child
      returning an object whose class the parent's table does not have is `ok = false` carrying the
      walk's message, not a stub; `crates/nvs-runtime/src/graph.rs` is where the refusal is raised
      and `a_closure_a_reference_or_a_resource_is_refused_at_the_boundary` is the sibling to copy.
- [ ] **`a_contained_panic_in_a_child_leaves_the_parent_running`** — ADR 0106 § 2. The child's task
      is torn down and the parent reads `ok = false`; `nvs_runtime::Teardown` is the containment
      boundary already on disk.
- [ ] **`a_cancelled_parent_leaves_no_orphan_and_no_leaked_arena`** — ADR 0072 § 5, over
      `Started::join`'s park loop in `crates/nvs-host/src/isolate.rs`, which cancels the child and
      keeps parking until it has ended.
- [ ] **`a_child_is_cancelled_at_its_next_safepoint`** — item 24's first consumer of the
      function-entry safepoint. `Ctx::cancel` and `SafepointFlags::CANCEL` are the mechanism.

## Backlog

- Item 22 — `Core\Script::args()` and `Core\Script::valueOrThrow($result)`, plus `ScriptResult`'s
  `code`/`trace` fields (`crates/nvs-types/src/expr/isolate.rs`'s module doc).
- `tests/conformance/isolate/` holds two cases now; the two `docs/agent/loop-goal.toml` names at
  Stage 8 (`a-child-shares-nothing-with-its-parent`, `a-childs-failure-is-a-value-not-an-exception`)
  are still unwritten.
- `output: capture|inherit`'s carrier type is still `string` — item 24 owns it (ADR 0088 §§ 3, 5).
- `spawn`/`spawn worker` still do not exist; only `spawn script` does (ADR 0006 § *Decision*).
- `limits:`, `grants:` and `on:` are `E0777` until goal 3 enforces them.
- No `.nvst` case pins an argument that cannot cross (`GraphError` at the spawn) — the three other
  sites sharing that stem are covered, so the gate is green and the hole is real.
