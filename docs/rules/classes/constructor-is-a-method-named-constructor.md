The constructor is a method named `constructor` — an ordinary lowercase-first name that needs no
casing exception of its own. It has every role a constructor has: `new` invokes it, it is
where `rule:classes/definite-property-initialization`'s obligation attaches, and a subclass
discharges its inherited properties by calling `parent::constructor(...)`.

`__construct` is not recognized as a constructor and cannot compile as an ordinary method either,
because two leading underscores fail the identifier pattern
(`rule:classes/no-leading-underscore-identifiers`). It gets its own diagnostic rather than the
generic mis-casing one, since the mechanical rename would otherwise suggest `construct` and lose
the intent.

Choosing a real word instead of a reserved spelling is what removes the last carve-out from the
casing check: nothing in the identifier grammar has an exception any more. The cost is one fixed
mechanical rename for every converted class, which is the cheapest kind of break this project takes.
