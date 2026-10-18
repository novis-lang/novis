One operation is reachable exactly one way. A stateless operation is a static method on a domain class;
anything with identity or a lifetime is an object (`rule:core-api/a-lifetime-is-an-object`). Nothing is
reachable both ways — there is no procedural twin of a class API and no class wrapper around a static one —
and a domain class's static members never mirror an object's own methods: `Time::format($instant, $fmt)`
may not exist beside `$instant->format($fmt)`.

This closes the second and less visible kind of duplication, the one a function list cannot show: a
procedural function beside every method of a class, so that a whole API exists twice. A tree API and a streaming reader over the same data are *different jobs* rather
than twins, and each such class's introduction page says so where that could be misread. So are two parsers onto one
tree: `Core\Html::parse` and `Core\Xml::parse` take different languages under opposite failure
contracts and produce the same node family, so each tree operation is written once and reached through
neither door's own surface (`rule:core-classes/html-parsing`).

**An operator is syntax, not a second API, and is never counted here** — `is`, `as ?T` and the
pipeline (`rule:expressions/pipeline-substitution`) reach the same member through the same call and add
nothing to reach. The one genuinely reachable two-spellings case is a compile error: a `Core` *instance*
member written as a static call is refused, because such a member's receiver travels in argument slot 0 and
would otherwise pass the same arity check the instance call passes.

**Where the language has an operator, `Core` has no member for it.** `Core\Arr` covers whole-array
operations only, and element access stays in the language:

| Operation | Written | Note |
|---|---|---|
| read an element | `$a[$k]` | throws on a missing key (`rule:core-api/shape-rules` R4) |
| read a possibly-absent element | `$a[$k] ?? $default` | there is no `Arr::get` |
| write an element | `$a[$k] = $v` | |
| remove an element | `unset($a[$k])` | an array element of a named holder only (`rule:classes/unset-is-refused-on-a-property`) |
| test whether a key exists | `Arr::hasKey($a, $k)` | the one way to test for a key |
| combine two arrays | `Arr::overlay` / `underlay` / `appendAll` | `$a + $b` does not compile (`rule:types/array-combination`) |

The same division holds elsewhere: `**` is exponentiation, so there is no `Math::pow`; `%` is integer
modulo, so `Math::mod` exists only for `float`; `is` is an operator, so `Core\Reflect` has no `isA`.
