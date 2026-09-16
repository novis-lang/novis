# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15`, goal-owned is down to 48 over 89 items**
(`python tools/owners.py`), `--deferrals` green. The 15 unowned are the scheduling questions and
none of them is this goal's own gap.

**§ 5's normalisation is built, and it is one function.** `crates/nvs-db/src/plan.rs:528`'s
`stored_as` writes a type with `ddl::column_type` and reads it back with `catalog::scalar_type`,
which is the round trip with the server left out, so every lossy case the two maps already document
is normalised by construction and a spelling either map grows is covered the day it is written.
`same_column` compares the folded types and is otherwise the plain equality it was; a step still
carries `want`'s own spelling. `crates/nvs-db/src/catalog.rs` gaps 1 and 2 and
`crates/nvs-db/src/ddl.rs` gap 1 are struck, built rather than deferred.

**SQL Server's assembled spelling carries `DATETIME_PRECISION`**, for `time`, `datetime2` and
`datetimeoffset` and for no other type: `date` and `datetime` report a precision they cannot be
declared with, and assembling `date(0)` would fail the whole read rather than describe a column
better. That list is a list in the `CASE`, not a non-null test.

**What proves it without a container**: every vocabulary type on every dialect, applied and read
back off its own emitter, is an empty plan (`catalog.rs`'s
`every_type_read_back_off_its_own_emitter_is_an_empty_plan`), and a `uint32` and a `bytes(64)`
applied to a real SQLite file come back as an `int64` and an unbounded `bytes` with the plan
still empty. The four matrix fixtures are unchanged, so no container leg moved.

## Next group

**Stage 5: the vocabulary's one read-only case** — one file set: `crates/nvs-db/src/schema.rs`,
`crates/nvs-db/src/catalog.rs`, `crates/nvs-db/src/ddl.rs`. Both slices are the same `Decided:`
sentence, the opaque default, and `rule:core-classes/schema-is-a-value` is what the vocabulary
sits inside.

- [ ] **Add the opaque, read-only `ColumnDefault` case, compared verbatim and never constructed**
      — `crates/nvs-db/src/schema.rs:320` is the enum and `crates/nvs-db/src/schema.rs:925` /
      `crates/nvs-db/src/schema.rs:938` are the surface half: `to_node` shows it in a dump and
      `from_node` refuses it, which is what "not constructible from a program" means in the one
      place a program could construct one. `crates/nvs-db/src/ddl.rs:510` (`literal`) is the
      emitter half — only a *read* side can hold the case and a step carries `want`'s, so the arm
      exists to be unreachable rather than to spell anything.
- [ ] **Narrow `unquote`'s whole-string fallback to MySQL and answer the other three with that
      case** — `crates/nvs-db/src/catalog.rs:844` (`unquote`) under
      `crates/nvs-db/src/catalog.rs:732` (`column_default`), striking the gap now at
      `crates/nvs-db/src/catalog.rs:78`. MySQL's `information_schema` is the one catalog that
      prints a literal unquoted; on the other three an unquoted spelling is an expression, and
      holding it verbatim is what makes the plan converge. `rule:core-classes/schema-introspection`.

## Backlog

- Widen `assembled_fixture` to a `uint` and a bounded `bytes` when a matrix run can be spent —
  `crates/nvs-db/src/catalog.rs`; its doc names the two cases that cover them without a server.
- `docs/agent/carried-gaps.md:191` is half stale: the fold exists now, and what is left in that
  entry is the SQL Server default-constraint name, the opaque default and the two constructs no
  backend takes portably.
- `crates/nvs-db/src/plan.rs` is outside `[context] modules` although the group's work lands in
  it; the driver sweeps it in from this session's commits, so nothing needs editing by hand.
- `crates/nvs-stdlib/src/path.rs` gaps 1 and 2 (UNC, drive-relative) are one file set for a later
  group — `docs/agent/carried-gaps.md`.
- `crates/nvs-stdlib/src/zip.rs` gaps 1 and 2 (Zip64, entry CRC) likewise.
