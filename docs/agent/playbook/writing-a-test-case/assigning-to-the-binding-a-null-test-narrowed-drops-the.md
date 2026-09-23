- **Assigning to the binding a `!= null` test narrowed drops the narrowing on that statement's own
  right-hand side**, so the walk `while ($cursor != null) { … $cursor = $cursor->previous; }` is
  `E0459` on the assignment while every other read in the body is fine. The narrowing holds for the
  body but not across the write that re-widens the binding. Spell the step `$cursor =
  $cursor?->previous;` — `?->` answers `null` and the loop's own condition is what ends it.
  [until: reviewed 2026-09-19]
