- **A case asserting two float computations agree has to pick rows whose *intermediate* is exactly
  representable, or it pins one libm.** `Core\Math::hypot($x, $y)` against `Core\Math::sqrt($x * $x
  + $y * $y)` can agree on every row of a table only because MSVC happens to round them the same
  way, and nothing says glibc on the WSL leg does. Restrict the table to Pythagorean triples, zeros
  and dyadic fractions so the intermediate is exact and IEEE 754 requires the two to agree — a
  property of the table, not the host. [until: gone crates/nvs-stdlib/src/math.rs:"hypot"]
