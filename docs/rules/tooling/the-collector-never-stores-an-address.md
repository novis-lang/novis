No sender's address is written to disk, by the program, the Novis server or the TLS proxy in front of
it. The address lives in memory for one request, and once more as a keyed hash in the rate limit.

- **The rate limit.** Each report calls `Core\RateLimit::shed` with ten reports per hour per address.
  The key is `Core\Hash::hmac` of the address under a 32-byte salt from `Core\Random`. The salt is
  kept in `Core\Cache::memory()` and expires at the end of each UTC day, so a key cannot be linked
  across days and nothing reverses it. `shed` keeps its count in the serving process's memory
  (`rule:core-classes/ratelimit-two-members`). A refused report gets `429` with `Retry-After`.
- **The server's log.** The application's `nvs.toml` sets `[log] client_ip = false`
  (`rule:http-server/a-log-line-can-leave-out-the-client-address`), so not even a `5xx` line carries
  the address. The program never passes the address to `Core\Log`.
- **The proxy.** The deploy guide's proxy configuration has no access log, and the Novis server
  trusts it as its one proxy, so `Core\Request::clientIp()` is the sender and not the proxy.

An office of many developers behind one address sends a few reports a week, far under the limit.
A flood from one address is cut off within the first ten reports of an hour. A flood from many
addresses is not stopped, and the value cap bounds what each report can add
(`rule:tooling/a-telemetry-report-is-checked-against-a-closed-schema`). That damage is accepted, and
an identifier that would stop it is not.

A test sends reports from a known address and then reads every table and every log line the test
server wrote, and the address is in none of them.

- **Memory:** per process, one GCRA timestamp per address that sent a report in the last hour, and
  one 32-byte salt.

**Not on disk.** There is no `apps/telemetry/`.

**Declined:**
- **Storing a hashed address to count unique installs.** The weekly gate already makes one report
  one install, and a stored hash with a known salt is an address list that is easy to rebuild.
- **Rate limiting in the proxy alone.** The promise would then depend on configuration nobody can
  test with the program. The limit in the program is tested with it.
- **No rate limit.** One script could then write any number into the public totals.
