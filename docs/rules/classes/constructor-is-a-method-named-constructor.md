The constructor is a method named `constructor` — an ordinary lowercase-first name that needs no
casing exception of its own. It has every role a constructor has: `new` invokes it, it is
where `rule:classes/definite-property-initialization`'s obligation attaches, and a subclass
discharges its inherited properties by calling `parent::constructor(...)`. It is never `static`
(`E0106`): `new` runs it on the object just made, and a static method has no `$this` to assign a
property through.

`__construct` is not recognized as a constructor and cannot compile as an ordinary method either,
because two leading underscores fail the identifier pattern
(`rule:classes/no-leading-underscore-identifiers`). It gets its own diagnostic rather than the
generic mis-casing one, since the mechanical rename would otherwise suggest `construct` and lose
the intent.

Choosing a real word instead of a reserved spelling is what removes the last carve-out from the
casing check: nothing in the identifier grammar has an exception any more. The cost is one fixed
mechanical rename for every converted class, which is the cheapest kind of break this project takes.

**Declined:**
- **Keep `__construct` as a reserved exception to the casing check.** That is a single-name
  carve-out, the kind `rule:core-api/casing-has-no-suppression` declines to generalize from;
  removing the reason for the exception is cleaner than narrowing its wording.
- **Another spelling: `init`, `new`, `__init__`.** `constructor` is what a PHP, Java or TypeScript
  reader already expects and needs no invented vocabulary. `__init__` fails the identifier pattern
  for the same reason `__construct` does, and `init` or `new` could collide with an ordinary method
  that has its own meaning.
