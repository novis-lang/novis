How many workers the server runs to accept and serve requests. Each worker runs on its own CPU core.

When the file does not set it, the server starts one worker for each CPU core the machine gives it.
Set a lower number to leave cores free for other programs on the same machine.

The server creates its workers when it starts, so a new value takes effect only after a restart.
Until then, the server writes the new value to its log once, in a `configuration restart pending`
line. A program cannot change this setting.
