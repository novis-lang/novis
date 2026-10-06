Two spellings are refused, and each diagnostic names its replacement:

- `static int $calls = 0;` in a function — *function-scope `static` is not supported; declare a
  `private static` property on a class, or pass the value as a parameter*.
- `global $x;` — *`global` is not supported; pass `$x` as a parameter, or make it a `static` property or a
  `const`*.

A diagnostic that says only "not supported" is a bug against this rule, not a faithful implementation of
it.

A function static is the one binding whose definite assignment cannot be checked: its initialiser runs on
the first call and on no later one, so on every call after the first the binding is live while its
initialiser is not on the executed path. It is also a third storage class for one keyword — a per-function
slot, per isolate, with a run-once flag the JIT cannot fold away — and it hides from the signature that a
function's result depends on how often it has been called. A function static could never outlive its
request in any case, so nothing a program could rely on across requests is lost.
