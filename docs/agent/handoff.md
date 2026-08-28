# Handoff

## State

**M4's Stage 7 is closed; the frontier is Stage 8's corpus floor of 750** and the tree is
at **743**. `python tools/loop.py --list` reports no named `.nvst` case owed by any stage,
so depth is the whole of what is left, and `python tools/gaps.py` is what ranks it.

- **That ranking was wrong and is fixed.** A case belongs to a class when it names it *or*
  when it holds one of its values: `producers` reads each member's `CoreTy::Instance(...)`
  return type out of the registry, a written `Owner::member(` attributes the case to what
  it builds, and an instance call `->member(` on a class the case already holds attributes
  it to what that builds, to a fixed point. `tools/gaps.py`'s `coverage` docstring is that
  rule's home. No class now has a member no case calls; the thinnest are
  `Core\Time\DateTime` 0.71, `Core\ObjectMap` 0.78 and `Core\Validate` 0.83.
- **The eight `DateTime` builders share one rule and it is pinned by count.**
  `tests/conformance/core/time-datetime-builders-answer-a-new-value-in-the-same-zone.nvst`
  asks each of `plus`/`minus`/`next`/`previous`/`with`/`withTime`/`startOf`/`endOf` three
  questions over 24 receivers — the receiver still reads as it did, the zone travelled,
  and the answer is a civil time that zone actually has — and reads `576|576`.
- The two DateTime cases beside it own the *values*: `-is-a-civil-time-in-a-zone` the
  per-member answers, `-parts-agree-with-the-value-they-came-from` the component views
  against `format`. A fourth case reaching for another row of either is the thing to
  avoid; ADR 0079-style *agreement* over a shared rule is what is left there.

## Next group

**The three deterministic `Core\Time` members with a PHP twin and no oracle case** —
`python tools/gaps.py --differential` ranks them. The file set is
`crates/nvs-stdlib/src/time.rs` for the signatures and `tests/differential/core/` for the
cases; PHP computes every expectation, so nothing is frozen by hand.

- [ ] **`Core\Time::at` ← `mktime`/`gmmktime`** (`crates/nvs-stdlib/src/time.rs:1140`) —
      the options bag against PHP's positional argument order, a clamped month, and the
      date that does not exist, which Novis throws at and PHP rolls over.
- [ ] **`Core\Time::fromIso` ← `strtotime`** (`crates/nvs-stdlib/src/time.rs:1722`) — the
      texts both accept, written against the offsets PHP also reads; the relative
      expressions PHP accepts are refused here and belong in the standing conformance
      case rather than in the oracle half.
- [ ] **`Core\Time::parse` ← `strptime`** (`crates/nvs-stdlib/src/time.rs:2468`) — the
      CLDR pattern against PHP's own `date`-formatted output, so the twin renders what
      the pattern reads.

## Backlog

- `Core\Time::now`/`monotonic`/`sleep` have twins but no *stable* expectation — both
  sides must print a derived bound rather than a clock; decide that shape first
  (`crates/nvs-stdlib/src/time.rs:1634`).
- `Core\Encoding::encodeText`/`decodeText`/`isValidText` oracle cases are blocked on the
  Windows `php` having no `mbstring` (playbook, *Writing a test case*).
- `gaps.py`'s `registry` still reads `symbol:` in a fixed 600-character window, so a
  member with a long inline options bag anchors on its signature line rather than its
  implementation. `producers` bounds its own search at the next literal instead.
- Stage 8's floor is 750 conformance cases; the tree holds 743.
- `Core\ObjectMap` (0.78) and `Core\Validate` (0.83) are the next thinnest after the time
  family, and both already carry five or more cases — a new one there needs a rule no
  existing case asserts, not another member.
