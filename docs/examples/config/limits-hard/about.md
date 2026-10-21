The ceiling a program cannot raise itself past.

`[limits]` sets what a request starts with. `[limits.hard]` uses the same keys and sets the most a
program may ask for with `Core\Config::set`. A value above the ceiling returns `false`, and the
previous value stays in force. The program keeps running.

When `[limits.hard]` does not set a key, the ceiling is the value the request started with. A
program can then lower that limit, but it cannot raise it. There is one exception. A program you
start from the command line may raise its `memory` limit to any value when `[limits.hard]` does
not set `memory`. When `[limits]` does not set `memory`
either, the start value is `128M`. `false` removes a ceiling.

You can lower or raise a ceiling while the server runs. Edit the file and save it, and the server
applies it. A request can never change a ceiling. An application may have a lower ceiling of its
own than the rest of the server.
