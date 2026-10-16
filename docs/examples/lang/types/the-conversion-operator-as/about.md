Changing a value into another type is written one way in Novis: `as`, followed by the type you want.

A conversion gives you a value of that type, or it throws an error where it was written. It never
rounds, never drops a fraction, and never uses a zero or an empty string because the value does not
fit. Text becomes a number only when the whole text is a number. Write a question mark in front of
the type, as in `as ?int`, and the result is `null` where the plain form would throw. Add `??` to
use a default value instead.

`as` also checks an object against a shape, such as `as Point`, and turns a class name into a class
reference with `as class<T>`.

**Good to know:** `as` converts only the value directly before it, so `$a + $b as float` converts
`$b` alone.

**The examples below** read numbers out of text that came from outside the program, then give a
default or list the answers that are not numbers. Last, they add up an order total and skip a row
that throws an error.
