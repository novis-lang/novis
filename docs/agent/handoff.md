# Handoff

## State

**Stage 8's `cargo-named` check is closed.** All six names it lists exist in
`crates/nvs-stdlib/tests/queue.rs` and assert against `tests/db/compose.yaml`'s PostgreSQL —
`python tools/db-matrix.py --driver postgres` is green, and with `NVS_DB_MATRIX_DRIVER` unset
every case there skips, so `python tools/verify.py` is unchanged on a machine with no containers.

**ADR 0084 § 3 is asserted as the pair it is, over one connection.** Each of the two cases writes an
application row beside the job inside the same transaction: `orders` creates that table and `landed`
reads both counts in one statement, because "one exists without the other" is a state two separate
reads can each miss. A commit lands both, a rollback leaves neither, and the id `INSERT` answered
with then names nothing — a sequence does not roll back, and the row is all § 3 ever promised.

**§ 6's arithmetic is not asserted twice.** `crates/nvs-stdlib/src/queue.rs`'s
`a_retry_is_exponential_jittered_and_capped` walks the ladder as the pure function it is; the live
case adds only what a server can say — the backoff is a wait the claim enforces, two jobs failing in
one moment are armed for two different ones, and the ladder ends, counted as four claims for two
jobs at two attempts and then a queue with nothing in it however far ahead the worker asks.

**What is open is stage 2, and that check's `args` is half of it.**
`a_named_connection_is_memoized_for_the_request` is `Core\Db::connect`'s, so `nvs-stdlib`'s, and it
sits in the `-p nvs-db` block: writing the test alone leaves the check reporting it forever.
`local_infile_is_refused_and_no_file_is_sent` reports "did not run" permanently in this goal-run —
it is MySQL's, and stage 2's own comment forbids a second driver until PostgreSQL is green end to
end — so closing the first two names moves the ledger's report to that one rather than clearing it.

## Next group

**Stage 2's three unwritten names. The first is `crates/nvs-stdlib/src/db.rs` plus the check block;
the other two share `crates/nvs-db/src/pg.rs`, so take them together and the first on its own.**

- [ ] **A named connection is memoized for the request** — ADR 0067 § 2, the name the driver's
      acceptance check reports. Assert that two `Core\Db::connect("main")` calls in one request are
      one connection and one handshake: `crates/nvs-stdlib/src/db.rs:2298` is the helper and
      `crates/nvs-stdlib/src/db.rs:2163` the name it resolves. **Then move the name out of the
      `-p nvs-db` block at `docs/agent/loop-goal.toml:2764`** — ADR 0132 § 1 means `nvs-db` cannot
      host it — into a stage 2 `-p nvs-stdlib` check of its own, and mirror the edit into
      `docs/agent/goals/5-database.toml:2764` or the next `goal-switch.py` restores the old one.
- [ ] **No driver path interpolates a value into SQL** — ADR 0067 § 1's "emulated prepares do not
      exist in any form", and genuinely `-p nvs-db`'s. The rewriter is
      `crates/nvs-db/src/sql.rs:423`, which answers with a `Statement` and never with text carrying
      a value, and the statement path it feeds is `crates/nvs-db/src/pg.rs:586`.
- [ ] **The connection charset is forced to UTF-8** — ADR 0067 § 9's "connection charset forces
      UTF-8". PostgreSQL's is a startup parameter, so it is set where the handshake is built:
      `crates/nvs-db/src/pg.rs:586`. Nothing in the crate mentions `client_encoding` yet.

## Backlog

- The other four drivers' queue statements — ADR 0084 § 2; `tools/db-matrix.py`'s `SUITES` already
  points this suite at whichever server a leg brought up.
- `Core\Db::open` waits on a shape-parameter type — `docs/implementation-plan.md`, `Open now`.
- Stage 5, three of seven still open — `docs/agent/loop-goal.toml`, that stage's blocks.
- `nvs_stdlib_tests_orders` is this suite's own table and no migration owns it; if a second case
  ever needs an application table, it is `crates/nvs-stdlib/tests/queue.rs`'s `orders` to widen.
