`callable` is the only type name for a value made by an anonymous function or a method reference.
PHP's `Closure` is not a type in Novis, and naming it is a diagnostic that says to write `callable`.
This is a rename rather than a behaviour change: after `rule:types/callable-values` narrowed which
values satisfy `callable`, the two names had identical membership, and `callable` reads more
accurately for a value that captures nothing at all, such as a reference to a static method.

`bind`, `bindTo` and `call` still exist, called with the same method-call syntax, now as builtin
operations on an opaque type rather than inherited methods of a base class a program could name in an
`is` test or extend. `Closure::fromCallable` is dropped, because after that narrowing there is
nothing left for it to normalise away from.

A rebind gives `$this` an object of the class the anonymous function's body was checked against, or
of a subclass of it, and nothing else. `$fn->bindTo($obj)` and `$fn->bind($obj)` are one operation
under two names: they take exactly one `?object` and return a callable of the receiver's own type.
`$fn->call($obj, ...$args)` rebinds the same way and calls the result, and its type is `mixed`. A
callable that does not use `$this` comes back unchanged
(`rule:statements/an-anonymous-function-captures-this-only-where-it-uses-it`). One that does is copied
with the new `$this`, and an object of any other class, or `null`, throws a `LogicError`. The test is
made when the call runs, because `callable` does not say whether a callable uses `$this`. It is the
line between a rebind and a memory-safety hole: the compiled body reads `$this` at its own class's
layout. A scope argument does not compile (`E0402`), because a scope opens another class's
`private` members. A static check was weighed and left out: it would need a part of the `callable`
type, naming whether and where the callable uses `$this`, that every assignment and comparison of
callables then carries.

`call_user_func` and `call_user_func_array` are dropped with it. Every `callable` value supports
direct invocation, which is what they existed to route around:

```nvs
$result = $fn($arg);       // replaces call_user_func($fn, $arg)
$result = $fn(...$args);   // replaces call_user_func_array($fn, $args)
```
