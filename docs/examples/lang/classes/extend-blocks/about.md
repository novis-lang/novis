An `extend` block adds methods, constants and properties to a class, an interface or an enum that `Core` or another package declares.

You write `extend Core\Time\Date as BusinessDays { … }`, and the methods inside work on every `Date`. Inside them, `$this` is the `Date`. A file sees the added methods only when it has the line `use Shop\BusinessDays;`. This is true even in the file that declares the block. So each file shows where its methods come from.

An added method can only add a new name. It cannot replace a method the type already has. The type's `private` members stay private, and the block sees only what other code sees. A call of an added method costs the same as a call of a static method.

**Good to know:** only code that knows the static type finds an added method. A `mixed` value, `Core\Reflect`, JSON and `clone` do not see it.

**The examples below** add business days to `Date`, store a value on the route of a request, and show a package that allows one of its added methods only in its own code.
