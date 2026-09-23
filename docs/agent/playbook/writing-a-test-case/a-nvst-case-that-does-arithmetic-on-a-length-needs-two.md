- **A `.nvst` case that does arithmetic on a length needs two conversions nothing in the corpus
  makes obvious.** `Core\Bytes::length` answers `uint` while `Core\Bytes::slice`'s length parameter
  is `?int`, so `Core\Bytes::length($b) - 5` is `E0401`; and `$a / $b` is `int|float`, whose
  `as int` **throws** on a non-integral value rather than truncating, so a loop that splits a buffer
  into equal pieces needs `Core\Math::intDiv`. Write `(Core\Bytes::length($b) as int)` for the
  subtraction and `Core\Math::intDiv($a, $b)` for the division, and run
  `target/debug/nvs.exe test <one case>` before believing either shape.
  [until: reviewed 2026-09-16]
