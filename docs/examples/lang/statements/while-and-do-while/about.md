`while (condition) { … }` runs its body again and again for as long as the condition is true. The
condition is tested first, so a condition that is false at the start means the body never runs.

`do { … } while (condition);` runs the body first and tests the condition afterwards. The body
therefore always runs at least once. Note the semicolon after the closing bracket.

`break` leaves the loop. `continue` skips the rest of the body and goes to the next test. In a
`do … while` loop, `continue` goes to the test at the bottom, and does not restart the body.

The condition is read the same way an `if` condition is. It does not have to be a `bool`.

Use `while` when you do not know how many rounds you need, such as reading until nothing is left.
Use a `for` or a `foreach` loop when you are counting or walking a collection.

**The examples below** show a `while` loop that drains a queue of work, then a `do … while` loop
that always runs once, then a retry that stops on success or after a limit.
