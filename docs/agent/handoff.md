# Handoff

## State

**M4's Stage 8 is where the work is, and `Core\Time\DateTime` is no longer the thinnest
class**: its three calendar-arithmetic members gained the cases § 4's opening rule asks
for, so the tree is at **746 conformance plus 189 differential** and `python
tools/gaps.py` now ranks `Core\ObjectMap` (0.78) thinnest, then `Core\Validate` (0.83)
and `Core\Uri` (0.84). The frontier is still the conformance floor of 750 and, past
coverage, depth; `python tools/loop.py --list` reports no named `.nvst` case owed by any
stage, and `python tools/gaps.py --differential` still ranks 0.

- The three landed cases are the three shapes: **agreement** (one step asked of a
  `DateTime` and of its `toInstant()`, counted over nine exact units and then parted by a
  short month, a DST night and a leap year), **a bound at both ends** (`startOf`/`endOf`
  over all eleven `Core\Unit` cases, each `endOf` asserted to be exactly one nanosecond
  short of the next repetition's start), and **an edge** (`next`/`previous` asked for the
  weekday the receiver already is, which is a whole week away and never the receiver).
- **A `Core` enum cannot be a closure or method parameter's declared type** — the
  playbook bullet under *Writing a test case* has the spelling and the two codes. It is
  why none of the three sweeps is factored over `Core\Unit`; each factors the comparison
  instead. It is a checker hole rather than a case-writing rule, and it is in `## Backlog`.

## Next group

**`Core\ObjectMap`'s edges**, the thinnest class `python tools/gaps.py` ranks. The file set
is `crates/nvs-stdlib/src/objmap.rs` for the signatures and `tests/conformance/core/` for
the cases; the three standing cases are `object-map-keys-on-identity.nvst`,
`object-map-iterates-its-keys.nvst` and
`object-map-pairs-its-two-lists-through-every-write.nvst`, so what is left is what each
member answers where it stops accepting.

- [ ] **`get`/`remove` on a key the map does not hold** (`objmap.rs:84`, `:98`) — the
      *edge* shape: what an absent key answers, what `remove` answers for one, and that a
      failed `remove` leaves `count` and the two lists exactly as they were.
- [ ] **`clear`/`isEmpty`/`count` agree over a sweep** (`objmap.rs:105`, `:112`, `:133`) —
      the *invariance* shape: `count` and `isEmpty` say the same thing after every write,
      overwrite, removal and clear, counted rather than read off a line, and a cleared map
      is reusable rather than merely empty.
- [ ] **`set` over a key already held is an overwrite, not a second pair**
      (`objmap.rs:77`) — the *agreement* shape: `keys()` and `values()` stay index-paired
      and the count does not grow, identity being the key (ADR 0090 § 3).

## Backlog

- A written `Core\Unit`/`Core\Weekday` annotation is not the registry's enum type
  (`E0401` at the call, `E0708` at an `as int` inside) — no owning doc yet; it is a
  `nvs_types` type-interning hole, not a `Core\Time` one.
- `Core\Time::sleep` returns at once for a negative `Duration` where all three of PHP's
  functions raise a `ValueError` — named in the sleep case's own prose, undecided.
- `Core\Validate` (0.83) and `Core\Uri` (0.84) are the next two thinnest classes after
  `Core\ObjectMap` — `python tools/gaps.py`.
- A `require` whose path is not a string literal runs nothing, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- ADR 0024 § 4's sink list and ADR 0033's `Core\Log` inspection wait on M7/M8 `Core`
  classes — `docs/agent/loop-goal.md` § *Standing decisions*.
