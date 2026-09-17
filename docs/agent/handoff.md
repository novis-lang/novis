# Handoff

## State

**Goal `tds-bytes` is met.** A `bytes` bound on SQL Server goes out as a `varbinary` parameter and
comes back equal: `crates/nvs-db/src/tds/rpc.rs:178`'s `Bound` is the form a bound value carries,
`binary_param` (`crates/nvs-db/src/tds/rpc.rs:517`) writes the octets as themselves, and
`crates/nvs-db/src/tds/plan.rs:954`'s scripted case asserts the write half and `decode_column`'s read
half as one equality. The real-server half is `a_mssql_bytes_binds_as_a_varbinary_and_comes_back_equal`
in `crates/nvs-db/tests/handshake.rs:1361`, confirmed to run by breaking its assertion and watching
the `mssql` leg name it.

Stages 2 and 3 landed last session under names no check named, so the acceptance list read `did not
run` over green work. The goal's checks now name the tests the tree holds, in both toml copies; the
one drafted name that was real work — the encoder's refusal for the tags with no form — is
`an_array_and_a_non_finite_float_are_still_refused_by_the_tds_encoder`, and it names no object
because `NvsObj::new` is `unsafe` and `nvs-db` forbids it.

`python tools/db-matrix.py --all` is 8/8 legs green, and the owner and playbook gates report the goal
owns no gap and no carried row.

## Next group

**Follow-on, unscheduled — one file set: `crates/nvs-stdlib/src/db/pool.rs`,
`crates/nvs-db/src/tds/rpc.rs`.**

- [ ] **No case binds a `bytes` through `Core\Db` on SQL Server** —
      `crates/nvs-stdlib/src/db/pool.rs:220` is where the driver picks `nvs_db::tds::encode`, and
      every case for the new form sits at the `TdsConn` surface below it, so the one line that would
      hand this driver PostgreSQL's renderer is unasserted. `rule:core-classes/db-one-api`.
- [ ] **The form travels in band, and a hand-written parameter can claim it** —
      `crates/nvs-db/src/tds/rpc.rs:106` owns why `BINARY_MARK` is one octet in front of the value
      rather than a typed parameter list, which is `Encoder::Wire`'s shared
      `fn(Value) -> Option<Vec<u8>>` and not this driver's choice. Widening that signature is the
      repair, and it is five drivers' seam. `rule:core-classes/db-column-types`.

## Backlog

- `Core\Db::stream` and the schema half on SQL Server — `gap-zero`'s register names their owners.
- The `nvs/rest` package is the only unscheduled half of the REST/OAuth client plan.
