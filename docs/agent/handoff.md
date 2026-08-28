# Handoff

## State

**M4's Stage 7 is closed; the frontier is Stage 8's corpus floor of 750** and the tree is
at **743 conformance plus 186 differential**. `python tools/loop.py --list` reports no
named `.nvst` case owed by any stage, so depth is the whole of what is left, and
`python tools/gaps.py` is what ranks it.

- **`Core\Encoding`'s three text members now have five oracle cases**, and
  `python tools/gaps.py --differential` is down from six members to **three**, all of
  them clocks. The twin throughout is `iconv` and not `mb_convert_encoding`: this box's
  `php` carries no `mbstring` at all (`extension_loaded("mbstring")` is `false`), so
  `isValidText`'s own case is written against the same-charset `iconv` probe a program
  without mbstring already reaches for, and says so in its own prose.
- The three divergences are one shape: **PHP picks between three lossy answers by
  suffix** where these members throw. Plain `iconv` answers `false` and raises a
  `Notice`, `//IGNORE` drops the character or the sequence, `//TRANSLIT` approximates it
  where the C library has an approximation and answers `false` where it does not — and
  `mb_convert_encoding`'s U+FFFD, the fourth, is not an error a caller can notice later
  at all. Each case writes the row twice: the refusal, and the lossy spelling said out
  loud in the caller's own text, which is what makes it a demand for the spelling rather
  than a capability Novis lacks.
- **One checker hole was found and is not fixed**: a `Core` enum case is not assignable
  to a binding of its own enum type (`E0401`, "expected `Core\Charset`, found
  `Core\Charset`"), for `Core\Charset` and `Core\Unit` alike, where the identical shape
  over a user-declared `enum` passes. It is in `## Backlog` with the reproduction, and
  the playbook bullet under *Writing a test case* is why the new cases' sweeps are
  written out a row at a time.

## Next group

**The three `Core\Time` clock members with a PHP twin and no oracle case** — everything
`python tools/gaps.py --differential` still ranks, and the last of that gap. The file set
is `crates/nvs-stdlib/src/time.rs` for the signatures and `tests/differential/core/` for
the cases. **None of the three has a deterministic value**, so each case asserts a
*property* both implementations must hold — a bound, an ordering, an agreement between
two spellings — rather than an output PHP can print; the `--ORACLE--` half computes the
same property from PHP's own clock and prints the same verdict.

- [ ] **`Core\Time::now` ← `time`/`microtime`/`date_create`**
      (`crates/nvs-stdlib/src/time.rs:1634`) — the epoch seconds each side reads sit
      within a few seconds of the other's, and `now` is at or after a fixed past instant
      and before a fixed future one, so the case is a bracket rather than a value.
- [ ] **`Core\Time::monotonic` ← `hrtime`** (`crates/nvs-stdlib/src/time.rs:1645`) —
      two reads never go backwards and the difference over a bounded loop is
      non-negative and under a generous ceiling; the divergence half is that `hrtime`
      answers an `int` of nanoseconds or a two-element array while this member answers
      a value that cannot be confused with a wall clock.
- [ ] **`Core\Time::sleep` ← `sleep`/`usleep`/`time_nanosleep`** (`time.rs:1664`) — the
      monotonic clock advanced by at least the requested duration, measured on both
      sides, with the duration small enough that the case costs no wall clock worth
      naming.

## Backlog

- A `Core` enum case is not assignable to a binding of its own type — `Core\Charset $c =
  Core\Charset::Ascii;` is `E0401`, and so is `array<Core\Charset> $s = [...]` and
  `Core\Unit $u = Core\Unit::Month;`, while the same shape over a user `enum` compiles.
  Every `Core` enum argument must therefore be written inline at its call site. Owner:
  `nvs_types::expr::is_assignable` against ADR 0047 § 5's literal/case types.
- A `require` whose path is not a string literal runs nothing at all, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: a `secret` value behind an `array<T>` element or an ADR 0036
  shape field carries no bit — `nvs_stdlib::debug`'s known gap 1.
- `nvs_stdlib::test`'s known gap 1: a non-`Comparable` object under `assertEquals` throws
  where ADR 0079 § 4 writes a compile error.
- `signatures.rs`' known gap: a class constant's declared type is unmodelled, so
  `Class::TOKEN` infers `mixed` at every expression site.
- ADR 0024 § 5's `string as Core\Html\Markup` is `nvs-ir`'s one remaining `as` catch-all
  target and waits on `Core\Html` existing at all (M7).
