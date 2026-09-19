An array holds a list of values, or a table where every value has a name. This is the syntax for
building one, reading it and changing it.

A literal is written `[10, 20]` or `["city" => "Graz"]`, and it always needs a declared type, such as
`array<int> $prices = [10, 20]`. `$prices[] = 30` adds a value at the end. Every key is text, so
`$prices[8]` and `$prices["8"]` are one entry. Reading a key that is not there throws a
`RuntimeError`, so write `$prices["milk"] ?? 0` where a missing key is normal, and
`isset($prices["milk"])` to test for one.

**Good to know:** an array is a value and not a handle. Assigning it, passing it to a function and
returning it each give a copy, so a change on one side is never seen on the other.

**The examples below** build a list and a price table first, then read keys that may be missing,
then group rows for a small report.
