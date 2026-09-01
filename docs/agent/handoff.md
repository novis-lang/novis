# Handoff

## State

**`examples/transaction.nvs` passes Stage 5's acceptance check whole** — `inserted 2`, `rolled back`,
`count=0`, `committed`, `count=2`. What was failing was `frontend::bind`'s argument order: its first
name is the *portal* and its second the statement, and both call sites in `crates/nvs-db/src/pg.rs`
passed them the other way round, so every batch bound the unnamed statement while its `Parse` created
`s0`. Every unit test passed anyway because they run on `no_cache()`, where the two names are the same
empty string; the playbook's bullet owns that trap.

**`examples/db.nvs` now stops at compile time, not in the protocol.** Three pieces of ADR 0067 § 18's
surface are unlanded: `foreach` over `Core\Db\Rows` (`E0443` — `Rows` is not `Iterable`),
`$write->affected` as a *property* where the registry carries a method row, and `queryAs<T>`. That is
the next group, and its acceptance want is `rows=3`, `ada`, `grace`, `alan`, `affected=1`,
`typed row ok`.

**The CA is still not in git.** It belongs to the `certs` volume and is remade with it, so a checkout
that has never brought the fixtures up has no `tests/db/ca.crt` and `nvs` refuses to boot from the repo
root until `tools/loop.py` copies it out — deliberate, and `nvs_host::tls`'s module doc owns why.

## Next group

**§ 18's remaining `Core\Db` surface, which is exactly what `examples/db.nvs` needs — the file set is
`crates/nvs-stdlib/src/db.rs` and `examples/db.nvs`.**

- [ ] **Make `Core\Db\Rows` a subject `foreach` accepts** — `crates/nvs-stdlib/src/db.rs:577` is the
      `ROWS` class and `crates/nvs-diagnostics/src/lib.rs:777` the `E0443` that refuses it today.
      ADR 0067 § 18's table wants `foreach ($rows as Row $row)`, and ADR 0053 § 3 names the three
      subjects a `foreach` takes, so this is `Rows` gaining one of them rather than a fourth.
- [ ] **Settle `Write`'s three readers against `examples/db.nvs:87`** — the `affected` row is
      `crates/nvs-stdlib/src/db.rs:832` and its helper `crates/nvs-stdlib/src/db.rs:3116`. ADR 0067
      § 18 lists `affected`, `changed` and `lastId` without saying method or property; decide it
      against ADR 0063's member shape and make the fixture and the registry agree either way.
- [ ] **`queryAs<T>`** — `crates/nvs-stdlib/src/db.rs:321` is the `CONNECTION` class and
      `crates/nvs-stdlib/src/db.rs:2279` `query`'s helper, which it hydrates over. ADR 0067 § 6 and
      § 18. It may want the same shape-parameter type `open` is waiting on — check that first, and if
      it does, say so in the handoff rather than opening the type.

## Backlog

- `columns()` on `Rows`, over a `ColumnType` enum — the plan's *Open now*.
- `open` waits on a shape-parameter type — ADR 0067 § 2.
- § 13's per-core pool is Stages 3 to 7 of the goal — ADR 0067 § 13.
- The other four drivers, against real servers — ADR 0067 § 8, the goal's Stage 6.
- `tests/db/ca.crt` is remade with the `certs` volume — `tools/loop.py`'s `[docker.copy]`.
