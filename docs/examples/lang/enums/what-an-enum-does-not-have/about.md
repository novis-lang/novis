Says what an enum does not have, and what to write instead.

A case is a number with a name, not an object. It has no `->name` and no `->value`, and an enum has
no `cases()`, `from()` or `tryFrom()`. An enum has no properties and cannot implement an interface.

You write these things yourself. `$case as int` gives the number of a case. `$n as Level` and
`$n as ?Level` turn a number back into a case. For the name of a case, give the enum a method with a
`match` on `$this`. For a list of every case, give it a static method that returns `array<Level>`.
The compiler does not check that the `match` names every case. A case with no arm throws a
`LogicError` when it reaches the `match`.

**The examples below** show the name of a case, then the list of every case with a lookup by name,
then an enum that carries the behaviour of an order. Examples 04 and 05 show that `->name` and
`cases()` do not compile.
