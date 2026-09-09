# Handoff

## State

**Goal 25 is green and its audit is closed.** `Core\Compress`, `Core\Mime` and `Core\Zip` are on disk and
registered, every stage's acceptance check passes, and `python tools/check-migration.py --min 100` now
reads 1167 functions and 255 types with none open.

**The oracle inventory carries `fileinfo` and `zip`.** How it is regenerated is
`tools/dump-php-builtins.php`'s own comment; the sixteen names that arrived with them are rows under
`docs/spec/02-php-migration.md` § *Archives* and § *Content types*, and `check-migration.py`'s `UNAUDITED`
is 23 extensions, every one either Tier 1 or answered by `Core`'s own.

**The last § 17 line is `Core\Xml`**, goal 26's, in
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`. The chain advances there and
`tools/loop.py` reseeds this file, so the group below is only what this tree owes if the run continues
here.

## Next group

**Stage 5 — the incremental half of `Core\Compress`, which nothing in the chain builds; both are one
file set: `crates/nvs-stdlib/src/compress.rs`, with the second reaching into
`docs/spec/02-php-migration.md`.**

- [ ] **`Core\Compress\Stream`, the incremental half**, which `deflate_init`, `deflate_add` and
      `inflate_init` are pointed at — `crates/nvs-stdlib/src/compress.rs:38` states its shape and that it
      carries the same `Bound` per chunk, because a bound applied per call rather than per stream is not
      a bound. `rule:core-classes/decompression-bound`. No chain entry builds it, so taking it is a
      scheduling question rather than a session's.
- [ ] **The four incremental rows name a member spelling once that object exists** —
      `docs/spec/02-php-migration.md:1078` is `deflate_init`'s row and the three after it are its family;
      each says `Core\Compress` in prose today because there is no `Core\Compress\Stream::` member to
      name, and `crates/nvs-stdlib/tests/spec_registry_coverage.rs`'s walk is what holds the spelling to
      the registry. `rule:php-migration/every-php-builtin-is-a-completion-candidate`.

## Backlog

- `Core\Xml` is goal 26 and the last § 17 line —
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
- The symlink half of `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it` runs
  only where the host creates links; passing under WSL, skipped with a reason on the Windows leg —
  `docs/agent/playbook.md` § *Writing a test case*.
- `Core\Process::spawn` and `Core\Metrics` stay unowned — `docs/agent/carried-gaps.md` § *Unowned*.
- `mbstring`, `curl`, `intl`, `gd` and 19 more are still off the inventory; each becomes rows the day a
  build that loads them regenerates it — `tools/check-migration.py`'s `UNAUDITED`.
