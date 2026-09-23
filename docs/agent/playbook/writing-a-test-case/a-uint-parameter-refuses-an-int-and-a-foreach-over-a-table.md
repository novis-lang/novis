- **A `uint` parameter refuses an `int`, and a `foreach` over a table of counts is where a case meets
  that.** `Core\Crypto::deriveKey`'s `$iterations` and every other counted argument are `uint`, and an
  `int` reaching one is `E0401 expected uint, found int` at the call rather than a widening, so a
  bounds sweep declared `array<int>` compiles everywhere except the line that matters. Declare the
  table `array<uint>` and the loop variable `uint`. [until: reviewed 2026-09-12]
