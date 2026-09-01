# Handoff

## State

**Goal 4, M8.** The driver's failing acceptance check is the last gate the goal has open:
`differential`'s `min_passing = 250` (`docs/agent/loop-goal.toml:2630`), against 210 case files that all
passed. It is **not a regression** — nothing fails, the count is the item, and the work is writing cases.
`python tools/check-migration.py --min 74` passes at 90%, so the migration table's remaining `pg_*` family
gates nothing and is backlog rather than the next group.

**The suite is now 221 passing, 0 failing — 29 short.** This session added 11 oracle cases over the three
classes that had a PHP twin and no differential file at all: `Core\Hash` (the fourteen-algorithm roster
against `hash()`, the two CRC-32 polynomials against `crc32b`/`crc32c`, `Hash\Stream` against
`hash_init`/`hash_update`/`hash_final`, `hmac` against `hash_hmac` over RFC 4231), `Core\Encoding` (base64,
base64url against the `strtr` idiom, hex) and `Core\Bytes` (addressing, search, construction, ordering).

`python tools/gaps.py`'s *differential gap* list is down to **one** member — `Core\Task::afterResponse`
against `fastcgi_finish_request` — so the remaining 29 are depth over members that already have a twin.
The group below names the four families with the most untouched PHP surface, ranked by how much of it PHP
computes for free.

## Next group

**All four slices are new files under `tests/differential/core/`**, and they share the two things this
session's did: the spec section that names the member's *Replaces* column, and PHP as the expectation, so
nothing is frozen by hand. `--ORACLE--`, never `--EXPECT--`, and never in `tests/conformance/`
(`docs/agent/conventions.md`). Run each with `target/debug/nvs.exe test <file>` as you write it — the
compile errors are cheap and the oracle mismatch prints both sides.

- [ ] **`Core\Csv` and `Core\Out` against `str_getcsv`, `fputcsv` and the `ob_*` family** (~4 cases).
      § 12's second and third tables: quoting, embedded separators and newlines, the empty field and the
      empty document for `Csv`; nesting, `Out`'s answer at each depth and what a discarded level does for
      `Out`. `docs/spec/01-core-library.md:846`, `crates/nvs-stdlib/src/csv.rs:1`.
- [ ] **`Core\Uri` against `parse_url`, and `Core\Validate` against `filter_var`** (~4 cases). § 12's
      first table and its prose roster — the six genuine validators are the whole of what survives
      `filter`, and `filter_var`'s own `FILTER_VALIDATE_*` half is the oracle for each.
      `docs/spec/01-core-library.md:846`.
- [ ] **`Core\Json::encode` against `json_encode`** (~3 cases). § 6 has a decode case and no encode one:
      escaping, the object/list distinction over an array with a hole, and the depth and float spellings
      where the two libraries have to agree. `docs/spec/01-core-library.md:583`.
- [ ] **`Core\Path` against `pathinfo` and `realpath`'s lexical half** (~2 cases). § 8 already has
      `basename`, `dirname` and a hand-written normalize; the extension and the join half are untouched.
      `docs/spec/01-core-library.md:664`.

## Backlog

- `pg_*`'s 120 migration rows, the previous group — `docs/spec/02-php-migration.md:1528`, three slices,
  now backlog because `check-migration` passes at 90% against a 74% gate.
- `Core\Task::afterResponse` against `fastcgi_finish_request`, the one member `gaps.py` still lists as a
  twin with no oracle case — `crates/nvs-stdlib/src/task.rs:561`.
- `Core\Time`'s remaining twins: `date`'s format letters past what `a-date-format-string-renders-as-phps-does`
  covers — `docs/spec/01-core-library.md:430`.
- ADR 0112's compile-time half of § 15 is still absent — `crates/nvs-stdlib/src/cap.rs`'s module doc.
- `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is still 41 keys.
