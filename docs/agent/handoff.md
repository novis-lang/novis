# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6: the worker crossing is blocked on the budget, and the finding is
recorded where it belongs.**

`nvs_runtime::budget`'s two counters are **thread-local** (`crates/nvs-runtime/src/budget.rs:29-37`):
a request is charged the difference between its thread's balance now and the balance when its `Ctx`
was made, which `crates/nvs-runtime/src/ctx/isolate.rs:341-346` states as the whole of why no ceiling
crosses to a child. A child started on another core takes its zero point there and is bounded by that
core's reading, so it holds a `[limits] memory` of its own — one per core, for one request. That is
the arithmetic `rule:security/isolate-budget-is-the-trees` bounds, ADR 0184 § 4 asserts the opposite,
and priority 1 is not traded: **the arm does not post until the tree's counters are reachable from a
thread that did not open them.** Both placement words still start the child here
(`crates/nvs-host/src/group.rs:266`), and that module's `# Known gaps` is the one home of why.

What landed instead is the one piece of the arm that is correct on its own.
`crates/nvs-host/src/worker.rs:748`'s `destination()` secures a core **before** the work exists, and
`Destination::post` (`crates/nvs-host/src/worker.rs:726`) cannot fail; `post`/`post_on` are those two
back to back. A spawn needs that order because `nvs_runtime::graph::encode`
(`crates/nvs-runtime/src/graph.rs:719`) consumes the argument's reference, so an arm that encoded
first and was then refused a core would hold neither a placement nor a value to start here.

**Also settled while reading.** The child's configuration *can* cross: `nvs_config::Request`
(`crates/nvs-config/src/request.rs:51`) is `Clone` over an `Arc<Snapshot>` plus a `BTreeMap`, and
`Ctx::set_config` + `Ctx::config_mut` (`crates/nvs-runtime/src/ctx/wiring.rs:350-366`) put one back,
so `script.spawn` is asked on the far core against the parent's own overlay. A `Resolver` cannot:
`crates/nvs-runtime/src/script.rs:229-232` says it holds the unit cache and is `!Sync`.

## Next group

**Stage 6: the tree's budget across a core, then the arm** — one file set:
`crates/nvs-runtime/src/budget.rs`, `crates/nvs-runtime/src/ctx/isolate.rs`,
`crates/nvs-host/src/group.rs`, `crates/nvs-host/src/worker.rs`. **The first item gates the other
two**: an arm that posts before the budget crosses hands one request a fresh cap per core.

- [ ] **A tree's memory and output counters become readable from another thread** — the thread-local
      pair is `crates/nvs-runtime/src/budget.rs:29`, and the sentence that says no ceiling crosses is
      `crates/nvs-runtime/src/ctx/isolate.rs:341`. A root takes an atomic pair **only** when a child
      is first placed off its core, so an unplaced request pays one branch and no atomic;
      `Ctx::memory_used` then reads its thread-local balance plus that pair.
      `rule:security/isolate-budget-is-the-trees` is what this has to satisfy, and ADR 0184 § 4 is
      the assertion it makes true.
- [ ] **The `Placement::Worker` arm secures a core, encodes the argument and posts** — the one-arm
      match is `crates/nvs-host/src/group.rs:266`. `crate::worker::destination`
      (`crates/nvs-host/src/worker.rs:748`) first, `nvs_runtime::graph::encode`
      (`crates/nvs-runtime/src/graph.rs:719`) second, `StartError::Argument`
      (`crates/nvs-runtime/src/host.rs:311`) on a refusal, and a fall back to starting here when
      there is no core. The `Entry` and a cloned `nvs_config::Request`
      (`crates/nvs-config/src/request.rs:51`) travel with the bytes.
      `rule:security/isolate-values-cross-by-copy` minus the move (ADR 0184 § 2).
- [ ] **The far core answers a `Send` completion, decoded into the parent's arena at the join** —
      `Completion` is `crates/nvs-runtime/src/host.rs:366` and its `value` is a `Value`, so it
      crosses as `graph::encode` bytes and the parent's `Running::join` decodes them. A refusal on
      the way back is `ok = false` with the walk's message, which is
      `crates/nvs-host/src/isolate.rs:39-51`'s decision unchanged.

## Backlog

- Each worker core needs a `Resolver` of its own for the path form; one is `!Sync`
  (`crates/nvs-runtime/src/script.rs:229`), so the embedder registers a `Send + Sync` factory and
  `run_core` (`crates/nvs-host/src/worker.rs:833`) installs the result with `script::scoped`.
- `Entry::Method` on a worker placement needs the parent's class table, which is `Rc`-shared and does
  not cross — `crates/nvs-host/src/group.rs`'s `# Known gaps`.
- A serving core does not register its own inbox, so `nvs serve` places on a lazily started core —
  ADR 0184 § 5's fallback, `crates/nvs-host/src/worker.rs`'s `# Known gaps`.
- Stage 6 still owes the `nvs-types` options check and its three `.nvst` cases.
- `limits:` and `grants:` still stop at the checker —
  `rule:concurrency/an-upgrades-options-are-spawn-scripts`.
