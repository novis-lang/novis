# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6: the placement crosses; the two narrowing options do not yet.**

`on:` is end to end. `tests/conformance/isolate/a-child-on-a-worker-core-answers-as-one-on-this-core-does.nvst`
is the stage's first `.nvst` case and it passes: the **method** form spawned `on: "worker"` and the same
one spawned `on: "here"` agree on `ok`, on the returned value, on the captured output and on `error`,
and neither reached the parent's statics. It is the agreement shape `conventions.md` § *A `.nvst` test
case* names, and it is all a conformance case can assert — **which** core ran the child is not
observable from source, and `crates/nvs-host/tests`' `a_worker_child_runs_on_another_core_and_its_answer_is_copied_back`
is what pins that half.

**`limits:` and `grants:` are typed and then dropped.** `crates/nvs-ir/src/lower/expr.rs:3169-3241`
reads `SpawnOptionKey::Args`, `Output` and `On` and nothing else, so the spawn `CoreCall` carries four
values and `nvs_core_script_spawn` (`crates/nvs-stdlib/src/script.rs:649`) takes `args: [4]`. A program
that writes `grants: ["net.connect"]` therefore gets a child holding the parent's whole overlay — a
narrowing silently not applied, which is what
`rule:concurrency/an-upgrades-options-are-spawn-scripts` refuses options by name to avoid, and
priority 1 work rather than tidying. The two remaining `.nvst` cases of this stage assert those
narrowings, so they cannot be written before the next group lands.

The seam is `Host::start_isolate` (`crates/nvs-runtime/src/host.rs:661`), which takes the entry, the
argument, the output and the placement; `Isolate` (`crates/nvs-host/src/isolate.rs:106`) is what it
builds, and `Isolate::start` (`crates/nvs-host/src/isolate.rs:444`) is where a ceiling is already
asked about — `ctx.script_depth_breach()` — one line before anything is built. The child's own
ceilings are plain `Ctx` fields (`memory_limit`, `output_limit`, `cpu_limit`, `deadline`,
`max_script_depth`), copied in `Ctx::placed_isolate`
(`crates/nvs-runtime/src/ctx/isolate.rs:512`) and defaulted in `Ctx::isolate`
(`crates/nvs-runtime/src/ctx/isolate.rs:364`). The sub-cap names are
`SUB_CAP_SETTINGS`/`SUB_CAP_COUNTS` (`crates/nvs-types/src/expr/isolate.rs:412`): `cpu_time`,
`max_output`, `memory`, `wall_time`, `max_tasks`.

## Next group

**Stage 6: `limits:` and `grants:` reach the child** — one file set:
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-stdlib/src/script.rs`, `crates/nvs-runtime/src/host.rs`,
`crates/nvs-host/src/isolate.rs`.

- [ ] **The two options are lowered and decoded** — `crates/nvs-ir/src/lower/expr.rs:3206` is where
      `on:` is built and where the two arms go beside it; `crates/nvs-stdlib/src/script.rs:649` is the
      helper that grows from `args: [4]`, beside `placement_of` at
      `crates/nvs-stdlib/src/script.rs:603`. Both spawn symbols take them, and the method form's
      parameter-names argument stays last. `rule:security/isolate-budget-is-the-trees`.
- [ ] **`limits:` narrows the child's ceilings and never widens them** — carried through
      `crates/nvs-runtime/src/host.rs:661` into `crates/nvs-host/src/isolate.rs:106` and applied in
      `crates/nvs-host/src/isolate.rs:444`, against what remained of the tree rather than against the
      configured number. `rule:security/isolate-budget-is-the-trees`, `rule:errors/on-limit` for what
      a breach is.
- [ ] **`grants:` intersects the child's overlay** — the child's configuration is cloned whole at
      `crates/nvs-runtime/src/ctx/isolate.rs:364`, so the narrowing is applied there; a grant is a list
      and not a quantity, so it is an intersection with what the parent holds and a name the parent
      lacks stays lacking. `rule:security/isolate-shares-nothing`,
      `rule:security/capability-question-is-grant-and-scope`.
- [ ] **A bounded spawn still crosses, or stays here on purpose** — `crosses`
      (`crates/nvs-host/src/placed.rs:94`) decides what a far core can prepare, and the seed is
      `crates/nvs-runtime/src/ctx/isolate.rs:619`. Either the sub-caps and grants cross in it, or
      `crosses` answers `false` for a spawn that carries them and `placed.rs`'s `# Known gaps` says so
      — a narrowing dropped on the way to another core is the hole the group exists to close.
      `rule:concurrency/on-worker-runs-the-child-on-another-core`.
- [ ] **The two remaining `.nvst` cases** —
      `tests/conformance/isolate/a-child-given-limits-is-stopped-at-its-own-ceiling.nvst` and
      `tests/conformance/isolate/a-child-given-grants-holds-only-those-and-cannot-widen-them.nvst`,
      the stage's `nvs-suite` check's other two rows. The landed case at
      `tests/conformance/isolate/a-child-on-a-worker-core-answers-as-one-on-this-core-does.nvst:1` is
      the shape to follow.

## Backlog

- `Core\Queue`'s and `Core\Socket`'s `limits:`/`grants:` are refused by name until an isolate enforces
  them (`crates/nvs-stdlib/src/queue.rs:5718`) — that refusal can be lifted once the group above lands.
- Which core ran a child is unobservable from source; if a conformance case ever needs it, it is a
  debug probe (`rule:testing/debug-probes`), not a `Core` member.
