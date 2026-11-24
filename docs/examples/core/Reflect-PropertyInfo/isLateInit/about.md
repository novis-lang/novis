Tells you whether a property is declared `lateinit`.

`isLateInit()` returns `true` for a property declared `lateinit`. It returns `false` for every
other property. A `lateinit` property that a class inherits from its parent class returns `true`
too.

A `lateinit` property may have no value yet. If your program reads it then, the read throws a
`LogicError`. Checking `isLateInit()` first lets your program skip those properties.

**The examples below** print which properties of a class are `lateinit`, show a `lateinit`
property from a parent class, and save an object as text without its `lateinit` properties.
