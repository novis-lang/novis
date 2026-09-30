A closure in Novis is always written with `fn`, and it copies the outer variables that it reads.

PHP's `function (...) use (...) { }` does not compile. Write `fn(int $x): int => $x + $k` for one
expression. Write `fn(int $x): int => { ...; return ...; }` for a body with several statements. No
`use` clause exists, because the closure copies each outer variable by itself.

Each parameter declares its type. You may leave the type out when you pass the closure directly to
a parameter that gives the type, as in `Core\Arr::map`. `A::f(...)` and `$o->m(...)` work as in
PHP. `strlen(...)` does not work, because Novis has no functions outside a class.
`call_user_func($f, 1)` is written `$f(1)`.

**Good to know:** a call through a `callable` returns `mixed`. Convert the result, as in
`$f(1) as int`.
