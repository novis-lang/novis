# ADR 0132 — A driver is a sans-IO codec plus its own state machine over the parking stream, and the five are an enum rather than a trait

- **Status:** Accepted
- **Date:** 2026-09-01
- **Scope:** the *shape* of `crates/nvs-db` — where it sits in the crate graph, which protocol crate backs
  which driver, how a driver reaches the network and how TLS layers on it, how a connection's busy state is
  represented, and how five drivers share code. Not in scope: everything a Novis program can observe, which
  is [0067](0067-core-db.md); the signature list, which is [docs/spec/01-core-library.md](../spec/01-core-library.md)
  § 18; tier placement and the C-dependency test, which are [0051](0051-standard-library-tiers.md); the job
  table that runs on top of this, which is [0084](0084-durable-background-jobs.md).
- **Depends on:** [0067](0067-core-db.md) — it specifies the behaviour this implements and deliberately not
  the implementation; [0115](0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md) —
  the stream every driver but SQLite is written against.

> **In short:** `crates/nvs-db` is one crate below `nvs-stdlib`, holding five drivers and the half that has
> no driver in it. A driver is a **sans-IO codec plus a state machine we write**: `postgres-protocol` for
> PostgreSQL, `mysql_common` for MySQL and MariaDB, `rusqlite` for SQLite, and hand-written TDS for SQL
> Server, because every crate that would have supplied one needs an async runtime that spawns. The wire is
> [ADR 0115](0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md)'s parking stream and
> TLS is `nvs-host`'s one client, generalised over its transport so SQL Server's tunnelled handshake reaches
> the same session type and the same trust anchors. **A connection's busy state is a field on the
> connection, never on the stream**, with a fourth state the surface never names: a wire that is not at a
> known message boundary is `Poisoned`, and a poisoned connection is closed rather than reset — [0067
> § 13](0067-core-db.md)'s reset proves nothing when it is written into the middle of another message. The
> five drivers are an **enum with one `match` per entry point**, not a `Driver` trait: the set is closed,
> the differences are the point, and a trait wide enough to hold all five would be half `unimplemented!()`.

## Context

- [0067](0067-core-db.md) is a long ADR about *behaviour* and says almost nothing about code. It was written
  that way deliberately, and the gap it leaves is the one every driver-shaped project gets wrong once: the
  crate layout, the wire crates, and whether the drivers are a trait. Deciding that in the first driver's
  commit message means rediscovering it four times.
