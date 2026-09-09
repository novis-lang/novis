# Handoff

## State

**Goal 20's stage 4 is code-complete in `nvs-db`, and the matrix now carries the socket leg
in-process.** `nvs_db::matrix::Location` has a third arm — `Socket`, selected by the presence of
`NVS_DB_MATRIX_SOCKET` and carrying no CA, because nothing vouches for a socket — and
`crates/nvs-db/tests/handshake.rs` reads whichever leg was published into one `Leg` (an
`nvs_db::conn::Endpoint` plus the credentials), so the existing case list runs over either transport
with no case rewritten. SQL Server is left out on both sides for one reason,
`rule:core-classes/db-unix-socket-path`: `matrix::endpoint` stops a run that schedules a socket leg
for it, and `TdsConn::connect` takes an address rather than an `Endpoint`.

**What stage 4 still owes is the harness half.** `tools/db-matrix.py` sets no `NVS_DB_MATRIX_SOCKET`
and `tests/db/compose.yaml` bind-mounts no socket directory, so nothing ever hands a case one and
`AF_UNIX` is still asserted only against listeners this crate binds itself. That is
`crates/nvs-db/src/matrix.rs` gap 1, carried in [carried-gaps.md](carried-gaps.md) under owner 20.

**The acceptance check for `a_driver_answers_the_same_over_either_transport` cannot go green on this
host, and the tree is not what is wrong.** The test exists and passes; it is `#[cfg(unix)]`, and
nothing this repository runs on Windows compiles that arm — the playbook's *A `cfg(unix)`-only test
cannot satisfy a `cargo`-named acceptance check* bullet owns it. Do not write it a second time.

`python tools/verify.py` is 9 of 9 green on Windows, and nothing here adds a `cfg` arm: the socket leg
reaches `AF_UNIX` through `pg::socket_endpoint` and `mysql::socket_endpoint`, which both platforms
have.

## Next group

**Stage 4: the harness publishes the socket** — one file set: `tools/db-matrix.py`,
`tests/db/compose.yaml`, `crates/nvs-db/src/matrix.rs`.

- [ ] **The containers publish a socket directory** — `tests/db/compose.yaml:116`'s `mysql` service
      and its `mariadb` and `postgres` twins, each of which must bind-mount the directory its engine
      binds a socket in onto the host, so there is a path for the harness to export at all.
      `rule:core-classes/db-unix-socket-path` says what that path spells per engine: the directory
      itself for PostgreSQL, the socket file for the other two.
- [ ] **The harness exports it** — `tools/db-matrix.py:320`, where the `NVS_DB_MATRIX_*` group is
      composed per driver. `NVS_DB_MATRIX_SOCKET` for those three drivers and unset for the other
      two: `nvs_db::matrix::endpoint` stops a run that sets it for SQL Server, and SQLite reads its
      own path. A driver whose leg has no socket runs over TCP alone, on the same argument that
      reports a missing CA as `n/a` rather than as a failure.
- [ ] **Close the module doc's gap once both land** — `crates/nvs-db/src/matrix.rs:41`, and the row
      it is filed under at `docs/agent/carried-gaps.md:69`. Stage 4 item 5 is met when the TCP legs'
      own case list has run over `AF_UNIX` against a real server, which is one `python
      tools/db-matrix.py` run away once the two above land.

## Backlog

- The other five rules ADR 0142 created are still `designed` in `docs/rules/config.json`.
- `Core\Db::open`'s settings path takes a pinned address and never a path — the goal's standing
  decision, `crates/nvs-stdlib/src/db/open.rs:844`; no work, listed so it is not re-derived.
- `docs/agent/carried-gaps.md`'s *Unowned* section carries a literal count sentence; a row added to
  *Owned* does not touch it, but a row moved there does.
- The four socket tests goal 20 names run under WSL alone, so the driver's stage 4 check stays red on
  this host however much of the goal lands.
