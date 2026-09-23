- **A `!= null` test joined by `&&` narrows neither receiver, so `->` on either is still `E0459`.**
  `if ($a != null && $b != null) { echo $a->city; }` reports "this receiver is nullable" for both,
  while the same two tests written as two separate `if` blocks compile —
  `rule:expressions/nullable-conversion`'s narrowing reads one test per guarded branch and not a
  conjunction of them. Write one `if` per nullable receiver, or `?->` with `??` where the value is
  only echoed. [until: reviewed 2026-09-19]
