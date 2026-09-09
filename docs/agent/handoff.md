# Handoff

## State

**Goal 23 — `nvs serve` takes every core. Stage 2 is complete and green.** Goal 22's whole list is
this goal's Stage 1 floor and stays green.

**What landed:** step 4 is a compare rather than an assignment. `Compiler::advance`
(`crates/nvs-cli/src/script.rs:381`) takes the digest the resolve copied out of the map on its way
in and writes the path pointer only where the map still names it, so the slower of two
revalidations cannot roll back the fresher one; it is not told, because the unit table is keyed by
content and still answers it with what it compiled. All five of the stage's `[[check]]` tests are on
disk and passing, including the fleet's half of
`rule:config/an-edit-reaches-the-next-request-without-a-restart`: a reader that resolved before the
swap keeps answering out of its own `Arc` after the sweep, a stale revalidation loses the compare, a
winning one publishes while three reader threads are answered throughout its compile, and the
compile counter moves with contents rather than with cores.

`script.rs`'s first known gap (`crates/nvs-cli/src/script.rs:63`) is now single-flighting alone.
Nothing is blocked on a decision.

## Next group

**Stage 3: the fan-out — every `listen` entry bound, on every core** — one file set:
`crates/nvs-cli/src/serve.rs` and `crates/nvs-config/src/server.rs`.

- [ ] **`[server] workers`, and
      `a_server_workers_key_bounds_the_count_and_defaults_to_available_parallelism`** — the key does
      not exist yet: `crates/nvs-config/src/server.rs:248` (`listen_on`) is the neighbour to file it
      beside, and `crates/nvs-cli/src/serve.rs:93` (`run`) is what reads it. The goal's § *Standing
      decisions* is the specification — bounded by configuration, defaulting to the available
      parallelism, and no "auto" spelling that means anything else — and
      `rule:http-server/the-server-block-is-boot-class` is the class the key takes.
- [ ] **`every_entry_of_server_listen_is_bound_rather_than_the_first` and
      `listen_and_port_flags_still_override_the_file_and_still_conflict`** —
      `crates/nvs-config/src/server.rs:225` (`Listen`) is the entry shape and
      `crates/nvs-cli/src/serve.rs:750` (`address`) is where one of them becomes a bound socket
      today. `rule:http-server/two-deployments-and-nothing-a-proxy-owns` is what both state.
- [ ] **`one_worker_is_spawned_per_core_and_each_takes_its_own_listener_handle` and
      `a_unix_domain_entry_is_refused_once_rather_than_once_per_core`** —
      `crates/nvs-cli/src/serve.rs:93` (`run`, 505 lines: bind, then the accept loop) and
      `crates/nvs-cli/src/serve.rs:276`, where the shared unit cache is already handed out by `Arc`.
      The refusal is the check's own word; read the stage's comment header in `loop-goal.toml` before
      choosing where it is raised.

## Backlog

- Single-flighting a compile in progress — `crates/nvs-cli/src/script.rs:63`, the gap's remaining
  half, and what `docs/plan/m7.md`'s cold-request figure rests on across cores.
- A losing revalidation's `record` can sweep the winner's unit entry, costing the next resolve of
  that path one recompile — same module doc paragraph; it closes with the single-flight entry.
- Stage 4 (nothing leaks across a core) and stage 5, per `docs/agent/loop-goal.toml`.
