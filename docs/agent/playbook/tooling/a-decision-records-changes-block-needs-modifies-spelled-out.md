- **A decision record's `changes:` block needs `modifies:` spelled out even when the record modifies
  nothing.** `conventions.md` § *A decision record* shows both keys under one example that uses both, so
  a record creating two rules and editing none reads complete without the second — and then
  `tools/records.py --check` answers `changes: has no modifies: list`. Write `modifies: []` in the same
  keystroke as `creates:`, and run `records.py --check` beside `rules.py --render` rather than after it.
  [until: gone tools/nv/cmd/records.ts:modifies:]
