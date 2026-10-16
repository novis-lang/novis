The part of a request's memory limit that is kept for the code that reports a limit error.

Every request has a memory limit, `[limits] memory`. A request that reaches the limit stops. The
function you passed to `Core\Fatal::onLimit` then runs to report what happened, and it needs memory
too. This setting keeps that memory free. Normal code cannot use it.

The reserve is taken from the memory limit and is not added to it. A larger reserve leaves less
memory for the program. The default is 1 MiB. The reserve is never more than a quarter of the
memory limit.

Only the person who runs the server sets this value. A program cannot change it, and
`Core\Config::set` returns `false`.

The example prints the memory limit and the reserve, and then tries to change both. The reserve
stays the same. The program may lower its own memory limit.
