Changing a value into another type is written one way in Novis: `as`, followed by the type you want.

A conversion gives a value of that type, or it throws an error. It never rounds, never drops a
fraction, and never uses a zero or an empty string instead. With `as ?int`, the result is `null` where the plain form would throw. Add `??` to
use a default value instead.

`as` also checks an object against a shape, such as `as Point`, and turns a class name into a class
reference with `as class<T>`.

**Good to know:** `as` converts only the value directly before it, so `$a + $b as float` converts
`$b` alone.

**The examples below** read numbers out of text from outside the program, add up an order total, and
skip a row that throws an error. Then they show which error each conversion throws and choose a class
by its name. The last two do not compile.
