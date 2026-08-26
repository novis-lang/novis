# Handoff

## State

**Conformance is at 545 committed of 600, and it is the only frontier left.** Verify is green (1597
cargo tests, 74 suites, 546 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt`
trees itself, so after a green `verify.py` there is nothing else to run (playbook, *Running things*).

This session was **user-fired and is not loop work**: a design question about uploads that turned into
one ADR slice. No library code, no conformance case. **[ADR 0105](../adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
replaces [ADR 0097](../adr/0097-development-server-and-proxied-origin.md) § 8's two body paths with
one:** `Core\Request::files(): Iterable<Part>` is now the only way to receive an uploaded file, the
buffered array is withdrawn, and a part is consumed by `readAll({max?})`, by iterating
`content: Iterable<bytes>`, or by `saveTo`, which delegates to a new `Core\IO::writeStream`. `[limits]`
gains a second row: `request_body` (`"8M"`/`"64M"`) now means bytes parsed into memory, `upload_total`
(`"256M"`/`"2G"`) bounds a streamed multipart body and is enforced on the wire, refused pre-dispatch
when `Content-Length` already exceeds it. There is still no temp file. All of it is M7's to build; none
of it exists in `crates/`.

The reason the buffered path went, recorded here because it is a fact about this runtime rather than
about HTTP: a `bytes` value is a `StrHeader` with its payload allocated inline
(`crates/mwl-runtime/src/string.rs:127`) and there is **no slice-of-parent representation**, so a
multipart part could never alias the accumulated body buffer — it had to be copied out of it, making the
buffered path cost twice what its cap said.

**Two conformance cases are on disk untracked and are not this session's** —
`tests/conformance/core/time-date-members-undo-each-other-over-one-table.mwlt` and
`time-of-day-wraps-at-midnight-and-orders-by-the-same-clock.mwlt`, which are the first two slices of the
*Next group* below. They are left exactly as found: unstaged, unjudged, uncommitted. A session picking
that group up should read them before writing anything, because they may be finished work that only
needs committing. This is also why the plan's conformance counts were **not** advanced — the number on
disk (547 files) is not a number anyone has committed to.

## Next group

Unchanged by this session — three slices, **all on `crates/mwl-stdlib/src/time.rs`** and all writing a
new file under `tests/conformance/core/`, so a session taking two pays for the file set once.
`docs/spec/01-core-library.md` § 4 owns the `Time` rules. `Core\Time\Date` is the thinnest class left at
0.33 cases per member; `TimeOfDay` and `Duration` are next at 0.67 and 0.37. **Check the two untracked
files named in *State* first — the first two slices may already be written.**

- [ ] **`Core\Time\Date` is six members over one civil date, and they undo each other** — `at` and
      `format` are inverses over a table of dates, `plus`/`minus` in the same unit return the date
      they started from (except where a month step clamps, which is the boundary to name on both
      sides), `with` is the identity when handed the parts `format` just read, and `compareTo` agrees
      with the calendar order of the same table. `crates/mwl-stdlib/src/time.rs:2664` (`at`), `:2706`
      (`plus`), `:2713` (`minus`), `:2725` (`with`), `:2756` (`compareTo`).
- [ ] **`Core\Time\TimeOfDay` is a clock that wraps, and says so at both ends** — `plus`/`minus` wrap
      around midnight rather than carrying a date, so a sweep of offsets from a fixed time is
      `(minute-of-day + n) mod 1440` on every row, and `compareTo` orders by that same number.
      `crates/mwl-stdlib/src/time.rs:2860` (`at`), `:2905` (`plus`), `:2920` (`with`).
- [ ] **`Core\Time\Duration`'s constructors and its `to…` readers are one scale** — every unit
      constructor times its own factor is the same nanosecond count, `multipliedBy` and `negated`
      compose, and `parse` reads back what `toString` wrote for a table spanning both signs.
      `crates/mwl-stdlib/src/time.rs:473` (`parse`), `:545` (`multipliedBy`).

## Backlog

- The two untracked `.mwlt` files above: read, verify, commit or delete. Nobody owns them right now.
- `Core\Uri` is 0.53 cases per member over 19 — the thinnest class outside `Core\Time`
  (`python tools/gaps.py --coverage`).
- `gaps.py --differential` still names 9 members with a PHP twin and no oracle case, all `Core\Time`
  and `Core\Encoding`; those belong in `tests/differential/`, not here.
- `Core\Json::decodeAs<T>` still decodes a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`) — its
  `$_FILES` row is unwritten, and ADR 0105 is what it now maps to.
