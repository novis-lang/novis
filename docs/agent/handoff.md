# Handoff

## State

**Goal 4's only open gate is `check-migration --min 74`, and it reads 56%** — 645 of 1,152 inventory
names classified, 507 open — after three families landed this session: files, directories and streams;
hashing, passwords and identifiers; output buffering and the process. 74% needs roughly **207 more rows**,
which is two or three sessions of the same work.

Nothing else about § 15's stdlib half moved. `Core\Env` and `Core\Cap::has` are on disk, the compile-time
half of ADR 0112 is still absent, and `crates/nvs-stdlib/src/cap.rs`'s module doc owns what that costs.
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is still 41 keys and none of them is this
goal's.

**The largest block left is not in the next group on purpose.** The four database extensions are 236
names — alone more than enough to clear the gate — and ADR 0067 owes each a row, which is a different
kind of pass than mapping a family onto a roster that already exists.

## Next group

**Every slice is `docs/spec/02-php-migration.md` alone.** Each maps one family onto one section of
`docs/spec/01-core-library.md`, appends a `##` section before `## Not yet classified`, and trims the
domain it closed out of that section's closing paragraph. `python tools/check-migration.py` reports the
open names and validates every `Core\X::y` spelling; run it after each family, not at the end.

- [ ] **Migration rows: reflection and the class API.** `class_exists`/`interface_exists`/`trait_exists`/
      `enum_exists`, `get_class` and the `get_class_*`/`get_object_vars` family, `method_exists`,
      `property_exists`, `is_a`, `is_subclass_of`, `class_implements`/`class_parents`/`class_uses`,
      `get_declared_*`, `spl_object_id`/`spl_object_hash`, the whole `spl_autoload_*` family against
      ADR 0061's compile-time discovery, `call_user_func*`/`forward_static_call*`/`func_get_args*`,
      `serialize`/`unserialize`, `var_dump`/`print_r`/`var_export`/`debug_zval_dump`, `debug_backtrace`,
      `token_get_all`/`token_name`, and `get_resource_*` against R14's "there is no `resource`".
      Roughly 55 names against 01 § 13's table. `docs/spec/02-php-migration.md:900`,
      `docs/spec/01-core-library.md:949`.
- [ ] **Migration rows: sessions, requests and headers.** The ~25 `session_*` names against
      `Core\Session`, `header`/`header_remove`/`headers_list`/`headers_sent`/`header_register_callback`,
      `setcookie`/`setrawcookie`, `http_response_code`, `http_*_last_response_headers`,
      `request_parse_body`, and `filter_input*`/`filter_var*` against `Core\Validate`'s validators-only
      roster. ADR 0012 is the reason most of them are `dropped` rather than renamed.
      `docs/spec/02-php-migration.md:900`, `docs/spec/01-core-library.md:1038`.
- [ ] **Migration rows: compression.** The ~20 `gz*` handle functions, `readgzfile`, `deflate_init`/
      `deflate_add`, `inflate_init`/`inflate_add`/`inflate_get_status`/`inflate_get_read_len`, and
      `zlib_encode`/`zlib_decode`/`zlib_get_coding_type` against 01 § 17's `Core\Compress` — one API for
      gzip, deflate, brotli and zstd, and no handle. `docs/spec/02-php-migration.md:900`,
      `docs/spec/01-core-library.md:1123`.

## Backlog

- The four database extensions: 236 names, ADR 0067 owes a row each — [docs/adr/0067-core-db.md](../adr/0067-core-db.md).
- XML: `xml_*`, `xmlwriter_*`, `simplexml_*`, `libxml_*` — ~90 names against 01 § 17's `Core\Xml`.
- Networking and DNS: `dns_*`, `gethostby*`, `checkdnsrr`, `getmxrr`, `inet_*`, `ip2long`/`long2ip`,
  `getprotoby*`/`getservby*`, `net_get_interfaces` — against `Core\Net`, 01 § 16.
- PHP's own introspection: `phpinfo`, `phpversion`, `phpcredits`, `get_loaded_extensions`,
  `get_defined_*`, `version_compare`, `assert*`, and the `error_*`/`trigger_error`/`set_error_handler`
  family against ADR 0020's ladder.
- The tail nobody owns yet: `pack`/`unpack`, `parse_ini_*`, `readline*`, `mail`, `syslog`/`openlog`/
  `closelog`, `get_browser`, `getimagesize*`, `iptc*`, `hebrev`, `highlight_*`.
- The compile-time half of ADR 0112 — § 1's per-namespace `[grants]` table and § 4's `E0604` —
  `crates/nvs-stdlib/src/cap.rs`'s module doc.
