# Handoff

## State

**Stage 2's crate half is on disk.** `crates/nvs-db` exists and is green:
[ADR 0132](../adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md) § 5's `Connection`
enum with all five variants, § 4's four-state busy field as a `Cell` on each driver's own
connection, and the `NVS_DB_MATRIX_*` reader. Its module doc is the home of both the state machine
and the skip rule (a case that finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing, so `verify.py`
stays green with no containers); `tools/db-matrix.py` states the harness half and defers to it.
§ 1's edges are wired both ways — `nvs-stdlib` depends on `nvs-db` and each manifest carries the
no-cycle rule beside the edge.

**`NvsTls` is generic over its transport** — `NvsTls<T: Read + Write = NvsTcp>`, ADR 0132 § 3. The
default is what keeps the bare spelling working unchanged in `Core\Http\Client` and `Core\Mail`; the
deadline and the peer address stayed behind on `NvsTls<NvsTcp>`, because they belong to the socket
rather than to the session. § 3's body was folded to say "default type parameter" where it had said
"alias", which is the mechanism that landed and the cheaper one — no second name to keep in step.

**No wire code exists yet**, so the goal's three fixtures stay red at `E0405` and stage 6's matrix
check stays red. That is the ordinary state of this goal and not a regression. `rusqlite` has a
version in the workspace table but is deliberately a dependency of nothing: ADR 0132 § 2 requires
the slice that first takes it to add its `tools/gen-attribution.py` `C_DEPENDENCIES` row in the same
commit, and that gate fails on a recorded crate absent from the graph as well as on the reverse.

**Manifest gap:** `[context] adrs` in `docs/agent/loop-goal.toml` names no section of ADR 0132, so
the pack printed 0067 §§ 1/9/13 for an item specified entirely by 0132 §§ 1/3/4/5. That cost two
peeks. Add 0132 §§ 1-5 to that field; `[context] modules` also still globs
`crates/nvs-db/src/*.rs`, which now matches and needs no change.

## Next group

**The first PostgreSQL connection** — loop-goal Stage 2 item 3, specified by ADR 0132 §§ 2 and 3 and
ADR 0067 §§ 1 and 5. One file set: `crates/nvs-db/src/` and that crate's own `Cargo.toml`. Nothing
outside the new crate is touched, which is what makes these three one group.

- [ ] **`PgConn` opens, upgrades and authenticates** — ADR 0132 §§ 2 and 3: `SSLRequest` over the
      plaintext socket, the one-byte answer, `NvsTls::over` on the same `NvsTcp`, then SASL.
      `postgres-protocol` frames it and the sequencing is written here; it is already a version in
      the workspace table and needs only `postgres-protocol.workspace = true`. Anchors:
      `crates/nvs-db/src/conn.rs:158`, `crates/nvs-db/Cargo.toml:30`,
      `crates/nvs-host/src/tls.rs:174`.
- [ ] **The extended-query state machine moves `State` through its four values** — ADR 0132 § 4:
      `Parse`/`Bind`/`Execute` leave the connection `Streaming`, a `Sync` after closing the portal
      returns it to `Idle`, and a decode failure poisons it rather than draining a length prefix that
      has already proven untrustworthy. Anchors: `crates/nvs-db/src/conn.rs:118`,
      `crates/nvs-db/src/conn.rs:235`.
- [ ] **The `?`/`:name` rewriter and `inList` expansion** — ADR 0067 § 5, and the shared half ADR
      0132 § 5 keeps as plain functions with no driver in them, so it lands beside the drivers rather
      than inside one. Anchors: `crates/nvs-db/src/lib.rs:110`.

## Backlog

- `rusqlite` plus its `C_DEPENDENCIES` row, one commit — ADR 0132 § 2.
- The statement cache keyed by SQL text plus expansion arity — ADR 0067 § 1.
- The per-core pool and its reset-as-a-boundary — ADR 0067 § 13.
- MySQL, MariaDB and the hand-written TDS 7.4 driver — ADR 0132 § 2's table.
- `Core\Db`'s registry rows, cards and helper bodies in `nvs-stdlib` — ADR 0063's five edits.
- The `[context] adrs` gap above, in `docs/agent/loop-goal.toml`.
