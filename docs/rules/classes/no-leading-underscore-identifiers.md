No property, parameter or local variable name may begin with `_`. All three categories use the same
`camelCase` pattern methods have always used, and there is no allowance left to state for them.

The allowance existed only to spare a habit — `_privateField`, `$_unused` — that converts by
dropping one character. Keeping it would have preserved exactly the single-name carve-out the casing
rule argues against generalizing from, for comfort this project already declined to buy elsewhere.
Removing it, together with respelling the constructor
(`rule:classes/constructor-is-a-method-named-constructor`), leaves the identifier check with zero
exceptions of any kind.

What it costs is a mechanical rename per underscore-prefixed name during conversion, and one habit
a contributor used to it has to unlearn at the point the compiler names it.
