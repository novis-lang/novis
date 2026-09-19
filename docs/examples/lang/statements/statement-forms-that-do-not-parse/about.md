Novis reads PHP syntax, but not all of it. A few PHP statement forms do not parse, and the compiler
names the replacement when it stops.

`goto` and its labels are gone: use a loop, an early `return`, or a flag. `declare(strict_types=1)`
is not needed, because every Novis file is strict already. The alternative syntax `if (…): … endif;`
is not accepted: write braces. Instead of `global $x;`, pass the value as a parameter, or keep it in
a `private static` property. That property is also what replaces a `static` variable inside a
function. Write `[$a, $b] = …` instead of `list($a, $b) = …`, and `require` instead of `include` or
`require_once`. A `class`, `interface`, `enum`, `type`, `namespace`, `use` or `autoload` statement
goes at the top of a file, never inside a body.

**The examples below** show what to write instead of `goto`, instead of `global` and a `static`
variable, and instead of `list()` and the alternative syntax.
