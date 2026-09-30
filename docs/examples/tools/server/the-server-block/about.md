The `[server]` block of `nvs.toml` sets where `nvs serve` listens, how many requests it runs and
how long it waits.

Every key is optional. With no block, the server listens on `127.0.0.1:8000` and uses one worker
for each CPU core. `listen` is a list. An entry is an address with a port, or the path of a Unix
socket. `max_in_flight` is the number of requests that may run at the same time. A request over
that number gets the response `503`.

Four keys are timeouts for a connection that sends nothing or reads nothing. A slow upload that
still sends data is not closed. `drain_timeout` is the longest time that a connection with no
request stays open after a stop. You cannot turn a timeout off: `false` and `0` are errors.

**Good to know:** a running server applies a change to this block without a restart. `listen`,
`socket_mode` and `workers` are the exception and need a restart.

**The example below** reads two keys that the file sets and one key that it does not set.
