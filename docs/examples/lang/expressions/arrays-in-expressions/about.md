An array holds a list of values, or a table where every value has a name. This is the syntax for
building one, reading it and changing it.

A literal is written `[10, 20]` or `["city" => "Graz"]`. `var $prices = [10, 20]` takes the type
from the values, so `$prices` is an `array<int>`. An empty array, or one whose values have different
types, needs the type written: `array<int> $prices = []` or `array<int|string> $row = [1, "a"]`.
`$prices[] = 30` adds a value at the end. Every key is text, so `$prices[8]` and `$prices["8"]` are
one entry. Reading a key that is not there throws a `RuntimeError`, so write `$prices["milk"] ?? 0`
where a missing key is normal, and `isset($prices["milk"])` to test for one.

**Good to know:** an array is a value. Assigning it, passing it to a function and returning it each
give a copy, so a change on one side is never seen on the other.

**The examples below** build a list and a price table with `var` first and a written type after,
then read keys that may be missing, then group rows for a small report.
