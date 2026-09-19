`if` runs a block of code when its condition is true. `else` gives the block that runs when it is
not. `elseif` adds a further condition to test in between. You may write `else if` instead, and the
two are the same.

The arms are tested from top to bottom, and only the first one whose condition is true runs. `else`
comes last and takes everything the arms above it did not.

The condition is any expression, and it does not have to be a `bool`. The value is read as a
condition: `0`, `0.0`, `""`, `"0"`, `[]`, `null` and `false` count as false, and every other value
counts as true.

A body of one statement needs no braces. `if ($n > 1) echo "big\n";` is allowed. Braces around every
body are still the safer habit.

**The examples below** show a chain that picks one arm, then conditions that are not `bool` values,
then a shipping price worked out from several rules in order.