- **No async runtime, and that is structural rather than a preference.** `docs/plan/design.md` § *Thread-per-core,
  shared-nothing runtime* owns it and [0106 § 6](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
  is what it buys. This eliminates `sqlx`, `tokio-postgres` and `tiberius` outright, and it eliminates the
  sync `postgres` crate too, which is a `block_on` wrapper over `tokio-postgres` and pulls the same runtime.
  What remains in the ecosystem is exactly the useful half: the **sans-IO** crates those clients factored
  their message encoding into.
- The stream that replaces the runtime already exists and is proven:
  [`crates/nvs-host/src/net.rs`](../../crates/nvs-host/src/net.rs)'s `NvsStream` parks instead of blocking
  underneath `std::io::Read`/`Write`, and its `a_rustls_session_streams_over_it_unmodified` case has shown
  since ADR 0115 § 3 that an unmodified protocol implementation runs over it. A sans-IO codec is that
  sentence's ideal consumer: it hands us bytes and takes bytes, and the waiting is not its business.
- The five backends do not agree on what an operation *is*. PostgreSQL's extended protocol folds a prepare
  into the execute round trip; MySQL spends a whole extra round trip on the first execution of a statement
  ([0067 § 1](0067-core-db.md)); MariaDB has a bulk execute the others lack; SQL Server has MARS, which
  [0067 § 4](0067-core-db.md) refuses to use so that one program runs on all of them; and SQLite has no
  socket at all. Any shared abstraction is chosen against *that* list, not against a hope that they are the
  same shape underneath.

## Decision

### 1. `crates/nvs-db` is one crate, and it sits below `nvs-stdlib`

One new workspace crate. It depends on `nvs-runtime` (values, `Ctx`, faults), `nvs-host` (the stream, TLS,
and the blocking pool § 3 hands SQLite to) and `nvs-config` (the `[db.*]` block and the capability enum).
`nvs-stdlib` depends on **it**, never the reverse: `Core\Db`'s registry rows, reference cards and helper
bodies stay in `nvs-stdlib` where every other class's are, and the wire lives here, so the graph stays a
tree.

A crate rather than a module of `nvs-stdlib`, for two reasons that are both about the rest of the tree:
five protocol implementations and their dependencies would be rebuilt by every session that only touched
`Core\Str`, and the dependency set that an audit most wants to look at (`postgres-protocol`, `mysql_common`,
`rusqlite`) is worth having behind one `Cargo.toml` with one comment per entry rather than scattered through
the standard library's sixty.

**No per-driver feature flags.** Every driver is in every binary, for [0051 § 5](0051-standard-library-tiers.md)'s
reason — `Core` means always present — and for the reason
[`tls.rs`](../../crates/nvs-host/src/tls.rs)'s trust-anchor section already gives: the same binary should
behave the same way everywhere, and a feature matrix makes "does this deployment speak MariaDB" a property
of how someone built it. The cost is compile time and binary size, which are priorities 3 and 5 against a
simplicity argument at 4.

### 2. Which protocol crate backs which driver

| Driver | Codec | What `nvs-db` writes itself |
|---|---|---|
| PostgreSQL | `postgres-protocol`, plus `postgres-types` for the value codecs | startup, SASL, the extended-query state machine, the § 13 reset |
| MySQL | `mysql_common` | handshake and auth, `COM_STMT_*` sequencing, the `LOCAL INFILE` refusal |
| MariaDB | `mysql_common`, its own auth plugins and its own error table | the above, plus `COM_STMT_BULK_EXECUTE` and `RETURNING` |
| SQL Server | none — TDS 7.4 is written here | all of it, including § 3's tunnelled handshake |
| SQLite | `rusqlite` | nothing on a wire; there is no socket |

The rule the table encodes: **a codec is borrowed, a state machine is written.** Message framing, value
encoding and authentication mechanisms are large, fiddly and identical for everyone, and they are precisely
what the sans-IO crates are. Sequencing — what to send next, what a park in the middle means, when the
connection is reusable — is where this project's own decisions live (§ 4, [0067 § 13](0067-core-db.md)), and
borrowing it is what would have dragged a runtime in.

TDS has no sans-IO crate; `tiberius` is the only implementation and it is async over `futures-io` with its
TLS behind `tokio-rustls`. It is written by hand, and that is the largest single piece of new wire code this
goal buys.

SQLite is the one driver with no bytes: `rusqlite` is a C dependency, accepted in advance under
[0051 § 4](0051-standard-library-tiers.md)'s second question, and the slice that adds it also adds its entry
to `tools/gen-attribution.py`'s `C_DEPENDENCIES` ledger so the enumeration gate keeps failing on any
*other* addition. MariaDB's `ed25519` and `parsec` plugins are that section's other pre-answered case and
are a Rust implementation or a documented refusal — never a second C dependency.

### 3. The wire is the parking stream, and TLS is `nvs-host`'s one client over a generic transport

Four drivers open an `NvsTcp` (or a `NvsUnix` for a local socket, which is the same type over a different
source), negotiate the protocol's own upgrade, and then hold a TLS session for the rest of the connection's
life. [0067 § 3](0067-core-db.md) makes `VerifyFull` the default with no spelling for turning it off, so the
plaintext phase is only ever the upgrade request itself.

Two shapes, and the second is why `nvs-host` changes:

- **In-band upgrade over the same socket** — PostgreSQL's `SSLRequest`, MySQL and MariaDB's `CLIENT_SSL`
  capability flag. A few plaintext bytes, a one-byte answer, then every subsequent byte is a TLS record on
  the same socket. `NvsTls::over(NvsTcp, name)`
  ([`tls.rs:174`](../../crates/nvs-host/src/tls.rs)) expresses this today, unchanged.
- **A handshake tunnelled inside the protocol's own framing** — SQL Server wraps the TLS handshake records
  in TDS `PRELOGIN` packets, so during the handshake the bytes `rustls` produces are not the bytes that go
  on the socket. `NvsTls::over` cannot express that, because its transport is the concrete `NvsTcp`.

So **`NvsTls` becomes generic over its transport** — `NvsTls<T: Read + Write = NvsTcp>`, the default being
what keeps the bare `NvsTls` spelling `Core\Http\Client` and `Core\Mail` already use meaning exactly what it
meant, with no second name to keep in step — and the TDS
packet framer is an ordinary `Read`/`Write` adapter in `nvs-db`. What does *not* generalise is what belongs
to the socket rather than to the session: the deadline and the peer address stay on `NvsTls<NvsTcp>`, so
there is still one clock, on the thing that waits. One TLS client, one answer to "whose
certificates do you believe", one place the future `nvs.toml` anchor bundle plugs into. A second `rustls`
session built inside `nvs-db` would be a second answer to a question `tls.rs` has already decided at length.

SQLite is the exception and it is not a small one: it is a C library doing synchronous file I/O and it
cannot park. A SQLite call goes to `nvs_host::blocking::run`, which is
[0044 § 5](0044-core-process-argv-only-no-shell.md)'s other half of the same rule — a `Core` member either
parks on readiness or hands the core to the blocking pool, and [0106 § 6](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
says there is no third option.

### 4. Busy state is a field on the connection, and a wire that is not at a message boundary is closed

[0067 § 4](0067-core-db.md) requires a second statement on a streaming connection to throw `LogicError`.
That state is **not** on the stream. `NvsStream`'s `registered`
([`net.rs:130`](../../crates/nvs-host/src/net.rs)) is one task's readiness registration, deliberately
invisible above `Read`/`Write`; a connection is busy whether or not its socket is readable, and SQLite must
answer the same `LogicError` with no stream underneath it at all. So each driver's connection carries:

- **`Idle`** — the wire is at a message boundary; a new command may be written.
- **`Executing`** — a buffered statement is in flight, and only its own call may write.
- **`Streaming`** — rows remain unread. A second statement here is § 4's `LogicError`, naming both fixes.
- **`Poisoned`** — the wire is *not* at a known message boundary: a deadline fired mid-message, a decode
  failed, or a stream was abandoned in a shape the driver cannot drain.

A plain `Cell`, with no atomic and no lock, because a task never migrates and a connection is owned by one
request at a time — `nvs-host`'s `Scheduler` is `!Send` and that is what makes it true rather than hoped.

**A poisoned connection is closed, never reset, and never returned to the pool.** This is the section's one
security-relevant call. [0067 § 13](0067-core-db.md) makes the reset a boundary because a connection
carrying one request's state into another's is a cross-tenant leak; a `RESET ALL` written into the middle of
an unfinished message is not a reset, it is a fragment of one request's protocol stream that the *next*
request will read as its own. Draining first would mean trusting a length prefix that has already proven
untrustworthy. Closing costs one handshake and is the only answer that is provable.

An abandoned stream is the ordinary case of this, and it is not automatically poison: a driver that can
cancel and drain a partial result deterministically (PostgreSQL's `Sync` after closing the portal) returns
the connection to `Idle` and pools it. One that cannot marks it `Poisoned`. That choice is per driver, in
the driver, which is § 5's whole point.

### 5. Five drivers are an enum with one `match` per entry point, not a `Driver` trait

```rust
enum Connection { Postgres(PgConn), MySql(MySqlConn), MariaDb(MariaConn), SqlServer(TdsConn), Sqlite(SqliteConn) }
```

Each variant owns its own state machine, its own error-code table and its own reset. `Core\Db`'s entry
points `match` once.

Three reasons, in this project's priority order:

1. **The set is closed.** [0067 § 12](0067-core-db.md) makes a new backend an ADR, not a plugin, and
   [0003](0003-extension-system.md)'s wasm extensions cannot host one anyway — a pool is the thing a sandbox
   boundary cannot hold ([0067](0067-core-db.md) § *Context*). Open-set extensibility is the one property a
   trait buys, and this design does not want it.
2. **A trait wide enough for all five would be half `unimplemented!()`**, and one narrow enough to be honest
   pushes the differences back into the caller, where they are worse. `executeMany` is one prepare and N
   executions on four drivers and a bulk protocol message on the fifth; a prepare is a round trip on two and
   free on one; a reset is four different command sequences and a rollback on the fifth. A trait method that
   four drivers implement by returning an error is a lie the type system helped tell.
3. **Static dispatch on the request path.** `Box<dyn Driver>` is an allocation and an indirect call per
   connection operation (priority 3), where the enum's `match` is something the compiler can see through.

Where a signature repeats five times, **it repeats**. Five `fn query` bodies that each say what their own
protocol does are cheaper to read and to fix than one that makes four of them lie. What keeps them honest is
not a type: it is `tools/db-matrix.py` running one assertion set against five real servers.

What *is* shared is the half with no driver in it, and it is shared as plain functions and data rather than
through the drivers at all: the `?`/`:name` rewriter and `inList` expansion ([0067 § 5](0067-core-db.md)),
the statement cache and its arity-aware key ([§ 1](0067-core-db.md)), the pool and its acquire path
([§ 13](0067-core-db.md)), the Novis side of the type map ([§ 9](0067-core-db.md)), and `ErrorKind`
normalisation ([§ 8](0067-core-db.md)) — whose per-driver code tables are *data* each driver supplies, not
behaviour it overrides.

## Consequences

- **Per connection**, per [0004](0004-memory-for-simplicity.md): a read buffer, a write buffer, the
  statement cache sized by `statement_cache`, and for a TLS connection `rustls`'s session state. That is
  O(in-flight connections), released when the connection is destroyed, and bounded by `cores × max` from
  [0067 § 13](0067-core-db.md). Per process it adds nothing to what `tls.rs` already counts: the trust
  anchors are the same one set.
- **A poisoned connection costs a full handshake** on the next acquire. It is the rare path by construction
  — a deadline or a protocol error — and it buys the property in § 4.
- **TDS is ours to maintain**, including its authentication and its tunnelled TLS. That is the price of
  SQL Server being in the matrix at all, and it is paid once.
- **The C-dependency ledger gains exactly one entry**, `libsqlite3-sys`, when the SQLite driver lands. The
  gate in `tools/gen-attribution.py` still fails on anything else, which is what makes that entry an
  exception rather than a precedent.
- **Adding a sixth backend is an ADR and five edits**, not an implementation of a trait — a variant, a
  state machine, an error table, a reset, and a row in the matrix harness.

## Alternatives rejected

- **`sqlx`, `tokio-postgres`, `tiberius`, or the sync `postgres` crate, with `tokio` behind a feature
  flag.** Structurally refused: a second reactor beside ADR 0115's, on a runtime whose whole design is that
  a core is never blocked and a task never migrates. The feature flag makes it worse rather than better, by
  making the runtime's presence a build property.
- **A `dyn Driver` trait.** § 5.
- **One crate per driver** (`nvs-db-postgres`, …). Five crates that all depend on the same shared half, are
  never used independently, and reintroduce the feature matrix § 1 rejects. The audit argument that favours
  a separate `nvs-db` does not extend to splitting it five ways.
- **A second `rustls` session inside `nvs-db` for TDS**, instead of generalising `NvsTls`. Two answers to
  the trust-anchor question, one of which nobody would remember to update.
- **Busy state on `NvsStream`.** It would put protocol knowledge in a generic stream, and SQLite — which
  owes the same `LogicError` — has no stream to put it on.
- **Draining a poisoned wire and pooling it anyway.** § 4: it trusts framing that has already failed, to
  save one handshake on a path that is rare by construction.

## Verification

- **M8, per driver, against real servers**: `python tools/db-matrix.py` over `tests/db/compose.yaml`, which
  is [0067](0067-core-db.md)'s own *Verification* list pointed at five endpoints. That list is the one home
  for what is asserted; this ADR adds no fixtures of its own.
- **Codec-level cases with no server**, which is what sans-IO buys: a recorded byte stream decodes to the
  expected messages, and a truncated one leaves the connection `Poisoned` rather than `Idle`.
- **`Poisoned` is closed, not reset**: a connection whose response is truncated mid-message is destroyed at
  release, asserted by the pool's own test rather than by a driver's — the property is [0067
  § 13](0067-core-db.md)'s and holds for every driver.
- **The ledger gate**: `python tools/gen-attribution.py --check-c-deps` passes with `libsqlite3-sys`
  recorded and fails if a sixth C dependency appears.
