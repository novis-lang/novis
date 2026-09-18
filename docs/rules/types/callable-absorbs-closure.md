`callable` is the only type name for a closure value. `Closure` is not a type in Novis, and naming it
is a diagnostic. This is a rename rather than a behaviour change: after
`rule:types/callable-is-a-closure` narrowed which values satisfy `callable`, the two names had
identical membership, and `callable` reads more accurately for a value that captures nothing at all,
such as a reference to a static method.

`bind`, `bindTo` and `call` still exist, called with the same method-call syntax, now as builtin
operations on an opaque type rather than inherited methods of a base class a program could name in an
`is` test or extend. `Closure::fromCallable` is dropped, because after that narrowing there is
nothing left for it to normalise away from.

`call_user_func` and `call_user_func_array` are dropped with it. Every `callable` value supports
direct invocation, which is what they existed to route around:

```php
$result = $fn($arg);       // replaces call_user_func($fn, $arg)
$result = $fn(...$args);   // replaces call_user_func_array($fn, $args)
```
