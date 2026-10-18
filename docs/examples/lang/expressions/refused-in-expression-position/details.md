Each form below is a compile error with its own code. The help line of the error names what to
write instead.

- `eval(…)` is `E0201`. Put the code in a file and `require` it, or run it with `spawn script`.
- `extract($a)` is `E0205`. Use destructuring, or read each key.
- `settype($x, "int")` is `E0208`. Convert into a new variable with `as`.
- `compact(…)`, `strlen(…)` and every other free function is `E0320`. Use a `Core` method such as
  `Core\Str::length($s)`.
- `$$name` and `${$expr}` are `E0202`. Use an `array<T>` with a key for each name.
- `list($a, $b) = $p;` is `E0230`. Write `[int $a, int $b] = $p;`.
- `(int)$x` and the other casts are `E0225`. Write `$x as int`.
- `@$a["k"]` is `E0236`. Catch the error with `try` and `catch`.
- `$b =& $a;` is `E0701`. Write `$b = $a;`, which copies the value.
- `die` is `E0228`. Write `exit`.
- `include` and `require_once` are `E0221`. Write `require`.
- In a generator, a `yield` whose value is used, such as `mixed $x = yield 1;`, is `E0448`. Write `yield 1;` as a
  statement of its own.
- `self`, `static` and `parent` outside a class are `E0303`. This is the same for `self::X`,
  `static::$x`, `parent::f()` and `new self()`. Write the class name.

A backtick is the one form with no replacement. `` echo `ls`; `` is the error `E0001`, an unexpected
character, and its message names nothing else. To run another program, use `Core\Process::run` with
the path and an array of arguments.
