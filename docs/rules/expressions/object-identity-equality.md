`$a == $b` on two class instances asks whether they are the same object. It never walks properties.

**Six `Core\Time` classes compare by value:** `Date`, `TimeOfDay`, `Instant`, `Duration`, `Zone` and
`DateTime`. Two instances of one of them are `==` when they are the same class and their value slots
are equal. A `Date` compares its year, month and day, a `TimeOfDay` its hour, minute, second and
nanoseconds, an `Instant` its second and nanosecond, a `Duration` its nanoseconds and a `Zone` its id.
A `DateTime` compares its moment alone, so 10:00 in `Europe/Berlin` and 09:00 in `Europe/London` on one
day are `==`, though they print different text. `ObjectMap` and `ObjectSet` find a key with `==`, so an
equal value is the same key.

The decline of a property walk below does not reach these six. Each is `Core`'s, immutable and a
value, so identity tells a program nothing it can use: two `Date::at(2026, 1, 5)` calls are one day.
The slots are a fixed list of one to four scalars and strings, so the cost is bounded and known, and no
class of a program can join the list. Every reader of `==` agrees: a typed pair, a `mixed` pair, an
array, `match`, `Core\Arr`'s membership members and the two collections' keys.

**What it spends.** `==` on one of these classes reads one to four slots of each operand in place of one
pointer compare, a few nanoseconds per comparison. A typed `==` whose operand may hold one of them
(the class, `object`, `Comparable`, `Stringable` or `Parses`) calls a helper in place of the inline
pointer compare, and so do two user objects typed `Comparable`. A collection keyed by such a type hashes
the slots in place of the address. Nothing changes for any other class.

There is **no `__equals`, no `equals()` protocol and no `Equatable` interface**. A per-class equality
hook would make `==` mean something different in every file, and an ambient, undeclared property walk
has unbounded cost in the object graph's size with no way for a class to opt out.

Two named ways to ask the other question already exist:

- **`$a->compareTo($b) == 0`**, where the class implements `Comparable`. A class with a meaningful
  notion of "same value" almost always has a meaningful order too, and this gets both from one
  declaration.
- **`Core\Test::assertEqualsDeep`** in a test, which walks properties, arrays and shapes and produces
  a diff. It is a test member on purpose: a structural walk is a debugging affordance, not something a
  request path should reach for by accident.

A class needing content equality in production and wanting no order writes an ordinary named method.
That reads worse than `==` by exactly one call, and it is visible at the call site. A value class ported
from PHP, whose `==` compared two objects by their properties, pays that call and a `Comparable`
declaration.

**Declined:**
- **Structural equality, PHP's `==` reading.** Its cost is unbounded in the object graph's size, it is
  ambient and undeclarable, and no class can opt out: the same refusal `rule:classes/comparable` makes
  for ordering.
- **An `Equatable` interface or an `__equals` hook.** `==` would mean something different per class,
  which is the property removed from every other magic method.
- **A separate identity operator, so `==` could be structural.** That is two equality operators again.
  Identity is the reading a reference type wants by default, and content comparison is the one that
  gains from being visible at the call site.
- **Value equality for more `Core` classes, or a list a program's class can join.** Each addition is a
  class whose `==` changes meaning, and an open list is the per-class hook declined above. The six are
  the immutable values whose identity no program can use.
- **A content-equality member in `Core`.** None exists. One would be a method, never an operator or an
  interface, and the case for it is `compareTo(…) == 0` repeated in real programs.
