# Handoff

## State

**Goal `tds-bytes` — stages 2 and 3 have landed.** A `bytes` bound on SQL Server goes out as a
`varbinary` parameter: `crates/nvs-db/src/tds/rpc.rs:178`'s `Bound` is the form a bound value carries,
`declarations` gives a binary marker its own `varbinary` entry, and `binary_param`
(`crates/nvs-db/src/tds/rpc.rs:517`) writes the octets as themselves. `crates/nvs-db/src/tds/mod.rs`
owns no gap now and the `carried-gaps.md` § *Owned* row is struck.

The form travels from `encode` to `bind` as one `BINARY_MARK` octet in front of the value —
`crates/nvs-db/src/tds/rpc.rs:106-115` owns why `0xFF` makes the two forms disjoint rather than
conventional, and why `Encoder::Wire`'s shared `fn(Value) -> Option<Vec<u8>>` leaves no other channel.
Stages 2 and 3 landed as one commit on purpose: the goal's *Standing decisions* says the refusal is
closed in `encode` and on the wire together or in neither.

What is left is stage 4 alone — the round trip pinned on the scripted server and on a real one.

## Next group

**Stage 4: the round trip, scripted and real** — one file set: `crates/nvs-db/src/tds/plan.rs`,
`crates/nvs-db/src/tds/testing.rs`, `crates/nvs-db/tests/handshake.rs`.

- [ ] **A bound `bytes` comes back equal, over the scripted server** — a case beside
      `crates/nvs-db/src/tds/plan.rs:901` that binds a marked value and answers a `varbinary` column
      carrying the same octets, so the write half and `decode_column`'s read half are one assertion.
      `crates/nvs-db/src/tds/testing.rs:305`'s `binary_type` builds the column and `plp_value` the
      body. `rule:core-classes/db-column-types`.
- [ ] **The same claim against a real SQL Server** — `crates/nvs-db/tests/handshake.rs:474`'s `mssql()`
      gate and `mssql_run` at `crates/nvs-db/tests/handshake.rs:546`, run under `python
      tools/db-matrix.py --all`. Break the new case's own assertion once and re-run: the tool reports
      `ok` for a case that never ran (playbook § *Running things*). `rule:core-classes/db-one-api`.
- [ ] **Then the goal is met** — `python tools/verify.py --doc`, `python tools/owners.py --closes
      tds-bytes` and `python tools/playbook.py --closes tds-bytes` before `DONE`. The owner gate is
      already green: `crates/nvs-db/src/tds/mod.rs:86` is where gap 1 was.

## Backlog

- A `.nvst` case binding a `bytes` through `Core\Db` on every driver, if none exists — `python
  tools/gaps.py` ranks it; the other four drivers already bind one.
- `Core\Db::stream` and the schema half of § 9's binary row stay `gap-zero`'s — `docs/agent/carried-gaps.md`.
