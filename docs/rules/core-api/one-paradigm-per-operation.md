One operation is reachable exactly one way. A stateless operation is a static method on a domain class;
anything with identity or a lifetime is an object (`rule:core-api/a-lifetime-is-an-object`). Nothing is
reachable both ways — there is no procedural twin of a class API and no class wrapper around a static one —
and a domain class's static members never mirror an object's own methods: `Time::format($instant, $fmt)`
may not exist beside `$instant->format($fmt)`.

This closes the second and less visible kind of duplication, the one a function list cannot show: a
procedural function beside every method of a class, so that a whole API exists twice. A tree API and a streaming reader over the same data are *different jobs* rather
than twins, and the spec says so explicitly where that could be misread.

**An operator is syntax, not a second API, and is never counted here** — `is`, `as ?T` and the
pipeline (`rule:expressions/pipeline-substitution`) reach the same member through the same call and add
nothing to reach. The one genuinely reachable two-spellings case is a compile error: a `Core` *instance*
member written as a static call is refused, because such a member's receiver travels in argument slot 0 and
would otherwise pass the same arity check the instance call passes.
