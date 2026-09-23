- **`as` refuses a `Core` class as its target, so a capture union's `Core\Uuid` arm cannot be
  downcast the way a user class's can.** `E0711` says the target "names no class to test the value
  against", because `rule:types/conversion` tabulates conversions into a *declared* class and
  `Core\Uuid` is not one — while `$capture as Slug` compiles right beside it and reads as if the two
  were symmetric. Read the engine's arm by rendering it instead: `echo $match->param("id")` prints the
  canonical text, since every arm of that union renders.
  [until: reviewed 2026-09-08]
