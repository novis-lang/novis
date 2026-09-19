What a class adds so that its objects can be written as text.

A class that implements `Stringable` declares one method, `toString()`, returning the text its objects
should appear as. From then on `echo`, joining with `.`, a value inside `"{...}"`, and an `as string`
conversion all call it, so you hand the object itself to anything that builds a line of text instead
of keeping a second field beside it. A function can also ask for `Stringable` rather than a class
name, which means "anything that can be written as text".

**Good to know:** a class without `toString()` is refused where it is rendered, and the message names
`Stringable` as the fix. The method has no special spelling — it is `toString`, not `__toString` — so
you may also call it by name like any other method.
