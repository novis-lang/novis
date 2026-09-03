# Handoff

## State

**M8 goal 5, stage 8. SQLite reaches every member of ADR 0067 the other four drivers do**, § 7
included: it is [`Transacting`]'s fifth arm in `crates/nvs-stdlib/src/db.rs`, over
`nvs_db::sqlite`'s own `begin`/`commit`/`roll_back`, and `transacting` is therefore **total**.
Known gap 2 is closed and its machinery is *deleted* rather than left with an empty roster to
render — `driverless`, `named_drivers` and `HAS_A_DRIVER` are gone, `Filed` is down to one field,
and that module's known-gap list item 2 argues the closure in full. The commands, the depth
accounting and the `SAVEPOINT` naming were already `nvs_db::sqlite`'s and did not move.

**§ 7's `{retries: n}` rests on § 8's kind, and that is now pinned rather than asserted in a doc.**
`crates/nvs-db/src/sqlite.rs`'s `a_lock_another_connection_holds_is_section_8s_deadlock_kind` opens
two handles on one `mode=memory&cache=shared` URI and takes the table with one of them; the loser's
refusal normalises to `Deadlock`, which is one of the two kinds a retry re-runs. A shared-cache
conflict is `SQLITE_LOCKED` where a file conflict is `SQLITE_BUSY` and § 8 maps both, which is why
the test asserts the kind and not the extended code.

**§ 9's map is now asked about every row it can reach on this backend**: the declared-type case
covers `blob` (a storage class SQLite really has, so no declaration rescues it) and `time` (which
only a declaration can reach, the cell being `TEXT` beside three other `TEXT` columns).

**Still open: a `[db.<name>] path` is not resolved against the config file's directory.**
`nvs_config::db` resolves `tls_ca_file` and nothing else, while `SqliteTarget::path`'s doc claims
ADR 0103 § 5's resolution "has already happened". One of the two is wrong and it is the tree.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`
and it is still not this goal's to close — but one third of `nvs_types::intrinsics`' known gap 6 has
gone stale under it; see the backlog.

## Next group

**One file set: `crates/nvs-config/src/db.rs`**, with `crates/nvs-db/src/sqlite.rs:82`
(`SqliteTarget::path`, whose doc is the claim being made true) and
`tests/conformance/core/` for the third.

- [ ] **A relative `[db.<name>] path` resolves against the config file's directory** (ADR 0103 § 5),
      which is one `origins` lookup beside the one `tls_ca_file` already does.
      `crates/nvs-config/src/db.rs:63` (the `tls_ca_file` loop this joins),
      `crates/nvs-db/src/sqlite.rs:82` (`SqliteTarget::path`'s doc claim),
      `crates/nvs-stdlib/src/db.rs:3469` (`sqlite_settings`, the `open` arm, where a
      program-supplied path is a sink and stays relative to the process instead — that asymmetry is
      the decision to state, not to remove).
- [ ] **A case pinning it**: a multi-file case whose `nvs.toml` names a bare file name opens the
      database beside the config file rather than beside the process's working directory.
      `tests/conformance/core/db-a-sqlite-block-opens-a-file-and-runs-statements-over-it.nvst:1` is
      the nearest shape and `crates/nvs-config/src/db.rs:46` is the resolver it has to prove, and
      the playbook's bullet on a multi-file case's working directory is the trap this one walks
      into.
- [ ] **Retire the stale third of `nvs_types::intrinsics`' known gap 6**, whose first obstacle
      ("`Core\Db::open` has no registry row at all") stopped being true when ADR 0135's shape
      parameter landed. `crates/nvs-types/src/intrinsics.rs:69`. The other two obstacles stand, so
      stage 9's check keeps failing — say so in the gap rather than leaving a reader to re-derive
      which third moved.

## Backlog

- `Core\Db::open`'s host-against-`db.open`-grants check needs a capability set at check time —
  `nvs_types::intrinsics` known gap 6, third obstacle. It is an ADR-shaped question (`crate::Env`
  carries no capabilities, and no capability is checked at check time at all), not a missing test.
- `crate::queue`'s four members are still PostgreSQL-only — `crates/nvs-stdlib/src/db.rs`'s
  known-gap list item 2 names it, `crates/nvs-stdlib/src/queue.rs` owns it.
- An `open` pool's bounds are the defaults with no spelling for an operator — that module's known
  gap 1.
- Stage 10's `nvs-suite` `cases` list still names `tests/conformance/db/…` paths in a directory
  layout the corpus never adopted — `docs/agent/loop-goal.toml`, and the playbook bullet that
  describes exactly this.
