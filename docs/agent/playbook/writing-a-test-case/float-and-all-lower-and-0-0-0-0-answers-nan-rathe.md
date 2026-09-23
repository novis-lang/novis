- **Float `<`, `>`, `&&` and `||` all lower, and `0.0 / 0.0` answers `NAN` rather than throwing, so
  a case can assert "near, not equal".** The `E0715` refusal is about strings and the other
  unordered domains; two `float`s compare for order fine. The tolerance spelling is `float $tol =
  0.000000000001 * ($mag + 1.0);` then `if (($d < $tol) && ($d > 0.0 - $tol))`, with the magnitude
  taken by hand because `Core\Math::abs` answers `int|float` and not a `float`; `-0.0` echoes as
  `-0` and is told from `0.0` through `1.0 / $x`. [until: reviewed 2026-09-06]
