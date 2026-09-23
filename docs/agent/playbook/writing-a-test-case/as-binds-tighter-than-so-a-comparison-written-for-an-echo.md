- **`as` binds tighter than `==`, so a comparison written for an `echo` needs its own parentheses.**
  `$a == $b as string` parses as `$a == ($b as string)`, and a `.nvst` case surfaces it as `E0466:
  disjoint` plus `E0710: cannot be converted` naming the *right* operand — neither diagnostic says
  "precedence", so the case looks like a type error in the values rather than in the punctuation. Write
  `($a == $b) as string`, or better render the member's own text (`$at->toIso()`) so the line asserts a
  value rather than a `bool`.
  [until: reviewed 2026-09-16]
