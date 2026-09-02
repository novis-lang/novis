# Handoff

## State

**ADR 0067 § 13's configuration half is on disk.** `[db.<name>.pool]`'s four bounds and the
`pool = false` switch are declared in `crates/nvs-config/src/tree.rs:567` and resolved at boot by
`crate::db::pool_for` (`crates/nvs-config/src/db.rs:162`), which `crate::db::validate`
(`:142`) runs over every block from `resolve.rs:307`. Nothing builds a pool yet: the bounds have no
reader outside that check, and the next two slices are what give them one.

**One key in two shapes, and the `Deserialize` is hand-written for it.** § 13 writes `pool = false`
and `[db.<name>.pool] max = 16` against the same TOML key, so `tree::Pool` is a switch-or-bounds
enum. It is not `#[serde(untagged)]`: untagged buffers through `serde`'s `Content` and reports *data
did not match any variant* for a typo inside the table, throwing away the unknown-key refusal
`tree.rs`'s own module doc calls its security-relevant half. The visitor at `tree.rs:576` and
`crates/nvs-config/tests/db.rs`'s last case are that decision's two homes.

**The default set is § 13's own example** — `max = 16`, `idle = 2`, `lifetime = 30m`,
`acquire = 5s`, transcribed into `PoolBounds::DEFAULT` (`crates/nvs-config/src/db.rs:90`) rather
than chosen, so ADR 0074's finite-with-nothing-configured is a copy and not a judgement. `OFF` is
that set with `enabled: false`, deliberately still finite, so a caller that consults a bound on an
off pool reads a number rather than a zero.

**Three values parse and are still refused, each `E0601`**: `max = 0` (helps toward `pool = false`,
§ 13's one spelling of off), an `idle` above `max`, and a `lifetime` of `0` or `false`. `acquire = 0`
is *accepted* and means never wait — its field doc says so. That split is `pool_for`'s `# Errors`.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` is
`docs/agent/loop-goal.toml:2927`. § 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:139` argues it — do not re-derive). Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still cannot see the two
`nvs-stdlib` tests, and is still the user's call. The CA is still not in git.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13, which was exactly right for this slice.** The next
two need **§ 2** — the memoization key the pool keys on — so add `0067 § 2` to `[context] adrs` in
`docs/agent/loop-goal.toml`. §§ 7 and 8 are no longer worth asking for; § 8 is closed.

## Next group

**Stage 7's pool, the two slices this session did not reach. The file set is
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/db.rs`, `crates/nvs-db/src/pg.rs` and
`crates/nvs-config/src/db.rs`** — the last of which is landed and read-only for both.

- [ ] **The per-core store, keyed as § 2 already keys** — a request's connection is released to a
      per-core pool at teardown instead of being dropped, under the key § 2 already computes. The
      held connection is `crates/nvs-runtime/src/ctx.rs:295` and the key
      `crates/nvs-runtime/src/ctx.rs:2983`; `crates/nvs-stdlib/src/db.rs:2327` is the `connect` that
      memoizes one, and `crates/nvs-config/src/db.rs:162` hands over `max`, `idle`, `lifetime` and
      `acquire`. Per core and never shared between cores, so the acquire path takes no lock.
      ADR 0067 § 13.
- [ ] **The reset is the gate, and a failed reset destroys the connection** — PostgreSQL's targeted
      reset already exists as `crates/nvs-db/src/pg.rs:769` over
      `crates/nvs-db/src/pg.rs:2958`; what is missing is that release runs it and that anything but
      success closes the connection rather than returning it, which is a standing decision and not a
      call to make. Deliberately not `DISCARD ALL` — it would deallocate § 1's statement cache.
      ADR 0067 § 13.

## Backlog

- `pool = false` has no `.nvst` or example fixture — `docs/agent/loop-goal.toml` Stage 5.
- § 7's backoff, blocked on `nvs-runtime` known gap 3 — `crates/nvs-stdlib/src/db.rs:139`.
- Stage 5's `-p nvs-db` check cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- Stage 8's `Core\Queue` is unlanded and holds the acceptance line — ADR 0084.
- `driverCode` is written nowhere on PostgreSQL and never will be — `nvs_db::ServerError`'s doc.
- The CA bundle is not in git — `nvs_host::tls`'s module doc owns why.
