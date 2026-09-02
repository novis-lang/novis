# Handoff

## State

**§ 13's pool is proven live three ways now.** `crates/nvs-db/tests/pool_reuse.rs` holds two of
them — the reuse case, and `a_pool_that_is_off_hands_the_next_request_nothing`, which asks the
same question of `pool = false`: the store hands the second request nothing and it opens a backend
of its own, `pg_backend_pid()` on both sides again. The third is a program: `examples/pool.nvs`
reaches `max` and pins that the refusal comes no earlier than `acquire` and names both bounds.
`python tools/db-matrix.py --driver postgres` is green with both cases live.

**`nvs.toml` grew `[db.tight]`** — the same server under a second name with `max = 1`, `idle = 1`
and `acquire = "500ms"`, plus its own `[[app]]` grant. A second block rather than a
`[db.main.pool]`, because § 13 keys a pool by the block and the other two fixtures must not run
under a ceiling of 1; `idle` is written because it defaults to 2 and an `idle` above `max` does not
load. Both goal copies carry the two new checks (stage 7).

**The gap that stopped the rest of the item.** A `Core\Task` child's context carries no
configuration — `Ctx::child` at `crates/nvs-runtime/src/ctx.rs:2734` copies five request-wide
fields and not `config` — so `nvs_runtime::capability::granted` refuses **every** capability-gated
member inside one. Proven by hand: `Core\Db::connect("tight")` succeeds in `examples/pool.nvs`'s
main body and is refused verbatim inside a `Core\Task::all` child. So § 13's *park* — a request
waiting at the ceiling while the core runs other work, which is what separates it from a spin —
cannot be shown from a program yet, and the fixture pins the elapsed-time half instead. The
playbook bullet is the trap; the decision is item 1 below.

**Unchanged and still true.** The driver's acceptance line for `examples/queue.nvs` is Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression. Stage 6's `mariadb: n/a` / `mssql: n/a` are
did-not-run. § 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:149`). Stage 5's `args = ["test", "-p", "nvs-db"]` still cannot see
the two `nvs-stdlib` tests — the user's call.

**`orient.py`'s pack is short again.** `[context] modules` names neither
`nvs-runtime/src/pool.rs`, `nvs-stdlib/src/db.rs`, `nvs-runtime/src/ctx.rs` nor
`nvs-runtime/src/capability.rs`, and the last two are the next group's whole file set.

## Next group

**What a task child inherits from its request, and then the park proof that needs it. The file set
is `crates/nvs-runtime/src/ctx.rs` and `crates/nvs-stdlib/src/db.rs`, with `examples/pool.nvs` as
the fixture both end at.**

- [ ] **A `Core\Task` child sees its request's configuration** — `Ctx::child` at
      `crates/nvs-runtime/src/ctx.rs:2734` gains the field, beside the five it already copies, and
      its doc says which. The decision to make and record first is *what* crosses: the snapshot
      alone (`nvs_config::Request::new` over `Arc::clone(config.snapshot())`, so a child sees no
      `Core\Config::set` its parent made) or the whole overlay. ADR 0006's table is the one that
      says which side of request-wide this falls on, and `crates/nvs-runtime/src/capability.rs:93`
      is what reads the answer. A `-p nvs-runtime` case asserting `child.config().is_some()` is the
      cheap half; the fixture below is the live one.
- [ ] **The wait at the ceiling is a park and not a spin** — with the above landed,
      `examples/pool.nvs:60` becomes `Core\Task::all` over two children: one ticks through
      `Core\Time::sleep`, the other asks for `{shared: false}` and reads the tick count from inside
      its own `catch`. A count above zero is the core running other work while the wait is on
      (ADR 0106 § 6's tier-B failure, ruled out). The refusal is
      `crates/nvs-stdlib/src/db.rs:3159`'s. Update the fixture's frozen `want` in both goal copies.
- [ ] **A connection past its `lifetime` is retired rather than handed on** — release, then take
      with a `now` beyond the retire instant, over a real socket:
      `crates/nvs-db/tests/pool_reuse.rs:184` is where it goes and
      `crates/nvs-runtime/src/pool.rs:568` is the scan that retires it. ADR 0067 § 13.

## Backlog

- Stage 8's `Core\Queue` is what `examples/queue.nvs` is waiting on — ADR 0084.
- Stage 6's MariaDB and SQL Server anchors — `docs/agent/loop-goal.toml:2864`'s header.
- § 7's backoff, blocked on known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- Stage 5's `-p nvs-db` args cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- `[context] modules` is short four files — `docs/agent/loop-goal.toml`.
- `Core\Db::open`'s shape-parameter type — `docs/plan/m8.md`.
