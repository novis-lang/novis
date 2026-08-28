# Handoff

## State

**M4's Stage 8, depth.** The tree is at **858 conformance plus 189 differential**, all
green. Nothing is blocked.

Five cases landed over one file set — `crates/nvs-stdlib/src/uri.rs` read, and
`tests/conformance/core/` written — taking `Core\Uri` off the top of `gaps.py`'s table.
Two members, and in both the existing corpus already answered *whether* the member does
its job, so what was left was **scope** and **composition**:

- **`compareTo`, three cases.** RFC 3986 § 6.2.2's rewrites were each pinned by one row;
  none of them said how far it reaches. Case folding is § 6.2.2.1 and reaches 2 of the 6
  components that can hold a letter; escape normalization is § 6.2.2.2 and reaches all 5
  that can hold an escape, in both halves — an unreserved escape decodes, a reserved
  one's hex digits fold to upper case, and neither is the decoded byte (`%2f` ≡ `%2F` ≢
  `/`). § 6.2.2.3 is the rewrite with a *precondition*, so the same six dot-segment
  rewrites are counted twice, 6 of 6 under a leading `/` and 0 of 6 without one, with the
  rootless/path-absolute pair differing by that one byte. The third is the compositional
  one: `compareTo` is *predicted* from the seven readers for all 81 ordered pairs of a
  corpus and agrees on every one, and each of the seven decides the base against some
  member of it, so a member that grew a comparison the readers cannot see fails here.
- **`path`, two cases.** Where it *ends* — at the first `?` or `#` and no other byte,
  asserted by rebuilding all 14 relative references of a corpus out of the three readers
  that can see them, with a `?` after a `#` being a fragment byte and `%3F` never a
  delimiter. And which of § 3.3's five forms it may take, decided by what precedes it:
  9 of 9 authority-bearing rows are empty-or-rooted, 0 of 6 others begin `//`, and the
  `:` bound is named on both sides (`a:b` is a scheme, `./a:b` and `a%3Ab` are paths).

The move that found all five: where a class's corpus already pins each rule with one row,
the unasked question is that rule's **scope** — counted over the components or over the
positions the rule does *not* reach, with the neighbouring refusal beside it.

`orient.py`'s pack was complete for this goal; the `[context] modules` gap the previous
handoff reported (no `nvs-stdlib/src/*.rs` pattern) is still open and cost one `peek.py`
here too.

## Next group

**`Core\Uri`'s four percent-coders** — one shared file set, `crates/nvs-stdlib/src/uri.rs`
plus `tests/conformance/core/`, and the four members are one another's bounds, so a
session holding one holds all four. Read the three existing cases' `--TEST--` lines first
(`uri-percent-coders-are-two-inverse-pairs-over-a-byte-sweep`,
`uri-decode-form-value-is-decode-component-plus-the-plus`,
`uri-encodes-a-component-and-a-form-value-differently`) — the questions they already ask
are what makes the remaining one findable.

- [ ] **`Core\Uri::decodeFormValue`** (3 cases) — row `uri.rs:382`, helper `uri.rs:1772`.
      Spec § 12. The unasked half is the *refusal*: which byte sequences it declines
      (non-UTF-8, per gap 2) and whether it declines exactly the ones `decodeComponent`
      does, counted over a sweep rather than shown on a row.
- [ ] **`Core\Uri::encodeFormValue`** (2–3 cases) — helper `uri.rs:1753`. The scope
      question that worked above transfers: the encoder's set differs from
      `encodeComponent`'s on exactly two bytes (space and `~`), which is a *count* over
      all 256 and not two rows.
- [ ] **`Core\Uri::encodeComponent` / `decodeComponent`** — helpers `uri.rs:1714` and
      `uri.rs:1733`. § 2.3's unreserved set is 66 bytes; the round trip is already swept,
      the *set boundary* is not.

## Backlog

- `[context] modules` in `docs/agent/loop-goal.toml` names no `nvs-stdlib/src/*.rs`
  pattern, so a `Core` class's module doc is never in the pack — two sessions running.
- `Core\Uri::resolve` and `toString` are the two members of the class this group does not
  reach; `gaps.py` ranks them.
- ADR 0007 § 2's `array<T> as array<U>` still panics `nvs-ir`, which is what keeps several
  `gaps.py --errors` sites unreachable from source (playbook, `csv.rs:512`).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
