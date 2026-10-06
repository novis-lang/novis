`readonly` with an initializer is refused where it is written. `readonly`'s contract is one
assignment, during construction, in the declaring class's own `constructor` — a property whose
single assignment is its own declaration-site default is a per-instance constant, and a value known
at the declaration already has a spelling: `const` (`rule:types/class-constant`). The diagnostic
names it, and names dropping `readonly` as the other fix. Letting the default stand as the one
assignment would make a property spell what `const` spells — two names for one thing, the pattern
`rule:statements/nothing-gets-a-second-name` refuses.

PHP 8.6 allows the combination, chiefly for hooks in interfaces. The fix is to move the default into
the constructor, or make it a `const`.
