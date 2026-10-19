A `for` loop counts. Its header has three parts, separated by semicolons: where the counter starts,
how long the loop runs, and how the counter changes after every round. `for (var $i = 0; $i < 10;
$i++)` runs the body while `$i` is below 10.

The first part is either one new variable, or assignments to variables declared above the loop. The
second and third parts may each be a list separated by commas. Any part may be empty, and `for (;;)`
runs until a `break`.

The counter belongs to the function, not to the loop. After the loop, it has the value that ended
it. A second loop in the same function needs a different name. `continue` runs the third part before
the next test.

**The examples below** show a `var` counter, two counters that meet in the middle, `uint` counters,
a `for (;;)` loop, and a header that does not compile.
