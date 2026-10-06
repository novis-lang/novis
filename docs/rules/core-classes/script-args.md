`Core\Script::args(): mixed` is the deep-copied value the current isolate was spawned with, and
`null` where there was none — a child spawned without the option, and the root script, which nothing
spawned.

The type is `mixed` rather than `array<mixed>` because a spawn's argument accepts any value that can
cross the boundary, decided at run time, so nothing narrows the option at the call site. The answer
is `null` rather than an empty array because a program that wrote `args: []` said something a program
that wrote no option did not.

This is Novis's own superglobal being retired for the same reason every other superglobal is refused,
and consistency is the whole of the reason: an ambient, undeclared variable is the shape being closed, and one the project
introduced itself is no better for having been introduced deliberately. Each isolate's arguments are
its own.
