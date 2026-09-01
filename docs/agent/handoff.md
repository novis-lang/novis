# Handoff

## State

**Goal 4's only open gate is `check-migration --min 74`, and it now reads 66%** — 761 of 1,152 inventory
names classified, 391 open — after three families landed this session: reflection and the class API (52
rows), sessions, requests and headers (35, plus the `ob_clean` row the output-buffering pass had missed),
and compression (29).

What is left is two blocks and a tail. The **XML family** is ~77 names (`xmlwriter_*` 42, `xml_parser_*`
and its handler roster 22, `libxml_*` 8, `simplexml_*` 3, `dom_import_simplexml`). The **four database
extensions** are 226 (`pg_*` 120, `mysqli_*` 106), which ADR 0067 owes an audited row each. The tail is
~88 of PHP's own introspection — error handling, `readline_*`, `filter_*`, `phpinfo` and friends. **XML
plus the tail clears 74% without opening the database block**, which is why the group below is those two.

Nothing else about § 15's stdlib half moved. `Core\Env` and `Core\Cap::has` are on disk, the compile-time
half of ADR 0112 is still absent, and `crates/nvs-stdlib/src/cap.rs`'s module doc owns what that costs.
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is still 41 keys and none of them is this
goal's.

## Next group

**Both slices are `docs/spec/02-php-migration.md` alone.** Each maps one family onto a section of
`docs/spec/01-core-library.md`, appends a `##` section before `## Not yet classified`, and trims the domain
it closed out of that section's closing paragraph. `python tools/check-migration.py` reports the open names
and validates every `Core\X::y` spelling against 01 — run it after each family, not at the end, and read
the playbook's *Tooling* bullet on what that validation actually reads.

- [ ] **Migration rows: XML.** The ~77 names of six PHP APIs for one job: `xml_parser_create`/`_create_ns`
      and the eleven `xml_set_*_handler` callbacks, `xml_parse`/`xml_parse_into_struct`, the
      `xml_get_current_*` and `xml_error_*` pair, `simplexml_load_string`/`_load_file`/`_import_dom`,
      `dom_import_simplexml`, the whole `xmlwriter_*` roster, and `libxml_use_internal_errors`/
      `_get_errors`/`_clear_errors` plus the four entity-loader and stream-context functions — the last of
      which is where XXE and ADR 0052's closed doors meet. 01 § 17's `Core\Xml` row states the one rule
      that shapes the section: the tree API and the streaming reader/writer are different jobs, not twins,
      and no operation is available through both. `docs/spec/02-php-migration.md:1087`,
      `docs/spec/01-core-library.md:1128`.
- [ ] **Migration rows: PHP's own introspection and the tail.** The ~88 left once XML is gone:
      `error_reporting`/`error_get_last`/`error_clear_last`/`error_log`/`trigger_error` and the
      `set_*_handler`/`restore_*_handler` pairs against ADR 0020's ladder and `Core\Log`; `filter_*` (7)
      against `Core\Validate`; `readline_*` (8) against `Core\Cli`; `get_defined_*`,
      `get_loaded_extensions`, `get_included_files`, `phpinfo`/`phpversion`/`phpcredits`,
      `constant`/`define`/`defined`, `assert`/`assert_options`, `inet_*`, `highlight_*`, `setlocale`,
      `syslog`. `docs/spec/02-php-migration.md:1087`, `docs/spec/01-core-library.md:1116`.

## Backlog

- The four database extensions: 226 `pg_*`/`mysqli_*` names, one audited row each — ADR 0067, 01 § 18.
- `Core\Compress` has no member roster in 01 § 17, so this session's compression rows name the class and
  describe the shape in prose — 01 § 17 owns fixing that.
- ADR 0112's compile-time half is absent; `crates/nvs-stdlib/src/cap.rs`'s module doc owns the cost.
- `spec-members-part-two-outstanding.txt` is 41 keys, none of them this goal's.
- Networking's `socket_*`/`stream_socket_*` roster is in the unaudited-extension hole, not the open list.
