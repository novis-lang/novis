`break` stops a loop immediately. `continue` skips the rest of the current iteration and starts the
next one.

Both act on the loop closest to them. Inside a `switch`, `break` leaves the `switch`, and a plain
`continue` starts the next iteration of the loop around it, because a `switch` has no iterations of
its own.

Write a number after either word to leave more than one level at once. `break 2` leaves the loop
around the loop you are in. Levels count from 1 at the closest statement, and every loop and every
`switch` is one level. The number has to be written directly in the code, so a variable is not
allowed there.

**Good to know:** a level with no matching loop or `switch` does not compile. A wrong number is an
error you see before the program runs.

**The examples below** take these in turn: a search that stops at the first match, a loop that skips
the rows it does not want, and an order check that leaves two loops at once.
