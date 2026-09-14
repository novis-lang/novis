# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6 is done: both narrowings reach the child, and all three of the
stage's checks are green.**

`limits:` and `grants:` are lowered as arguments 4 and 5 of both spawn symbols — the method form's
parameter names moved to 6 — decoded in `crates/nvs-stdlib/src/script.rs`'s `limits_of` and
`grants_of`, and carried to the seam as `nvs_runtime::host::Narrowing`. `Host::start_isolate` takes
it, `Isolate::narrowed_by` holds it, and `Isolate::start` applies it through `Ctx::narrow`
(`crates/nvs-runtime/src/ctx/isolate.rs:530`) on the child's own context before its first statement.

**Two mechanisms behind the one word "narrows".** A sub-cap goes through the child's configuration
overlay, because `nvs_config::Request::set` already refuses a value wider than the one in force —
one reader for the number a spawn writes and the number the file writes — and the ceilings it
resolves are then clamped against `remaining(…)` of the parent's, which is `placed_isolate`'s own
arithmetic. `max_script_depth` is put back after the refresh: it is copied into a child precisely so
a child cannot re-read the file and widen it. A grant list is an intersection instead, held on
`Ctx::grant_filter` as an `Arc<[Cap]>` and asked in `capability::granted` *beside* the overlay and
never instead of it, so a name the parent lacks stays lacking and a second `grants:` deeper in the
tree can only shorten the list again.

**A spawn that wrote either option does not cross to a worker core.** The seed a far core builds
from carries the ceilings and the filter a parent already holds, but a narrowing written *at this
spawn* is applied to a `Ctx` only the parent's core builds, so crossing with it would drop it —
`crates/nvs-host/src/group.rs:224` declines the crossing and `crates/nvs-host/src/placed.rs`'s
`# Known gaps` owns the reading. Closing it is the narrowing computed where `Ctx::placed_isolate`
is and carried in `PlacedIsolate`: one more field and the far side's constructor.

## Next group

**Stage 7: the narrowing crosses to a worker core** — one file set:
`crates/nvs-runtime/src/ctx/isolate.rs`, `crates/nvs-host/src/placed.rs`,
`crates/nvs-host/src/group.rs`, `crates/nvs-host/tests/`.

- [ ] **`PlacedIsolate` carries the narrowing** — `crates/nvs-runtime/src/ctx/isolate.rs:512` is
      where the seed is read off the parent and `crates/nvs-runtime/src/ctx/isolate.rs:700` where it
      is built on the far core; `Ctx::narrow` at `crates/nvs-runtime/src/ctx/isolate.rs:530` is what
      has to run on that side instead, against ceilings the parent already resolved rather than
      against a parent it cannot reach. `rule:security/isolate-budget-is-the-trees`.
- [ ] **The crossing stops being declined** — `crates/nvs-host/src/group.rs:224`'s
      `narrowing == Narrowing::default()` guard comes out and `crates/nvs-host/src/placed.rs:43`'s
      `# Known gaps` paragraph with it, once the item above holds.
      `rule:concurrency/on-worker-runs-the-child-on-another-core`.
- [ ] **A worker child is held to its own sub-cap** — a case beside
      `crates/nvs-host/src/worker.rs:984`'s
      `a_worker_child_runs_on_another_core_and_its_answer_is_copied_back`, asserting that a child
      placed `on: "worker"` with `limits:` is stopped where the same child on this core is. Which
      core ran it is not observable from source, so this half cannot be a `.nvst` case.
      `rule:errors/on-limit`.

## Backlog

- `max_tasks` and `wall_time` are accepted as sub-caps and reach the child's overlay, but nothing in
  `nvs-runtime` reads either back — `crates/nvs-runtime/src/ctx/limits.rs`'s own gap list.
- A `cpu_time` sub-cap is set on a local child's `Ctx` but the watchdog charges the tree's
  published ceiling — `crates/nvs-host/src/isolate.rs:508`.
- `docs/plan/m5.md`'s stale `callable`-in-`Task::all` sentence — goal `plan-truth`'s.
- Module-doc gaps tagged `unowned` in these files — goal `unowned-closures`'s.
