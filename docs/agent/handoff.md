# Handoff

## State

**M4's Stage 8, depth.** The tree is at **864 conformance plus 189 differential**, all
green. Nothing is blocked.

Six cases landed over one file set — `crates/nvs-stdlib/src/uri.rs` read, and
`tests/conformance/core/` written — which takes `Core\Uri` off the top of `gaps.py`'s
table entirely (depth 6.0 now, where `Core\Bytes` is 4.0). The four percent-coders were
each already pinned *against each other*; what was left in every case was the **quantifier**:

- **`decodeFormValue`, three cases.** The refusal was shown on five hand-picked families
  and is now swept: all 256 single-byte escapes (128 accepted, 128 refused, and
  `decodeComponent` drawing the identical line on all 256), then all 256 second octets
  after the lead `%C3` (64 accepted — the continuation range, so a lone octet the first
  sweep refused is accepted *in company*, which is what makes the rule about the stream
  rather than the byte). Second: the refusal is the *only* one there is — total over all
  128 raw bytes and over all 256 arrangements of a `%` with a non-hex neighbour — and it
  takes the whole subject, never the prefix already decoded. Third: the two rewrites are
  **one pass over bytes**, which three rows separate from the two alternative orders —
  `%2+0` is `%2 0` (a `+` inside a malformed escape is still a space, swept over all 22
  hex characters in both positions), `a+%2B+b` is `a + b` (the `+` an escape produced is
  output, not input), and `%C3+%A9` is refused (a `+` is an octet, so it splits a sequence
  that was well formed without it). PHP 8.5's `urldecode` agrees on every textual row.
- **`encodeFormValue`, two cases.** Which bytes survive, counted: exactly 65 of 128 — RFC
  3986 § 2.3's set less the `~` the form encoding predates — with the other 63 being one
  `+` and 62 escapes compared against a hand-built `%` plus two upper-case hex digits, so
  the digits' case, number and value are one equality. And that the member is
  **deliberately not idempotent**: encoding twice is a fixed point for exactly the 65 and
  changes the other 63, one decode peels exactly one layer for all 128, and a space that
  became `+` re-encodes to `%2B` rather than being spared.
- **`encodeComponent`, one case.** The claim the module doc makes is about the *parser*,
  so it is asserted against the parser: every one of the 128 bytes, escaped and
  interpolated into a path segment, parses and reads back out of `$uri->path()` as itself
  (128/128), while the same bytes raw partition 80 stayed / 2 moved / 46 refused.

The move that found all six: where a corpus already pins a rule with rows, the unasked
question is the rule's **quantifier** — swept exhaustively over the domain the rule ranges
over, with the *other* member asked the same question in the same loop so the two agree by
count rather than by inspection.

`orient.py`'s pack was complete for the goal. The `[context] modules` gap two handoffs have
now reported — no `nvs-stdlib/src/*.rs` pattern, so no `Core` class's module line is ever
printed — cost one `peek.py` here as well.

## Next group

**`Core\Bytes`'s three thinnest members** — now the top of `gaps.py` at depth 4.0, floor 3.
One shared file set: `crates/nvs-stdlib/src/bytes.rs` read, `tests/conformance/core/`
written. The three are one another's bounds (a needle at each end of the haystack is the
same scan asked three ways), so a session holding one holds all three.

- [ ] **`Core\Bytes::startsWith` / `endsWith`** (3 cases each today) — rows
      `bytes.rs:211` and `bytes.rs:218`, helpers `bytes.rs:619` and `bytes.rs:629`.
      Spec § 13. The unasked half is the **empty and the over-long needle**, and the
      degenerate agreement: a needle equal to the subject satisfies both, an empty one
      satisfies both at every subject including the empty one, and one byte longer than
      the subject satisfies neither — counted over a sweep rather than shown on a row.
- [ ] **`Core\Bytes::contains`** (3 cases) — row `bytes.rs:204`, helper `bytes.rs:608`.
      Spec § 13. `contains` is implied by both of the above, so the case worth writing is
      the **agreement**: over a sweep of subject/needle pairs, `contains` is true wherever
      `startsWith` or `endsWith` is, and the pairs where it is true and neither is are
      exactly the interior occurrences.
- [ ] **A byte-boundary case for the three together** — `Core\Bytes` is not `Core\Str`,
      so a needle that is a *suffix of a multi-byte character's octets* matches here where
      the text member would not. That is the one property that says which of the two
      classes a program is calling, and no case asks it.

## Backlog

- `crates/nvs-stdlib/src/csv.rs:512`'s `Fault::thrown` is unreachable from source and owed
  no case — playbook, *Divergences and refusals already pinned*.
- `[context] modules` in `docs/agent/loop-goal.toml` has no `nvs-stdlib/src/*.rs` pattern.
- `docs/agent/guard-name-debt.md`: 54 of 156 guard names match nothing `cargo test` runs.
- `Core\Test`, `Core\Random` and `Core\Time` are the next three thinnest after `Core\Bytes`
  — `python tools/gaps.py`.
