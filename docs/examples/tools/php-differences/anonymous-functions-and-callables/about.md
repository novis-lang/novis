An anonymous function in Novis is always written with `fn`. PHP calls this a `Closure`. The function
captures the outer variables that it reads, which means it copies them.

PHP's `function (...) use (...) { }` does not compile. Write `fn(int $x): int => $x + $k` for one
expression. Write `fn(int $x): int => { ...; return ...; }` for a body with several statements. No
`use` clause exists, because the function captures each outer variable by itself.

Each parameter declares its type. You may leave the type out when you pass the function directly to
a parameter that gives the type, as in `Core\Arr::map`. A method reference such as `A::f(...)` or
`$o->m(...)` works as in PHP. `strlen(...)` does not work, because Novis has no functions outside a
class. `call_user_func($f, 1)` is written `$f(1)`.

**Good to know:** a call through a `callable` returns `mixed`. Convert the result, as in
`$f(1) as int`.
