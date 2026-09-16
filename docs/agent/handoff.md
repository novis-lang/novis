# Handoff

## State

**Goal `unowned-closures`, stage 6 (the register).** `python tools/owners.py` reads 79 items with
`unowned: 4`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, and `--deferrals` green. The four left
are two in `nvs-db` and two in `nvs-stdlib`.

**The stack-ceiling item was struck, not scheduled: the code is ahead of the prose the gap was written
from.** The scheduler re-arms every task from the stack it handed out, at the one place a task's
coroutine is built (`crates/nvs-host/src/scheduler.rs:826`, `crates/nvs-host/src/stack.rs:93`), so
`rule:concurrency/a-tasks-recursion-limit-comes-from-its-own-stack` is answered for every program —
`nvs run`, `nvs serve`, a queue worker and a test suite all reach their work through
`nvs_host::Scheduler::spawn`. `Ctx::new`'s asserted `STACK_CEILING` bounds only a context driven on a
thread's own stack, which is an embedder's case and no request's, and
`crates/nvs-runtime/src/ctx/mod.rs` states it as that bound.

**The decoded-`Core`-instance item took a chain entry.** Goal `core-class-tests` is now between
`worker-placement` and `gap-zero`: `instanceof` and `as` refuse a `Core` class because `nvs-types` has
no testable descriptor for one, while `nvs_stdlib::instance::class_descriptors` already publishes the
address a test would walk and `Ctx::class_desc` already resolves a decoded value under its own
descriptor — so it is a checker roster rather than any milestone's work, and no plan states the scope a
deferral would need. Prose, manifest, acceptance list and seed handoff are on disk; `graph.rs` tags to
it and its `docs/agent/carried-gaps.md` § *Unowned* bullet is a row in that file's § *Owned* table.

## Next group

**Stage 6: the register** — one file set: `crates/nvs-db/src/`. Each item is a scheduling decision
written into the gap's own `— owner:` line: a goal slug on the chain, or an M9+ deferral whose plan
states the scope (`python tools/owners.py --deferrals` is the gate on the second kind).

- [ ] **A span renders on a flag no grant turns on** — `crates/nvs-db/src/span.rs:65` gap 1.
      `rule:testing/debug-probes` writes the `debug.trace` grant and spec § 11 gates the output on it,
      but nothing outside a test sets `DebugFlags::TRACE` from a capability. Both halves are above this
      crate — the flag is `nvs-runtime`'s, the grant `nvs-config`'s — so what the owner owns is the
      place a request's capability set is read into its `Ctx`, which is a seam rather than a driver
      change.
- [ ] **A `bytes` parameter is refused by the TDS driver** — `crates/nvs-db/src/tds/mod.rs:88` gap 1,
      this driver's only departure from [ADR 0067 § 9](../decisions/0067.md)'s table and pinned by
      `a_bound_parameter_renders_as_t_sql_reads_it_and_a_bytes_is_refused`. `rule:core-classes/db-one-api`
      is what the departure is measured against. Closing it makes `sp_prepexec`'s `@params` a function
      of the values a call binds rather than of the statement alone, which reaches § 1's plan-cache key
      — so the decision is that trade against stating the refusal as a bound of this dialect.

## Backlog

- The last two unowned items share `crates/nvs-stdlib/src/`: `cache.rs:155` gap 1 (a shared store
  behind a password, an index or TLS) and `response.rs:199` gap 2 (nothing adjudicates between two
  declarations).
- `crates/nvs-stdlib/src/queue.rs` gap 1 has two owners: the module tags `unowned-closures` and
  `docs/agent/carried-gaps.md` § *Owned* gives it to `gap-zero`. One of them is wrong and the register
  reads both.
- When this goal's checks go green the driver takes goal `class-scoped-types`.
