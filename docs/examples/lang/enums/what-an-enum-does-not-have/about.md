Says what an enum does not have, and what to write instead.

A case is a number with a name, not an object. It has no `->name` and no `->value`, and an enum has
no `cases()`, `from()` or `tryFrom()`. An enum body has cases and nothing else, so a method, a
constant and an `implements` clause do not compile either.

You write these things outside the enum. `$case as int` gives the number of a case. `$n as Level`
and `$n as ?Level` turn a number back into a case. For the name of a case, for a list of every case,
or for any behaviour at all, write a class that takes the enum and put a `match` in it. The compiler
checks that `match` against every case of the enum. So a case added later is a compile error in the
class, and not an error at the first request that reaches it.

**The examples below** show the name of a case, then the list of every case with a lookup by name,
then a class that carries the behaviour of an order.
