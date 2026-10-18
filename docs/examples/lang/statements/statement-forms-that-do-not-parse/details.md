Each form has its own error code:

- `goto` is the error `E0203`.
- `global $x;` is the error `E0204`.
- `static $n = 0;` inside a method is the error `E0209`.
- `list($a, $b) = …` is the error `E0230`.
- `include` and `require_once` are the error `E0221`.
- A `class`, `interface`, `enum`, `type`, `namespace` or `use` statement inside a body is the error
  `E0233`.

A label, `declare(…)` and the alternative syntax give a general syntax error. That error does not
name what to write instead.

In `[int $a, int $b] = …`, each variable needs its type. `[$a, $b] = …` does not compile, even when
`$a` and `$b` are already declared.
