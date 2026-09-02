# Handoff

## State

**The matrix carries a trust anchor, so a `-p nvs-db` case can reach a live server.**
`NVS_DB_MATRIX_CA` names a PEM bundle and is in the *required* group with the host and the port,
not beside it: every server in `tests/db/compose.yaml` is TLS-only, no public root vouches for one,
and `nvs_host::tls` has no spelling for connecting without verifying, so an endpoint with no anchor
is not one a case can reach at all. `crates/nvs-db/src/matrix.rs`'s module doc is that argument's
only home. `tools/db-matrix.py` exports the file out of the container per run — the anchor belongs
to a Docker volume and is reissued with it — into the scratch directory it already builds.

**Two of the five servers cannot be reached, and the harness says so rather than running them.**
MariaDB serves no certificate as the compose file configures it, and SQL Server keeps its own in
the instance rather than in a file, so `tools/db-matrix.py` prints `mariadb: n/a` / `mssql: n/a`
and exits 2. **Stage 6's `[[check]]` (`docs/agent/loop-goal.toml:2866`) will report those two lines
from now on: that is did-not-run, not a regression** — it was reporting `ok` for five legs that
never opened a socket, and the stage's own comment header now says what each of the two is owed.
`postgres: ok` runs green end to end today.

**The "five test files" sentence was rationale, not a roster.** `crates/nvs-db/src/lib.rs`'s
§ *`NVS_DB_MATRIX_*`* argues that one reader beats one parser per test file; it never named files
that are owed. It is reworded so it cannot read as a plan again, and that module doc's stale
"no driver can yet complete a handshake" paragraph — written before `tls_ca_file` landed — now
states the seam that exists.

**Unchanged and still true.** The driver's acceptance line for `examples/queue.nvs` is Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression. § 7's backoff is still blocked on
`nvs-runtime`'s known gap 3 (`crates/nvs-stdlib/src/db.rs:149` argues it). Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still cannot see the two
`nvs-stdlib` tests, and is still the user's call.

**`orient.py`'s pack is short the pool for the next group.** `[context] modules` names neither
`nvs-runtime/src/pool.rs` nor `nvs-stdlib/src/db.rs`, which the two slices below both open, and it
is still short `nvs-host/src/timer.rs`, `nvs-host/src/group.rs` and `nvs-runtime/src/host.rs` as
the last handoff reported.

## Next group

**§ 13's reuse and its queue, proven against the live PostgreSQL the anchor now reaches. The file
set is `crates/nvs-runtime/src/pool.rs`, `crates/nvs-stdlib/src/db.rs` and a new test file.**

- [ ] **Two requests on one core share one connection** — the first releases at teardown, the
      second draws it warm through `crates/nvs-runtime/src/pool.rs:495`'s `take` and its reset ran.
      Assert the *identity* of the connection, not that a second query worked. ADR 0067 § 13.
- [ ] **A third request at the ceiling waits and then throws naming `acquire`** — `max = 1` with a
      short `acquire`, two tasks, and the refusal from `crates/nvs-stdlib/src/db.rs:3159`. The
      queue's own unit cases are at `crates/nvs-runtime/src/pool.rs:905`; what this adds is a real
      park. ADR 0067 § 13.

## Backlog

- MariaDB needs the `certs` volume PostgreSQL mounts, and SQL Server a certificate on disk, before
  their matrix legs can run — `tests/db/compose.yaml:113`, Stage 6.
- § 7's backoff waits on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- Stage 5's `-p nvs-db` args cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- Stage 8's `Core\Queue` (ADR 0084) is what `examples/queue.nvs` waits on.
- `[context] modules` is short the five modules named in `## State` — `docs/agent/loop-goal.toml`.
