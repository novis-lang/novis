# Handoff

## State

**Goal 25 is green.** `Core\Compress`, `Core\Mime` and `Core\Zip` are on disk and registered, all five
stages' acceptance checks pass, and `python tools/verify.py` is 9 of 9 — 3762 unit, 1655 conformance,
276 differential.

**Stage 5 closed three ways.** The § 17 keys were struck from
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` by the slices that registered the
classes, so `Core\Xml` (goal 26) is the only § 17 line left; the migration table's zlib rows now answer
`Core\Compress::compress` and `::decompress` rather than naming the class in prose; and
`the_server_still_sets_no_content_encoding_of_its_own` is in `crates/nvs-server/src/serve.rs`, beside
the shipped-header case and asserting the absence over one connection for the same reason that one
does.

**The stage's check named a test no crate declares.** `every_migration_member_is_registered` was a
shorthand for `every_migration_member_row_names_a_registered_member`, which has carried that walk
throughout; `docs/agent/loop-goal.toml`'s stage 5 block now spells it as the tree does, the same
correction its stage 1 floor copy already carried.

**`fileinfo` and `zip` are on `tools/check-migration.py:61`'s `UNAUDITED` list**, so the migration
table can carry no `finfo_*` or `zip_*` row at all — a row for a name the inventory does not list is a
structural error, not an unfilled one. Stage 5's "the table's `zlib`, `fileinfo` and `zip` rows answer
a `Core` spelling" is therefore met for `zlib` and unreachable for the other two until the inventory is
regenerated against a build that loads them. That list's own comment is where the hole is named, and it
no longer calls `zip` a Tier 1 extension.

## Next group

**Stage 5 — goal 25 is green, so the chain advances to goal 26 and `tools/loop.py` reseeds this file;
these two are what this tree still owes if the run continues here** — one file set:
`tools/check-migration.py` and `docs/spec/02-php-migration.md`, with the second item alone in
`crates/nvs-stdlib/src/compress.rs`.

- [ ] **The PHP inventory is regenerated against a build that loads `fileinfo` and `zip`**, so the two
      classes this goal built are audited against PHP's own names instead of being absent from the
      table — `tools/check-migration.py:61` is the list they come off, `tools/dump-php-builtins.php`
      is what writes the inventory, and `docs/spec/02-php-migration.md:1727` is the paragraph that
      states the hole. Needs a PHP build carrying both extensions; `docs/setup.md` owns what this
      machine's PHP is. `rule:core-api/tier-roster`.
- [ ] **`Core\Compress\Stream`, the incremental half**, which `deflate_init`, `deflate_add` and
      `inflate_init` are pointed at — `crates/nvs-stdlib/src/compress.rs:38` states its shape and that
      it carries the same `Bound` per chunk, because a bound applied per call rather than per stream is
      not a bound. `rule:core-classes/decompression-bound`. No chain entry builds it, so it is a
      scheduling question rather than a session's.

## Backlog

- `Core\Xml` is goal 26 and the last § 17 line —
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
- The symlink half of `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it` runs
  only where the host creates links; passing under WSL, skipped with a reason on the Windows leg —
  `docs/agent/playbook.md` § *Writing a test case*.
- `Core\Process::spawn` and `Core\Metrics` stay unowned — `docs/agent/carried-gaps.md` § *Unowned*.
