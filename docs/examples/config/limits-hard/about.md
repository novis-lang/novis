The ceiling a program cannot raise itself past.

`[limits]` says what a request starts with, and a program is free to change its own. `[limits.hard]`
is the operator's answer to that freedom: the same budgets written a second time, as the most a
program may ever ask for. Asking for more is refused where the request is made and the previous
value stays in force, so a program that overreaches keeps running under what it already had.

The two blocks hold the same key names on purpose, so a deployment reads as one sentence — start
requests at this much, and never let one have more than that. A budget with no ceiling written is
one a program may set freely, which is the right default for anything that does not threaten the
host and the wrong one for anything that does.

A ceiling can be lowered or raised while the server is running, by editing the file and reloading;
it is never a value a request can move by any spelling. An application may be given a tighter
ceiling of its own than the rest of the deployment gets.
