Reads the value of a class constant when the constant's name is in a string.

`constant($name)` returns the value of the constant `$name`, including one the class inherits.
The result has the type `mixed`, so convert it with `as`. A constant can be read only where a
normal read of it would be allowed: a private constant cannot be read from outside its class.

Two kinds of error are thrown. A name the class has no constant for throws a `LogicError`. A
constant you may not read throws a `RuntimeError`. A constant declared `secret` is never
returned, not even inside its own class, because a `mixed` value cannot stay secret.

**The examples below** show two constants read by name, the two errors, and a plan's limit read
from the plan name stored with a customer.
