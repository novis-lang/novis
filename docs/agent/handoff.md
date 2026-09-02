# Handoff

## State

**ADR 0067 § 13's pool is on disk end to end.** `crates/nvs-runtime/src/pool.rs` is the per-core
store — a `thread_local!` `Vec`, so the acquire path takes no lock — released into from `Ctx`'s own
`Drop` (`crates/nvs-runtime/src/ctx.rs:1242`) and drawn from by `Core\Db::connect` through
`warm_connection` (`crates/nvs-stdlib/src/db.rs:3096`). § 13's PostgreSQL reset was already landed
and needed no work: `reset_session` (`crates/nvs-db/src/pg.rs:2958`) is six simple queries in one
pipelined batch, and `PgConn::reset` takes `self` **by value**, which is destroy-on-failure itself —
a caller gets a connection back only where every command succeeded.

**The reset is at the acquire end, not the release end**, and `pool.rs`'s module doc § *Where the
reset is* is that decision's home: release happens inside `Drop`, where no driver may do I/O, and a
`Ctx` is also dropped by a CLI and by a test with neither a core to hand back nor a deadline to wait
against. What that costs is stated there — a pooled connection waits carrying session state, never
an open transaction (`HeldConnection::is_poolable` refuses that at release), until the next acquire
clears it.

**The pool key is generation-scoped, and § 13 now says so.** `Ticket::for_block`
(`crates/nvs-runtime/src/pool.rs:81`) keys on the snapshot's address plus the block name, and the
ticket holds that `Arc<Snapshot>` so the address cannot be reused under it. Without the scoping, ADR
0078 § 1's reload could publish a `[db.main]` naming a different database user and the pool would
hand the new generation's request the old one's connection — exactly the sharing § 13 forbids.

**`max` and `acquire` are the two bounds nothing reads yet**, and `pool.rs`'s module doc § *The two
bounds this module does not read yet* says what a core does meanwhile: it opens as many connections
as its requests ask for, as it did before the pool. They are a ceiling on *live* connections and the
wait at it, and a wait needs the core's scheduler rather than a `Vec`. `idle` and `lifetime` are
honoured, and `enabled` is § 13's `pool = false`.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` is
`docs/agent/loop-goal.toml:2927`. § 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:149` argues it — do not re-derive). Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still cannot see the two
`nvs-stdlib` tests, and is still the user's call. The CA is still not in git.

**`orient.py`'s pack was right for both slices** — § 2 was in `[context] adrs` and was what the key
needed. The one file it could not have printed is `pg.rs`'s own reset, which the item predicted as
unwritten and which was already there.

## Next group

**§ 13's remaining half: the ceiling, the wait at it, and the proof that reuse is real and clean.
The file set is `crates/nvs-runtime/src/pool.rs`, `crates/nvs-stdlib/src/db.rs` and
`crates/nvs-db/src/matrix.rs`.**

- [ ] **`max` is a ceiling on live connections per key, not on idle ones** — the pool counts what it
      has handed out and not yet taken back, so a deployment's ceiling on the server is
      `cores × max` and the handbook can say so in those terms. `crates/nvs-runtime/src/pool.rs:217`
      is `take`, the checkout to count; `crates/nvs-runtime/src/pool.rs:81` is the ticket carrying
      the bound; `crates/nvs-runtime/src/ctx.rs:1242` is the release that gives one back.
      ADR 0067 § 13.
- [ ] **`acquire` is how long a request waits at that ceiling before it throws** — zero is legal and
      means never wait (`crates/nvs-config/src/db.rs:102` is that field's doc). This is the slice
      that decides whether the pool parks on `nvs-host` or refuses outright, since a wait needs the
      core; `crates/nvs-stdlib/src/db.rs:3096` is the one acquire site there is.
      ADR 0067 § 13.
- [ ] **A matrix test that reuse is real and clean** — two requests on one core, the second getting
      the first's connection and seeing none of its session state: a temp table, a `SET`, an
      advisory lock. `crates/nvs-db/src/matrix.rs:91` is `endpoint()`, which is how a test finds a
      live server, and `crates/nvs-db/src/pg.rs:2958` is the reset whose property list the case
      asserts. ADR 0067 § 13.

## Backlog

- Stage 5's remaining three of seven — `docs/agent/loop-goal.toml:2830`.
- `Core\Queue::push` and the rest of Stage 8 — ADR 0084, and the acceptance line that fails on it.
- § 7's backoff, blocked on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- `open` waits on a registry shape parameter — `nvs-stdlib`'s db module doc, known gap 1.
- The four drivers that are not PostgreSQL, none of them poolable — `crates/nvs-db/src/conn.rs`.
- The TLS CA is still not in git, so a fresh clone cannot run the fixtures — `nvs.toml`.
