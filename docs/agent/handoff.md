# Handoff

## State

**Goal 4's `check-migration --min 74` gate is green** — 926 of 1,152 inventory names classified, 80%, with
226 open. Two families landed this session, both in `docs/spec/02-php-migration.md`: the **XML** block (77
names, one `## XML` section covering `xml_parser_*` and its handler roster, SimpleXML, the `dom_import`
pair, `libxml_*` and the whole `xmlwriter_*` writer) and the **tail** (88 names as five sections —
networking and mail, errors and the log, `filter`, the terminal and the locale, and the program describing
itself).

What is left is **one domain**: `pg_*` (120) and `mysqli_*` (106), which
[ADR 0067](../adr/0067-core-db.md) owes an audited row each. `## Not yet classified` now names only that.

Nothing about § 15's stdlib half moved. `Core\Env` and `Core\Cap::has` are on disk, the compile-time half
of ADR 0112 is still absent, and `crates/nvs-stdlib/src/cap.rs`'s module doc owns what that costs.
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is still 41 keys, none of them this goal's.

## Next group

**All three slices are `docs/spec/02-php-migration.md` alone**, appending a `##` section before
`## Not yet classified` and trimming its closing paragraph when the last one lands.
[01 § 18](../spec/01-core-library.md) is the reference the cells name — it is a full signature roster, so
unlike `Core\Xml` these cells can name `Core\Db::…` members directly, and `check-migration.py` validates
every one of those spellings. Run it after each family, not at the end.

- [ ] **Migration rows: `mysqli_*`, the connection and statement half** (~55). `mysqli_connect`/`_init`/
      `_real_connect`/`_options`/`_ssl_set` against ADR 0067's connection naming and memoization and its
      root-owned `[db.<name>]` block, the `_prepare`/`_stmt_*` roster against "there is no `prepare`", and
      `_query`/`_real_query`/`_multi_query`/`_next_result` against the one query member.
      `docs/spec/02-php-migration.md:1375`, `docs/spec/01-core-library.md:1133`.
- [ ] **Migration rows: `mysqli_*`, the result and transaction half** (~51). `_fetch_*`, `_num_rows`,
      `_data_seek`, `_free_result` against the row iteration ADR 0053 gives, plus `_autocommit`,
      `_begin_transaction`, `_commit`, `_rollback` against ADR 0067's transaction shape, and the
      `_error`/`_errno`/`_sqlstate` trio against `Core\Db\DbError`. `docs/spec/02-php-migration.md:1375`,
      `docs/spec/01-core-library.md:1133`.
- [ ] **Migration rows: `pg_*`** (120), the largest single family left and worth splitting in two if the
      context runs short: connection, `pg_query*`/`pg_execute`/`pg_prepare` and the result roster first,
      then the `pg_copy_*`, `pg_lo_*` and `pg_send_*`/async tail — the last of which is where PDO's own
      shape and Novis's synchronous-over-parking-stream one diverge.
      `docs/spec/02-php-migration.md:1375`, `docs/spec/01-core-library.md:1133`.

## Backlog

- ADR 0112's compile-time half is still absent — `crates/nvs-stdlib/src/cap.rs`'s module doc.
- `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`, 41 keys.
- `Core\Xml`'s roster is one row in 01 § 17 (`docs/spec/01-core-library.md:1128`); 77 migration rows now
  point at a class whose members 01 does not spell, which is why they name the class and not `::member`.
- The unaudited extensions (`mbstring`, `curl`, `openssl`, … 25 of them) stay a known hole until the
  oracle build carries them — `docs/spec/02-php-migration.md` § *What the inventory is*.
- `docs/adr/0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md` § 11 owes one row
  per `gd`/`exif` name the day that build lands; four `standard` rows already point at it.
