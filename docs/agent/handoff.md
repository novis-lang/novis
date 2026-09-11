# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stages 0 and 2
are landed, stage 3 is landed for the CLI run path, and the served path is now wired end to end.** An
isolate publishes the tree it joins for as long as its body runs and clears it however the body ends
(`crates/nvs-host/src/isolate.rs:189`), and `nvs serve` hands its core's registration to every isolate
the handler builds (`crates/nvs-cli/src/serve.rs:706`). The two cases still red under stage 0's check
are the memory pair, which is stages 4 and 5. Goal `editor-surfaces`'s acceptance list is still this
goal's floor and is untouched.

**That wiring stops nothing yet, and the reason is outside this goal's stage list: a served request
carries no configuration at all.** The connection's root context is a bare `Ctx::new`
(`crates/nvs-server/src/serve.rs:1343`), `Ctx::isolate` copies the parent's `config`
(`crates/nvs-runtime/src/ctx/isolate.rs:363`), and the cached ceilings are refreshed only by
`Ctx::set_config` (`crates/nvs-runtime/src/ctx/wiring.rs:301`) — which nothing on the served path
calls. So `Ctx::cpu_limit()` is `0` there, every `[limits]` key reads as absent, and
`nvs_runtime::capability::granted` (`crates/nvs-runtime/src/capability.rs:95`) denies every capability
an entry asks for. Read from the code at those four anchors and **not** observed against a running
server. `rule:config/the-config-is-an-immutable-snapshot` is the rule it does not meet, and closing it
is what makes every ceiling on this path live — not the publication.

What a publication is charged against is settled. `Registration` holds the clock of the thread that
registered (`crates/nvs-host/src/watchdog.rs:500`), taken inside `register`, so no caller can hand it
another thread's; `Registration::publish` (`crates/nvs-host/src/watchdog.rs:540`) is the door, and a
tree under no cap clears the slot rather than filling it. What is published is the **tree's** handle
and the tree's ceiling, both read off the context `Isolate::start` was called on, because
`Ctx::isolate` carries no ceiling across on purpose.

## Next group

**Stage 3: a served request reads the tree the instance booted on** — one file set,
`crates/nvs-server/src/serve.rs` with `crates/nvs-cli/src/serve.rs` for what is handed across to it.
`rule:config/the-config-is-an-immutable-snapshot` is what the group owes: "a request clones the `Arc`
when it starts and reads from that clone for its whole life".

- [ ] **Carry the boot snapshot across the seam** — `crates/nvs-cli/src/serve.rs:452`'s `Core` already
      holds the `Arc<nvs_config::Snapshot>` every core reads, and `crates/nvs-server/src/serve.rs:1269`
      is the door it has to cross. That crate may name the type
      (`crates/nvs-server/Cargo.toml:26`); `nvs-host` deliberately may not
      (`crates/nvs-host/Cargo.toml:52`), so the builder on `crates/nvs-host/src/isolate.rs:147` is the
      wrong home for it.
- [ ] **Set it per request rather than per connection** — `crates/nvs-server/src/serve.rs:815` is the
      line that has both the connection's context and the isolate about to run on it, and a keep-alive
      connection serving a second request after a reload is why the store cannot go at
      `crates/nvs-server/src/serve.rs:1343` instead (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
- [ ] **Pin it** — `crates/nvs-cli/src/serve.rs:1575`'s `requests_on` already drives real requests
      through a real core; a case there asserting that a served request runs under the `[limits]` the
      tree states is what turns the paragraph above from a reading into a guard.

## Backlog

- A publication is one slot per registered thread, so a core interleaving requests charges the one it
  took up last and clears at the first to finish — `crates/nvs-host/src/watchdog.rs:95`'s module doc
  owns the over-charge; what it does not yet answer is a request that yields and *then* runs away.
- A publication carries the ceiling that stood before `Ctx::run_limit_handler` widened it by
  `fatal_reserve_time` — `crates/nvs-runtime/src/ctx/limits.rs:359`.
- `nvs test`'s in-process suite path runs user code under no sampler at all —
  `crates/nvs-cli/src/runner.rs:465`.
- Stage 4 and stage 5 are the memory pair stage 0's check is still red on —
  `docs/agent/loop-goal.toml:7974`.
