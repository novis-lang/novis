# Handoff

## State

**M8 goal 5, stage 8. A `[db.<name>] path` is resolved against the directory of the file that
wrote it** (ADR 0103 § 5), one `origins` lookup beside the `tls_ca_file` that already did it, in
`crates/nvs-config/src/db.rs`'s `canonicalize`. `SqliteTarget::path`'s doc claim is now true, and
the asymmetry it rests on is stated on both sides: a program-supplied `Db\Settings.path` stays
relative to the process, because resolving a string the program computed against a configuration
file it never named would make its meaning depend on where the operator keeps `nvs.toml`.

**That fix uncovered a real one.** `Snapshot::retype` deserializes `Config` out of the merged
*table*, so `canonicalize`'s rewrite of `resolved.config` never reached a driver — including the
`tls_ca_file` it had been doing all along, which was trust-checked at the resolved path and then
opened at the written fragment. Both keys are now written back into the table as well, by
`db::rewrite`, whose doc owns the reasoning; the playbook bullet is the general shape.

**What is resolved is a *relative file*, and `db::is_relative_file` is the whole of that rule.**
`:memory:`, the empty string and a `file:` URI are SQLite spellings rather than paths, and a
rooted path is not relative even where Windows says it is not absolute either. Both halves are
pinned in `crates/nvs-config/tests/resolve.rs`.

**`nvs_types::intrinsics`' known gap 6 is two obstacles, not three.** `Core\Db::open` has a
registry row since ADR 0135's shape parameter landed. What stands: `Intrinsic` addresses a
written argument position while § 18 puts the host inside a shape literal, and checking has no
capability set in front of it at all (`crate::Env` carries none).

The standing acceptance failure, stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`, is
gated on both of those and stays open — the gap now says so by name rather than leaving a reader
to re-derive it.

## Next group

**One file set: `crates/nvs-config/src/snapshot.rs`** with `crates/nvs-config/tests/resolve.rs`,
and `crates/nvs-config/src/app.rs` for the third.

- [ ] **Audit the other three `resolve()` passes for the table/tree split the `[db]` one had.**
      `crates/nvs-config/src/resolve.rs:298` (`secret::materialize`, which solves it a third way
      — beside the table, re-applied by `retype`), `crates/nvs-config/src/resolve.rs:313`
      (`app::canonicalize`, which is safe only because `Snapshot::build` reads
      `resolved.config.app` directly), `crates/nvs-config/src/snapshot.rs:203` (`retype`, the
      seam). Three mechanisms for one question is the finding to write down or reduce; say which
      in the module doc rather than leaving the next reader to diff them.
- [ ] **A case pinning that a relative `tls_ca_file` reaches the driver resolved**, which is the
      half of this session's bug fix that nothing yet asserts end to end.
      `crates/nvs-config/tests/resolve.rs:361` (`a_db_blocks_path_resolves_against_the_file_it_is_written_in`,
      the shape to copy — it asserts the table as well as the tree, and that second assertion is
      the one that would have caught this) and `crates/nvs-config/src/db.rs:97` (`rewrite`).
- [ ] **Say in `Snapshot::table`'s own doc that it is the half a driver reads.**
      `crates/nvs-config/src/snapshot.rs:63`. Its doc today explains why the table is *kept*
      (§ 9's `dump --origin`); what cost this session time is that it is also what `config` is
      rebuilt from, which the field's doc never says and `retype`'s says only in passing.

## Backlog

- `nvs config dump` prints the resolved tree now for these two keys; no case asserts it —
  `docs/adr/0103-configuration-is-a-tree-of-files.md` § 9.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` needs a capability set on
  `nvs_types::Env` — `crates/nvs-types/src/intrinsics.rs` known gap 6.
- ADR 0067 § 10's unterminated-literal disagreement between `nvs_types::intrinsics` and
  `nvs_db::sql` — same list, gap 5.
- `Core\Db::open`'s host has no check-time grant test; gap 6's first obstacle owns why.
