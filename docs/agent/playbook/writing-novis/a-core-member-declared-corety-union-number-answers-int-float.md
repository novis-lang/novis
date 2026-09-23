- **A `Core` member declared `CoreTy::Union(NUMBER)` answers `int|float|decimal`, which no binary
  operator meets a plain `int` or `float` across.** `Core\Math::abs($n) == $m` type-checks and dies
  at run time with *"nvs-codegen does not lower a binary operator over mismatched representations"*.
  Write `Core\Math::abs($x) as int` / `as float`; `grep -n 'CoreTy::Union' <the module>` lists the
  members that owe it. [until: gone crates/nvs-stdlib/src/math.rs:const NUMBER]
