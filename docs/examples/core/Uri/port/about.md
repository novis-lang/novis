Returns the port of an address as a number. The port is the number after the host, such as `8080`
in `http://localhost:8080/`.

The result is `null` when the address has no port. `$uri->port()` does not fill in a default port,
so for `https://example.com/` the result is `null` and not `443`. Use `??` to choose the port your
program needs in that case. The result is also `null` when a `:` has no digits after it, as in
`http://example.com:/`.

A port is always a number from `0` to `65535`. `Core\Uri::parse` throws an error for a larger
number, so `port` never returns one.

**The examples below** read a port, show when the result is `null`, and choose the port a program
connects to.
