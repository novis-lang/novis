# Handoff

## State

**Goal 4, M8.** The driver's failing acceptance check is the last gate the goal has open:
`differential`'s `min_passing = 250` (`docs/agent/loop-goal.toml:2630`). It is **not a regression** —
nothing fails, the count is the item, and the work is writing cases.

**The suite is now 231 passing, 0 failing — 19 short.** This session took spec § 12 whole: ten cases over
`Core\Csv`, `Core\Out`, `Core\Uri` and `Core\Validate`, seven agreeing with PHP outright and three
`--ORACLE-DIVERGES--` findings the sweeps turned up rather than predicted. The three divergences, each
now pinned in its own file: `{escape:}` is consumed by Novis where `str_getcsv` keeps both bytes and only
declines to end the field; `parse` answers a record *count* where `str_getcsv` answers one record for any
string including the empty one; and `isEmail`/`isDomain` refuse `user@[127.0.0.1]` and `example.com.`,
which `filter_var` accepts.

`python tools/gaps.py`'s *differential gap* list still holds one member with a PHP twin and no oracle file
at all — `Core\Task::afterResponse` against `fastcgi_finish_request` — so the remaining 19 are depth over
members that already have a twin, and the group below ranks the four with the most untouched PHP surface.

## Next group

**All four slices are new files under `tests/differential/core/`**, sharing the same two reads apiece: the
spec section holding the member's *Replaces* column, and its owning `nvs-stdlib` module for the option
names. `--ORACLE--`, never `--EXPECT--`, and never in `tests/conformance/` (`docs/agent/conventions.md`).
Write the whole sweep as one oracle case and run it with `target/debug/nvs.exe test <file>` — the runner
prints both sides aligned and tells you which rows are a divergence case instead.

- [ ] **`Core\Json::encode` against `json_encode`** (~3 cases). § 6 has a decode case and no encode one:
      the escaping of `/`, `<`, `>` and `&`, non-ASCII against `JSON_UNESCAPED_UNICODE`, float rendering,
      an empty array against `[]`/`{}`, and the pretty-printed spelling.
      `crates/nvs-stdlib/src/json.rs:145`.
- [ ] **`Core\Path` against `pathinfo` and `realpath`'s lexical half** (~3 cases). § 8 is pure string
      algebra, so every member has a twin that needs no disk: `basename`, `dirname`, `extension` and
      `withExtension` against `pathinfo`'s four keys, and `normalize` against the dot-segment removal
      `realpath` does before it touches the filesystem. `crates/nvs-stdlib/src/path.rs:91`.
- [ ] **`Core\Uri::resolve` and `$uri->compareTo` against the hand-written PHP** (~2 cases). Neither has a
      built-in twin, so the oracle is RFC 3986 § 5.4's own reference-resolution table written out in PHP,
      and § 6.2.2's folding compared against a `parse_url` + `strtolower` + `rawurldecode` reassembly —
      which is the code every PHP program comparing two URLs actually contains.
      `crates/nvs-stdlib/src/uri.rs:563`.
- [ ] **`Core\Validate::isAscii`/`isPrintable`, and `Core\Csv::format`'s whitespace quoting** (~2 cases).
      The two validators have no `filter_var` twin — `mb_check_encoding($s, "ASCII")` and `ctype_print`
      are theirs, and `ctype_print("")` is `false` where `isPrintable("")` is likely not, so expect a
      divergence file. `fputcsv` quotes a field holding a space or a tab and Novis does not, which is one
      more. `crates/nvs-stdlib/src/validate.rs:215`, `crates/nvs-stdlib/src/csv.rs:1`.

## Backlog

- `Core\Task::afterResponse` against `fastcgi_finish_request` — the last *differential gap* member,
  `python tools/gaps.py`.
- The migration table's remaining `pg_*` family — `python tools/check-migration.py --min 74` passes at
  90%, so it gates nothing (`docs/spec/02-php-migration.md`).
- `Core\Uri::parseQuery`'s tree is only asserted through the `buildQuery` round trip; rendering it
  directly needs a runtime string-or-array test (`docs/spec/01-core-library.md` § 12).
- `Core\Out::capture`'s `{through:}` has no oracle beyond ordering, because `ob_start($callback)`'s
  invisible pass-through is the thing spec § 12 declines to have.
