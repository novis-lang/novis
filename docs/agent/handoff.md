# Handoff

## State

**M4's Stage 8, and the depth worklist itself was the bug.** The tree is at **772 conformance plus
189 differential**. `python tools/gaps.py --coverage` now ranks by the **median cases per member**
rather than by case-files ÷ members, prints each class's `FLOOR` (its worst member) and names the
three members it asks least, with anchors — the old quotient ranked by class size and sent the
previous session at a `Core\Math` group whose three claims were all already on disk (playbook,
*Tooling*). Nothing is blocked.

- The re-ranked frontier is `Core\Bytes`, `Core\Test`, `Core\Time` and `Core\Encoding` at a median
  of 2, each with members carrying a single case.
- One of those closed here: `Core\Bytes`'s three predicates now have an *agreement* case beside
  their coverage one, asserting over all 64 ordered pairs of a table that `contains` is
  `indexOf(...) != null`, that `startsWith`/`endsWith` are the two windows `slice` cuts compared by
  `compare`, and that `compare` is antisymmetric with a zero exactly at mutual prefixes. The three
  true-tallies (20/17/16) are what stops a predicate stuck on one answer from agreeing with a
  derivation stuck with it.

## Next group

**`Core\Time`'s thinnest three, and `Core\Test`'s assertion members** — the file set is
`crates/nvs-stdlib/src/time.rs` plus `tests/conformance/core/`, with `test.rs` for the third.
Take the members `gaps.py` names, not a class: run `python tools/gaps.py --member 'Core\Time::…'`
first, because this is the run that learned what a stale worklist costs.

- [ ] **`Core\Time::fromEpoch` round-trips every `Instant` member that answers an epoch count**
      (`crates/nvs-stdlib/src/time.rs:1116` `fromEpoch`, `time.rs:1683` its implementation, with
      `Instant::toEpochSeconds` `time.rs:704`, `toEpochMillis` `:711` and `toEpochMicros` `:718`,
      each carrying one case) — the *agreement* shape: one swept table of epoch
      values, asserting the three resolutions agree with each other about the same instant and that
      `fromEpoch` undoes each, counted rather than read off the rows.
- [ ] **`Core\Time::monotonic` is ordered where the wall clock is not** (`crates/nvs-stdlib/src/time.rs:1102`, implemented at
      `time.rs:1648`, one case) — the *invariant* shape: a sweep of reads is non-decreasing, and its difference is a
      `Duration` rather than a clock reading. Check first what `sleep` costs in a case's runtime;
      keep the sweep tight enough that the suite does not pay for it.
- [ ] **`Core\Test::assertCount` and `assertThrows` name what failed** (`crates/nvs-stdlib/src/test.rs:172`
      and `:183`, implemented at `test.rs:365` and `:430`, one case each) — the *edges* shape, alongside the ledger case that already
      exists: a wrong count and a body throwing the wrong class each produce a message, read back
      through `expectFailure`.

## Backlog

- `Core\Regex\Match::offset` and `Core\Regex::quote`/`replaceWith` carry one case each — `gaps.py`.
- `Core\Str::fold`/`graphemes`/`indexOf` and `Core\Arr::column`/`flattenDeep`/`overlayDeep` are the
  single-case members of the two largest classes — `gaps.py`.
- 68 unasserted error paths, 65 of them `Fault::fatal` and mostly internal — `gaps.py --errors`.
- The `every_refusal_is_a_diagnostic_or_decided` allowlist may never grow — loop-goal.md.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  docs/agent/guard-name-debt.md.
