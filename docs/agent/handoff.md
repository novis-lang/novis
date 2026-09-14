# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6's first item is on disk: the host seam names the child's program
instead of carrying it.**

`nvs_runtime::host::Entry` (`crates/nvs-runtime/src/host.rs:243`) is now the name and not the code —
`Path(String)`, or `Method { label, names }` — so it is `Send`, and `Entry::program(ctx)` turns it
into a `Program` on the core that is about to run the child, asking
`rule:security/capability-check-at-the-door`'s `script.spawn` door there. `Host::start_isolate` lost
its `program` parameter and answers `StartError`, whose two variants are the two questions asked
before a child exists. The method form's closure moved to `nvs_runtime::script::method_program`
(`crates/nvs-runtime/src/script.rs:322`) with `bound_arguments` beside it, which is now that
function's one home; `nvs_host::Isolate` keeps a private `Form` tag for the one thing the name still
decides on its side, which context constructor arms the child.

**Still routed here.** `SchedulerHost::start_isolate` (`crates/nvs-host/src/group.rs:205`) sends both
placement words to this core's own start, and `crates/nvs-host/src/group.rs`'s `# Known gaps` is that
gap's one home: what is left is the route, not the shape. `limits:` and `grants:` still stop at the
checker.

## Next group

**Stage 6: the options, routing a worker placement** — one file set:
`crates/nvs-host/src/worker.rs`, `crates/nvs-host/src/group.rs`, `crates/nvs-host/src/isolate.rs` and
`crates/nvs-cli/src/runner.rs`.

- [ ] **`worker::place` splits into a post and a collect** — `place_on`
      (`crates/nvs-host/src/worker.rs:540`) parks the calling task until the answer is in the slot,
      and `Host::start_isolate`'s contract (`crates/nvs-runtime/src/host.rs:633`) is **eager**: the
      child is a runnable task before the call returns, so a parent that spawns three and awaits
      three overlaps them. Post the `Start` and hand back the `Answering`/slot pair as something a
      `Box<dyn Running>` can join later, leaving today's blocking `place` as one caller of the two
      halves. `rule:concurrency/on-worker-runs-the-child-on-another-core` is the mechanism it
      implements.
- [ ] **The `Placement::Worker` arm posts the entry and the argument's bytes** —
      `crates/nvs-host/src/group.rs:205`'s one arm becomes two. The `Entry` crosses as it is, the
      argument as `nvs_runtime::graph::encode`'s bytes (`crates/nvs-runtime/src/graph.rs:719`,
      decoded by `:917`), which is ADR 0184 § 2's copy at every node — and the encode belongs on this
      arm rather than at the seam, because a here-placement's crossing is one walk into the child's
      arena. A `GraphError` from it is `StartError::Argument`, exactly as `copy_graph`'s is.
- [ ] **The far core starts the child as a root task and answers with a copied `Completion`** —
      `Isolate::start` (`crates/nvs-host/src/isolate.rs:547` builds the child's context) takes the
      *parent's* `Ctx` for the tree's budget, the depth ceiling and the class table, none of which
      exist on the other core. Decide what crosses for those and record it under ADR 0184's § 4;
      `rule:security/isolate-budget-is-the-trees` is the constraint, and the cancellation edge is
      § 4's acknowledgement.
- [ ] **Each worker core needs its own resolver installed** —
      `crates/nvs-cli/src/runner.rs:454` installs one on the main thread only, and a resolver holds an
      `Rc` unit cache so it is `!Sync` and cannot be shared (`crates/nvs-runtime/src/script.rs:215`).
      Without one the far core answers `ResolveError::NoResolver` for every path form.

## Backlog

- `limits:` and `grants:` below the checker: typed at `crates/nvs-types/src/expr/isolate.rs:171`,
  dropped by the lowering at `crates/nvs-ir/src/lower/expr.rs:3206` — two of stage 6's three cases.
- The three `.nvst` cases `docs/agent/loop-goal.toml:9880` names, under `tests/conformance/isolate/`.
- A worker placement of the **method** form needs the parent's class table on the far core, which is
  `Rc`-shared today — `crates/nvs-host/src/group.rs`'s `# Known gaps`.
- `Core\Socket::upgrade` still carries a `Program` through `nvs_runtime::Upgrade`
  (`crates/nvs-stdlib/src/socket.rs:608`), which is right: a connection is a root isolate on this core.
