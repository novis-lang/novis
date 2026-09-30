The part of a request's CPU time limit that is kept for the code that reports a limit error.

A request that uses all of its CPU time, `[limits] cpu_time`, stops. The function you passed to
`Core\Fatal::onLimit` then runs to report what happened, and it needs some time to do that. This
setting keeps that time free. `[limits] fatal_reserve_memory` does the same for memory.

The reserve is taken from the CPU time limit and is not added to it. The default is 50
milliseconds, which is enough to build a message and write it to a slow log. The reserve is never
more than a quarter of the CPU time limit.

Only the person who runs the server sets this value. A program cannot change it, and
`Core\Config::set` returns `false`.

The example prints the CPU time limit and both reserves, and then tries to change all three. The
program may lower its own CPU time limit. Both reserves stay the same.
