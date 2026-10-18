Inside a `switch` that is inside a loop, `continue`, `continue 1` and `continue 2` do the same thing.
Each one starts the next iteration of the loop.

`break 1` is the same as `break`, and `continue 1` is the same as `continue`. The smallest level is 1,
so `break 0` does not compile.

These programs do not compile, because no statement matches the level:

- `break` or `continue` outside any loop or `switch`.
- `continue` inside a `switch` with no loop around it.
- `break 3` inside only one loop.
