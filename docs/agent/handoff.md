# Handoff

## State

**§ 13's reuse is proven against a real server.** `crates/nvs-db/tests/pool_reuse.rs` is the
tree's first live matrix case: one core's first request releases at teardown, the second
admits under the same key, `take`s what was released and resets it, and the assertion is
`pg_backend_pid()` on both sides — the identity of the backend process, so an equal pid is
one connection and not a second one that answered as well. The temp table the first request
left is gone after the reset. Its module doc is the only home for why it is an integration
test (a socket and an anchor, which a unit test in this crate has neither of) and for why
the teardown is `nvs_runtime::pool::release` called directly, as `Ctx`'s `Drop` calls it.

**Verified live, not just compiled**: `python tools/db-matrix.py --driver postgres` is green,
and the case was run by hand against a wrong database to see it panic at the handshake —
a green matrix leg alone cannot tell a live case from a skipped one (playbook, *Running
things*).

**`crates/nvs-db/src/lib.rs`'s crate map no longer says the pool is still to come.** The
store is `nvs_runtime::pool`, the acquire path is `nvs-stdlib`'s, and what this crate holds
is the release gate and the reset.

**Unchanged and still true.** The driver's acceptance line for `examples/queue.nvs` is Stage
8's unlanded `Core\Queue` (ADR 0084), not a regression. Stage 6's `mariadb: n/a` /
`mssql: n/a` are did-not-run, and `docs/agent/loop-goal.toml:2866`'s header owns what each is
owed. § 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:149`). Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests — the user's
call.

**`orient.py`'s pack is still short this group's files.** `[context] modules` names neither
`nvs-runtime/src/pool.rs` nor `nvs-stdlib/src/db.rs`, which every slice below opens, and is
also short `nvs-host/src/net.rs`, `nvs-host/src/timer.rs`, `nvs-host/src/group.rs` and
`nvs-runtime/src/host.rs`.

## Next group

**The rest of § 13 against the live PostgreSQL. The file set is `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-runtime/src/pool.rs` and `crates/nvs-db/tests/pool_reuse.rs`, which items 2 and 3
extend.**

- [ ] **A third request at the ceiling waits and then throws naming `acquire`** — `max = 1`
      with a short `acquire`, two tasks, and the refusal from
      `crates/nvs-stdlib/src/db.rs:3159`. The queue's own unit cases are at
      `crates/nvs-runtime/src/pool.rs:905`; what this adds is a real park. **Decide first what
      provides the two tasks**: `crates/nvs-db/tests/pool_reuse.rs:124` has no scheduler under
      it, so this is either a case where a core can be started or a Novis program under
      `examples/`. ADR 0067 § 13.
- [ ] **`pool = false` restores connect-per-request exactly** — nothing is taken back
      (`crates/nvs-runtime/src/pool.rs:520` returns before the store on `!enabled`) and
      `admit` still never refuses, so the second request's pid *differs*. Beside the reuse
      case at `crates/nvs-db/tests/pool_reuse.rs:124`. ADR 0067 § 13.
- [ ] **A connection past its `lifetime` is retired rather than handed on** — release, then
      `take` with a `now` past `retire`, and `crates/nvs-runtime/src/pool.rs:568`'s scan
      closes it, so the pid differs there too. Same file, at
      `crates/nvs-db/tests/pool_reuse.rs:124`. ADR 0067 § 13.

## Backlog

- Stage 6's own first work: the `certs` volume for MariaDB and a certificate on disk for SQL
  Server — `docs/agent/loop-goal.toml:2866`'s header.
- Stage 8's `Core\Queue` (ADR 0084) is what the acceptance line on `examples/queue.nvs` reports.
- § 7's retry backoff, blocked on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- `open` waits on a shape-parameter type — `docs/implementation-plan.md`, *Open now*.
- Stage 5 cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`, user's call.
- `[context] modules` is short `pool.rs`, `db.rs` and `net.rs` — `docs/agent/loop-goal.toml`.
