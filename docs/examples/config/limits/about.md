What one request or one script starts with: the memory it may hold, the CPU and wall time it may
spend, the bytes it may write back, and the other budgets it runs inside.

These are the ordinary per-request defaults, so a program is allowed to change its own. A report that
genuinely needs a bigger heap for one route asks for it, and a program that knows it should stay
small can say so and be held to it — which is worth doing, because a limit a program set for itself
still stops it, and stopping early is how a runaway is caught before it takes the host with it.

What bounds that freedom is the companion block, `[limits.hard]`: the ceiling the operator sets, that
no program can raise itself past. Asking for more than the ceiling allows is refused on the spot and
the previous value stays in force, so a program that asks for too much keeps running under what it
already had rather than failing. A few keys written beside these are the operator's alone, and they
say so where they are documented.
