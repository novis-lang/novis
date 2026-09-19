Says what to write instead of PHP's enum methods and properties.

A case is a number with a name, not an object. It has no `->name` and no `->value`, and an enum has
no `cases()`, `from()` or `tryFrom()`. An enum body holds cases and nothing else, so a method, a
constant and an `implements` clause do not compile either.

Everything those members did is written outside the enum. `$case as int` gives the number behind a
case. `$n as Level` and `$n as ?Level` turn a number back into a case. For the name of a case, for a
list of every case, or for any behaviour at all, write a class that takes the enum and put a `match`
in it. That `match` is checked against every case of the enum, so a case added later stops the class
at compile time instead of at the first request that reaches it.

**The examples below** show the name of a case, then the list of every case with a lookup by name,
then a class that carries the behaviour of an order.
