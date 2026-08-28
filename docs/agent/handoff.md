# Handoff

## State

**M4's Stage 8 is where the work is, and `Core\ObjectMap` is no longer the thinnest
class**: its two absent-key/sweep cases landed, so the tree is at **748 conformance plus
189 differential** and `python tools/gaps.py` now ranks `Core\Validate` (0.83) thinnest,
then `Core\Uri` (0.84) and `Core\Random` (0.86); `Core\ObjectMap` reads 1.00 (9 cases
over 9 members). The frontier is still the conformance floor of 750 and, past coverage,
depth; `python tools/loop.py --list` reports no named `.nvst` case owed by any stage, and
`python tools/gaps.py --differential` still ranks 0.

- The two landed cases are two of the four shapes: **an edge**
  (`object-map-get-and-remove-on-an-absent-key.nvst` — what `get` answers for a key the
  map never held, for one removed, and for one whose stored value is `null`, plus that a
  `remove` matching nothing leaves `count` and both lists byte-identical) and
  **invariance over a sweep** (`object-map-count-is-empty-and-clear-agree-over-a-sweep.nvst`
  — `count`, `isEmpty` and the two lists' lengths asserted to agree at all 17 points of a
  write/re-write/remove/clear sweep, counted rather than read off a line).
- **The group's third slice was already pinned and was not written.** "`set` over a key
  already held is an overwrite, not a second pair" is
  `object-map-pairs-its-two-lists-through-every-write.nvst`'s second and fifth sections
  (the re-`set` in place, the re-added key landing at the end) plus the re-`set` rows of
  the new sweep; a third case over that rule would be another row of the same shape, which
  conventions.md § *A `.nvst` test case* rules out.

## Next group

**`Core\Validate`'s remaining edges**, the thinnest class `python tools/gaps.py` ranks.
The file set is `crates/nvs-stdlib/src/validate.rs` for the lines each predicate draws and
`tests/conformance/core/` for the cases; the standing five already own the four length
limits on both sides, the `{version?: 4|6}` literal union, the `isIp` families, the
`isAscii`/`isPrintable` bounds and the four structural predicates agreeing about a
degenerate sweep — so what is left is each predicate's own *character* boundary.

- [ ] **`isMac`'s accepted spellings and the first refused one** (`validate.rs:184`) — the
      *bound at both ends* shape: which separator forms and which hex case the member
      takes, named beside the nearest spelling it refuses.
- [ ] **`isEmail`'s local part** (`validate.rs:163`) — the *edge* shape: the leading,
      trailing and consecutive dot, plus-addressing, and the first character the member
      stops accepting, each cited to the RFC the member implements rather than to whatever
      it printed.
- [ ] **`isDomain`'s label characters** (`validate.rs:170`) — the *edge* shape again: a
      leading and a trailing hyphen, an underscore, a trailing dot, and the agreement
      between `isDomain($d)` and `isEmail("a@" . $d)` over the same sweep, counted.

## Backlog

- `Core\Uri` (0.84) and `Core\Random` (0.86) are the next two thinnest —
  `python tools/gaps.py`.
- A `Core` enum cannot be a closure or method parameter's declared type, so a sweep
  factors its comparison rather than its subject — playbook, *Writing a test case*.
- 68 unasserted error paths, 65 of them `Fault::fatal` — `python tools/gaps.py --errors`,
  judged before written (playbook has the four-`nvs run` recipe).
- `csv.rs:512`'s `thrown` is unreachable from source — playbook, and it stays owed nothing.
- `orient.py` printed everything this session needed; no `[context]` field was missing.
