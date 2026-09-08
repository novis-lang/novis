# Handoff

## State

**Goal 20's stage 4 is code-complete: every wire driver answers a socket `host`.** MySQL and MariaDB
open the socket *file* as written — one `nvs_db::mysql::socket_endpoint`, handed to the door for both
drivers rather than a second identical function under MariaDB's name — PostgreSQL derives
`<directory>/.s.PGSQL.<port>`, and SQL Server refuses the path in `TdsTarget::resolve`.
`rule:core-classes/db-unix-socket-path` is `shipped` and names where each answer lives.

**The stage's four named tests exist and pass.** `a_driver_answers_the_same_over_either_transport` is
a unit case in `crates/nvs-db/src/mysql.rs`: the refusal a server sends *instead of* a greeting is the
only exchange a plaintext script can drive over both arms, because `read_greeting` answers an `0xFF`
packet before it compares capabilities and before the in-band upgrade. `crate::pg` is outside that
sweep by design — over TCP its refusal arrives inside TLS.

**A `#[cfg(unix)]` arm is compiled by nothing this repository runs on Windows**, `verify.py` included;
the playbook bullet has the one-line WSL command. All three socket arms build and the four tests pass
there. `python tools/verify.py` is 9 of 9 green on Windows.

**What the goal prose still owes is stage 4 item 5, the matrix's socket leg**, and no `[[check]]`
covers it — so the driver can close goal 20 green with it open. It is filed for that outcome in
[carried-gaps.md](carried-gaps.md) under owner 20, with the detail at `crates/nvs-db/src/matrix.rs`
gap 1. Nothing is blocked.

## Next group

**Stage 4: the matrix's socket leg** — one file set: `crates/nvs-db/src/matrix.rs`,
`crates/nvs-db/tests/handshake.rs`, `tools/db-matrix.py`, `tests/db/compose.yaml`.

- [ ] **`Location` grows a socket arm** — `crates/nvs-db/src/matrix.rs:76`, today a published
      `Server` and a SQLite `File`. A third arm carrying `NVS_DB_MATRIX_SOCKET`, read beside the
      others in `endpoint_from` at `crates/nvs-db/src/matrix.rs:107`, is what lets a case ask for the
      transport `rule:core-classes/db-unix-socket-path` gives the three drivers. The module doc's
      required-group argument applies: a socket endpoint needs no CA, so it is its own arm rather
      than a `Server` with an empty field.
- [ ] **The case list dials the location rather than an address** — `crates/nvs-db/tests/handshake.rs:167`'s
      `address(server)`, and the three `*_connect_as` helpers above it, hand a `SocketAddr` to a
      `connect` that now takes `impl Into<Endpoint>`. One helper answering an `Endpoint` runs the
      existing list over either transport with no case rewritten, which is the property stage 4 item
      5 asks for.
- [ ] **The harness publishes the socket** — `tools/db-matrix.py:320`, where the `NVS_DB_MATRIX_*`
      group is composed per driver. The socket directory of each container is bind-mounted onto the
      host in `tests/db/compose.yaml`; a driver whose leg has no socket is run over TCP alone, on the
      same argument that reports a missing CA as `n/a` rather than as a failure.

## Backlog

- The other five rules ADR 0142 created are still `designed` in `docs/rules/config.json`.
- `Core\Db::open`'s settings path takes a pinned address and never a path — the goal's standing
  decision, `crates/nvs-stdlib/src/db/open.rs:844`; no work, listed so it is not re-derived.
- `docs/agent/carried-gaps.md`'s *Unowned* section carries a literal count sentence; a row added to
  *Owned* does not touch it, but a row moved there does.
