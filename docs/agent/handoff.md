# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6: the host seam names the child's program, and a placement is now
two calls rather than one.**

`nvs_host::worker::post` (`crates/nvs-host/src/worker.rs:645`) posts the `Start` and answers a
`Posted` (`crates/nvs-host/src/worker.rs:400`) without parking; `Posted::collect`
(`crates/nvs-host/src/worker.rs:439`) is the park, and `finished`/`abandon`
(`crates/nvs-host/src/worker.rs:428`, `:468`) are the other two a `Box<dyn Running>` owes. `place` is
the two halves back to back, so the blocking form is one caller of them rather than the only shape,
and `Err(f)` hands the function back untouched for the three states with no core — off a core, no
wake, no core startable. That is `Host::start_isolate`'s eagerness
(`crates/nvs-runtime/src/host.rs:628`) made reachable across a thread.

**Still routed here.** `SchedulerHost::start_isolate` (`crates/nvs-host/src/group.rs:205`) sends both
placement words to this core's own start, and `crates/nvs-host/src/group.rs`'s `# Known gaps` is that
gap's one home. `limits:` and `grants:` still stop at the checker.

Stage 6's `cargo-named` check over `nvs-host` passes as it stands — its three tests are in
`crates/nvs-host/src/worker.rs` — so what is red is the `nvs-types` options check and the three
`.nvst` cases.

## Next group

**Stage 6: the options, the route a worker placement takes** — one file set:
`crates/nvs-host/src/group.rs`, `crates/nvs-host/src/worker.rs`, `crates/nvs-host/src/isolate.rs` and
`crates/nvs-runtime/src/host.rs`. **The three land together**: an arm that posts with nothing on the
far core to answer it is a half-built crossing, so this is one group and not three sessions.

- [ ] **The `Placement::Worker` arm posts the entry and the argument's bytes** — the one-arm match is
      `crates/nvs-host/src/group.rs:245`. `Entry` is already `Send`; the argument is not, so the arm
      encodes it with `nvs_runtime::graph::encode` (`crates/nvs-runtime/src/graph.rs:719`) and answers
      `StartError::Argument` (`crates/nvs-runtime/src/host.rs:311`) on a refusal, which is the variant
      already there for it. `rule:security/isolate-values-cross-by-copy` is what the encode
      implements, minus the move (ADR 0184 § 2).
- [ ] **The far core starts the child as a root task and answers with a copied `Completion`** — the
      posted job runs with **no `Ctx`**: `receive` spawns it with a `Ctx::new(OutputSink::Sink)` the
      closure ignores (`crates/nvs-host/src/worker.rs:780`), so the job builds the far side's root
      context itself and carries the tree's budget and script depth across —
      `Isolate::start` reads both off the context it is called on
      (`crates/nvs-host/src/isolate.rs:444`, the depth breach at `:462`).
      `Completion` (`crates/nvs-runtime/src/host.rs:366`) cannot cross as it stands, because its
      `value` is a `Value`: `Answer<T>`'s `T` is a crossed form carrying `graph::encode`'s bytes,
      decoded back on the parent's core at the collect.
      `rule:security/isolate-budget-is-the-trees` is the budget half, ADR 0184 § 3 the mechanism.
- [ ] **Each worker core installs two resolvers, not one** — `Entry::program`
      (`crates/nvs-runtime/src/host.rs:292`) asks `nvs_runtime::script`'s thread-local resolver, whose
      install wants a `&'static dyn Resolver` and whose `scoped` form is what a resolver that is not
      one uses (`crates/nvs-runtime/src/script.rs:218`, `:226`); `graph::decode`
      (`crates/nvs-runtime/src/graph.rs:917`) wants a class resolver `&dyn Fn(&str) -> Option<*const
      ClassDesc>`, which is `ctx.class_desc` on the far core's own context
      (`crates/nvs-stdlib/src/serialize.rs:162` is the shape). `run_core`
      (`crates/nvs-host/src/worker.rs:746`) is where the first goes, beside the reactor it already
      installs; the second arrives with the context the job above builds. The CLI's resolver caches an
      `Arc`-held compile already (`crates/nvs-cli/src/script.rs:145`), so a worker core shares one
      compile rather than making its own.

## Backlog

- `limits:` and `grants:` below the checker: typed at `crates/nvs-types/src/expr/isolate.rs:171`,
  dropped by the lowering at `crates/nvs-ir/src/lower/expr.rs:3206` — two of stage 6's three cases.
- The three `.nvst` cases `docs/agent/loop-goal.toml:9880` names, under `tests/conformance/isolate/`.
- A worker placement of the **method** form needs the parent's class table on the far core, which is
  `Rc`-shared today — `crates/nvs-host/src/group.rs`'s `# Known gaps`.
- A serving core registers no inbox, so `nvs serve` places on a lazily started core instead of a
  sibling — the pre-authorized fallback, `crates/nvs-host/src/worker.rs`'s `# Known gaps`.
- `Core\Socket::upgrade` still carries a `Program` through `nvs_runtime::Upgrade`
  (`crates/nvs-stdlib/src/socket.rs:608`), which is right: a connection is a root isolate on this core.
