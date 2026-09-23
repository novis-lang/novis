- **A `.nvst` case cannot hold `Core` instances in an `array<mixed>`.** `$one as Core\Uri` on an
  element is `E0711` ("`rule:types/conversion` tabulates no conversion into an object"), so a sweep
  that builds many objects and then asks one question of each has no way back to the object. Collect
  the *answers* instead — `$built[] = $u->toString();` — and assert over the text.
  [until: reviewed 2026-09-06]
