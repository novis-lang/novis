# Handoff

## State

**M4's Stage 7 is closed; the frontier is Stage 8's corpus floor of 750** and the tree is
at **743 conformance plus 181 differential**. `python tools/loop.py --list` reports no
named `.nvst` case owed by any stage, so depth is the whole of what is left, and
`python tools/gaps.py` is what ranks it.

- **`Core\Time`'s three PHP twins with no oracle case now have six**, three agreeing
  halves and three divergences, and `python tools/gaps.py --differential` is down to
  six members from nine. `at` against `mktime`/`gmmktime` (the epoch rows, and both
  civil times New York's DST transitions make ambiguous, which the two implementations
  resolve identically), `fromIso` against `strtotime`, and `parse` against
  `DateTime::createFromFormat` — `strptime` is removed from PHP outright, so it is the
  twin every `parse` oracle is written against.
- The three divergences are one shape: PHP **guesses** where these members refuse. A
  calendar field out of range rolls over (`mktime(0,0,0,13,…)`), a text with no offset
  is placed in ambient zone state, a text that spells nothing is `false`, and a field
  the pattern did not name is filled from the clock. Each case writes the row twice —
  the refusal, and the member that says out loud what the PHP call meant.
- **One classification was wrong and is fixed.** A **zonal** field in a `Core\Time::parse`
  pattern is a mistake in the *pattern*, so it is a `LogicError` raised against the
  compiled pattern before a byte of text is read (`crate::cldr::civil_fields_only`, the
  third sibling of `date_fields_only`/`time_fields_only`), where it used to arrive from
  inside `read` as the `ParseError` that names the input's failure — which contradicted
  `nvs_core_time_parse`'s own doc comment. `read`'s branch survives as an
  internal-consistency check, and
  `tests/conformance/core/time-parse-refuses-a-pattern-and-a-text-in-different-classes.nvst`
  — which pinned the old class, with a reason two sibling guards over the same
  compiler already refute — moves that row into its pattern half and counts it.

## Next group

**The three `Core\Encoding` text members with a PHP twin and no oracle case** — the last
three `python tools/gaps.py --differential` ranks that are not a clock. The file set is
`crates/nvs-stdlib/src/encoding.rs` for the signatures and `tests/differential/core/`
for the cases. **The twin is `iconv` and not `mb_convert_encoding`**: this box's `php`
has no `mbstring` (playbook, *Writing a test case*), and `utf8_encode`/`utf8_decode` are
deprecated in 8.5, so a case naming either writes a deprecation notice into the very
output it is compared on.

- [ ] **`Core\Encoding::encodeText` ← `iconv`** (`crates/nvs-stdlib/src/encoding.rs:751`)
      — the agreeing half over the encodings both carry, and the divergence at the
      byte `iconv` substitutes or drops where this member refuses.
- [ ] **`Core\Encoding::decodeText` ← `iconv`** (`crates/nvs-stdlib/src/encoding.rs:781`)
      — the round trip against the encode half, and the invalid sequence.
- [ ] **`Core\Encoding::isValidText` ← `mb_check_encoding`** (`crates/nvs-stdlib/src/encoding.rs:809`)
      — with no `mbstring` the twin is `iconv` answering `false`, so this one is likely
      a divergence case rather than an oracle: say so in its `--ORACLE-DIVERGES--`.

## Backlog

- `Core\Time::now`/`monotonic`/`sleep` are the differential gap's other three, and none
  of them has a constant answer — a case there is a *bound* (elapsed ≥ the sleep, the
  monotonic clock never going backwards), not an oracle. `crates/nvs-stdlib/src/time.rs:1634`.
- `Core\Time\DateTime` is still the thinnest class `python tools/gaps.py` ranks (0.71),
  with `Core\ObjectMap` 0.78 and `Core\Validate` 0.83 behind it.
- ADR 0079-style *agreement* over a shared rule is what is left in the three standing
  `DateTime` conformance cases; a fourth reaching for another row of either is the thing
  to avoid.
