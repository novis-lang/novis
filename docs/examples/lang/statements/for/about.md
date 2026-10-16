A `for` loop counts. Its header has three parts, separated by semicolons: where the counter starts,
how long the loop runs, and how the counter changes after every round. `for (var $i = 0; $i < 10;
$i++)` starts at 0, runs the body while `$i` is below 10, and adds one to `$i` after each round.

The first part is either one new variable, or assignments to variables declared above the loop.
`var $i = 0` makes an `int`, or you write the type, as in `uint $i = 0`. The second and third parts
may each be a list separated by commas, so one loop can move two counters. Any part may be empty,
and `for (;;)` runs until a `break`.

The counter belongs to the function, not to the loop. You can read it after the loop, where it holds
the value that ended it. A second loop in the same function needs a different name. `continue` runs
the third part before the next test.

**The examples below** show a `var` counter that numbers a list, two counters that meet in the
middle, and `uint` counters that send rows in batches.
