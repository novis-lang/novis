An asymmetric property writes both halves: `public private(set) string $name;`. A bare `private(set)` with
its read side left to inference does not compile, and reports the same missing-visibility error, whose
message names the pair rather than a single keyword.

Inferring a `public` read side from the bare form would be the same implicit `public` this language
removes, wearing a different spelling. Accepting it would preserve exactly one slot where a member's read
visibility is a rule you have to know instead of a word you can see.

The plain keyword written alongside a `(set)` half is always the read half, so a reader and the checker
take it from the same place.
