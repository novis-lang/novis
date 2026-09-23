- **A `Core` member's numeric parameter is often `uint`, and a helper factoring a sweep has to
  declare it that way.** A literal `64` places as `uint` at the call site, so
  `Core\Str::repeat("36", 64)` compiles and hides the rule, but a count arriving through a helper
  parameter typed `int` is `E0401: expected uint, found int` at every call. Type the parameter
  `uint` (arithmetic on it stays `uint`); and since `Core\Str::length` *returns* `uint`, a loop
  counter fed from it wants `as int` or a `uint` of its own. [until: reviewed 2026-09-06]
