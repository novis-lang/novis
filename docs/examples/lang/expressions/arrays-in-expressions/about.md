An array is a list of values, or a table where every value has a name.

You write an array as `[10, 20]` or `["city" => "Graz"]`. `var $prices = [10, 20]` takes the type
from the values, so `$prices` is an `array<int>`. An empty array, or one whose values have different
types, needs the type written: `array<int> $prices = []`. `$prices[] = 30` adds a value at the end.
Every key is text, so `$prices[8]` and `$prices["8"]` are one entry. Reading a key that is not there
throws a `RuntimeError`. Write `$prices["milk"] ?? 0` where a missing key is normal.

**Good to know:** an array is a value. Assigning, passing or returning it gives a copy, so a change
on one side is never seen on the other.

**The examples below** build a list and a price table, read missing keys and group rows.
Example 04 shows keys, spread and nested arrays, 05 destructuring, and 06 does
not compile.
