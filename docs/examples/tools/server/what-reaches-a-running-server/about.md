A running server applies a change to your code or to its configuration without a restart. Four
configuration keys are the exception.

The server checks the files of a program regularly, and `[opcache] revalidate_freq` sets how often.
When a file changed, the server waits until the files stay unchanged for the time in
`[opcache] settle`. It then compiles the whole program again, and the next request runs the new
code. A request that is already running finishes on the old code.

The server checks its configuration files every two seconds. When you save a file, the server reads
and checks all of them, and then the next request uses the new values. The server writes each
reload to its log, with the keys it changed. A file with an error is logged once, and the server
keeps the configuration it has.

`[server] listen`, `[server] socket_mode`, `[server] workers` and `[server] watchdog_margin` need a
restart.

**Good to know:** when the new code does not compile, the requests that reach it fail with the
compile error. The server does not run the older version. The requests work again after you fix the
file.
