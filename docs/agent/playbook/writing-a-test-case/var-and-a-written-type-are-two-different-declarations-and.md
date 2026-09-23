- **`var` and a written type are two different declarations, and `var T $x = …` is neither.** `var
  $x = …` infers; a declared type is `T $x = …;` with no `var` at all, so `var Core\Regex\Pattern $p
  = …` is four diagnostics (`E0101` three times, then `E0301` for a name never declared), none of
  which says "drop the `var`". The inferring spelling is the one every case reaches for, so the
  typed one looks like it should take a keyword too. [until: reviewed 2026-09-06]
