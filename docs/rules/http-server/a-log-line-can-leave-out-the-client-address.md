`[log] client_ip` is `true` by default, and `false` keeps the client address out of every record the
server writes: the access line, the `5xx` line that is written whatever `access` says, and every
diagnostic about a forwarded header. The field `clientIp` is left out of those lines, not written as
`null`, so a reader cannot mistake a removed address for a request that had none. Spans and metrics
carry no client address in either setting. `Core\Request::clientIp()` returns the address as before,
because a program that limits a rate or checks an allow-list needs it in memory, and what it writes
to its own log is its own code.

It is a `System` directive, applied at reload. A request that could turn it back on would undo the
promise an operator made to the people whose addresses these are, so only the file sets it.

Why it exists: an operator who promises that no address is stored needs a setting that holds for every
line, including the `5xx` line `rule:http-server/the-access-log-is-a-mode-default` writes
unconditionally. Turning the access log off is not enough. The telemetry collector is the first program
that makes this promise (`rule:tooling/the-collector-never-stores-an-address`).

- **Memory:** none. The setting removes one field from a record that is written anyway.

**Not on disk.** `[log]` has no `client_ip` key, and `crates/nvs-server/src/trace.rs` always writes
`clientIp`.

**Declined:**
- **A masked address, such as the first three bytes of an IPv4 address.** It is a second format to
  get right for IPv6 and for every proxy shape, and a masked address is still personal data in many
  places. An operator who wants the address keeps it whole, and one who does not leaves it out.
- **Leaving the address out of `Core\Request::clientIp()` too.** The program then cannot limit a rate
  per address at all, and the setting is about what is written, not what is known for one request.
