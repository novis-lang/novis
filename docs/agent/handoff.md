# Handoff

## State

**Stage 0 item 22 is two thirds landed, and § E's second bullet in `docs/perf/userland-gap.md`
is struck.** Reading a `string` argument is now a **tag check**: the unchecked read lives once,
behind `mwl_runtime::MwlStr::text_of` (`string.rs:@text_of`), with `Value::as_text`
(`value.rs:@as_text`) as the safe caller — the tag *is* ADR 0009's UTF-8 guarantee, and a debug
build re-validates inside that one reader so the invariant stays checked. `Core\Str`'s `text`
(`str.rs:568`) therefore validates nothing at its 56 call sites. The second half is
`crate::granularity`'s fast-path test (`granularity.rs:91`): an `is_ascii` scan plus a separate
search for `\r` are one branchless fold, so `Core\Str::length` makes one pass and then `len`.

**Measured as an A/B against the commit before this one** (both halves together; they were not
attributed separately): `05-string-replace` **0.72× → 0.91×**, `06-string-split-join`
**0.78× → 0.96×**, `07-string-normalize` **0.42× → 0.51×**, `04-string-format` work 92.9 ms →
82.0 ms. The suite median held at **0.69×** — those four rows crossed *over* the median rather
than lifting it. The ledger's suite table is a fresh full 9-rep sweep. Verify is green (**1583**
tests, 74 suites, clippy and fmt clean); no refcount edge changed, so no valgrind run was owed.

**`orient.py` still does not print `docs/perf/userland-gap.md`** — `[context]` in
`loop-goal.toml` has no field selecting a perf doc, and every remaining Stage 0 item cites a
section of it. This is the second session to pay for it.

## Next group

All of `crates/mwl-stdlib/src/`, and the two slices share the same five files: `str.rs`,
`bytes.rs`, `path.rs`, `regex.rs`, `uri.rs`. `docs/perf/userland-gap.md` § E's **first** bullet
is the specification, and when both land § E is struck whole and Stage 0 is done.

- [ ] **`produced` writes its result once.** § E's first bullet. A member builds a `String` and
      `produced` then allocates an `MwlStr` and copies it — `str.rs:639`, `bytes.rs:425`,
      `path.rs:434`, `regex.rs:678`, plus `uri.rs`'s own. Where the result length is known
      (`replace`, `padStart`/`padEnd`, `join`, `repeat`) the member can write straight into one
      `MwlStr`; `MwlStr::from_pieces` (`string.rs:235`) is the existing one-allocation seam and
      may be all that is needed. The guard § E asks for is that a `Core\Str` member allocates
      its result once.
- [ ] **The sibling `text()` helpers stop re-validating too.** Same change as this session's,
      copied into the modules that copied the helper: `path.rs:380`, `regex.rs:575`,
      `uri.rs:688`, `uuid.rs:215`, `validate.rs:258`, `csv.rs:248`, `json.rs:1017`,
      `time.rs:1514` — each is `std::str::from_utf8` over a `Tag::Str` argument, and
      `Value::as_text` is the whole replacement. **Three sites must keep validating** and are
      not this slice: `encoding.rs:455` and `:580` are ADR 0009 § 3's *checked*
      `bytes as string`, and `uri.rs:713` decodes percent-escapes into octets no tag vouches
      for.

## Backlog

- `Core\Out::capture` is the last key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`,
  and the last § 12 member — the plan's *Open now*.
- Item 19 still owes the append-only `docs/perf/history.ndjson` entry item 15 asked for.
- The ledger's `Core\Str::length` micro row (14.1 ns) predates this session and is now an upper
  bound; it is flagged as such under that table rather than re-measured.
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json` gap 2, now that a call-site type argument parses.
- Stage 4's own counts: conformance 436 of 600, differential 90 of 150.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s gap list.
