# Handoff

## State

**§ 13's pool is proven live four ways, and the fourth is the park.** `examples/pool.nvs`
now ends in a `Core\Task::all` (ADR 0072 § 1) whose first child asks the full pool for the
connection the request itself is holding while its sibling sleeps three times beside it: a
wait that kept the core would take the two in sequence and the fixture prints the elapsed
milliseconds instead of its verdict. Both goal copies carry the fourth line (stage 7).

**A task child now sees its request's configuration.** `Ctx::child`
(`crates/nvs-runtime/src/ctx.rs:2734`) copies `config` beside the five fields it already
copied, and it copies the **whole `nvs_config::Request`, overlay included** — ADR 0006's
table row ("derived, never shared: a copy of the parent's *effective* config, which the
spawn may narrow"), which is what `Ctx::isolate` already did and for the same reason. The
direction is the security one: `[capabilities]` is a `RuntimeTighten` directive, so a child
re-reading the snapshot alone would hand back a capability its parent had dropped. That doc
comment is the decision's home; no ADR was opened, because 0006's row already decides it.
`a_task_child_is_granted_what_its_request_was_granted` in
`crates/nvs-runtime/src/capability.rs` is the unit half, with an unconfigured child as its
negative control.

**Unchanged and still true.** The driver's acceptance line for `examples/queue.nvs` is
Stage 8's unlanded `Core\Queue` (ADR 0084), not a regression. Stage 6's `mariadb: n/a` /
`mssql: n/a` are did-not-run. § 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:149`). Stage 5's `args = ["test", "-p", "nvs-db"]` still cannot
see the two `nvs-stdlib` tests — the user's call.

**`orient.py`'s pack is still short.** `[context] modules` names none of
`nvs-runtime/src/pool.rs`, `nvs-runtime/src/ctx.rs`, `nvs-runtime/src/capability.rs` or
`nvs-stdlib/src/db.rs`, and this session read all four; the first is the next group's own
file set.

## Next group

**The two live halves of § 13's bounds that only a real server can show. The file set is
`crates/nvs-db/tests/pool_reuse.rs` and `crates/nvs-runtime/src/pool.rs`, and both cases
join the two already in that file under the same `tools/db-matrix.py` skip rule.**

- [ ] **A connection past its `lifetime` is retired rather than handed on, over a real
      server** — the live double of `crates/nvs-runtime/src/pool.rs:778`, which only moves a
      clock. Take a connection, release it, move past `[db.tight.pool] lifetime`, take again,
      and assert `pg_backend_pid()` **differs** — the inverse of
      `crates/nvs-db/tests/pool_reuse.rs:126`'s assertion, in the same shape. The scan that
      closes it is `crates/nvs-runtime/src/pool.rs:564`.
- [ ] **`idle` bounds what a pool keeps, over a real server** — the live double of
      `crates/nvs-runtime/src/pool.rs:714`: release more connections under one key than
      `idle`, and the surplus is closed rather than filed, so the next request's pid is a new
      one. Anchors as above, beside `crates/nvs-db/tests/pool_reuse.rs:198`.

## Backlog

- A closure's `catch` binding that shadows an enclosing one panics the lowerer —
  `crates/nvs-ir/src/lower/expr.rs:2424`; the playbook holds the workaround.
- Stage 8's `Core\Queue` (ADR 0084) is what the driver's acceptance line reports.
- § 7's backoff waits on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- `open` still waits on a shape-parameter type — `docs/adr/0067-core-db.md` § 2.
- Stage 5's `-p nvs-db` args cannot reach the two `nvs-stdlib` cases — the user's call.
- MariaDB and SQL Server report `n/a` until `tools/db-matrix.py` has a server for them.
