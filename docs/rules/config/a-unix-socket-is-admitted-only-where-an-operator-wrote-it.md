`[cache.shared] url` and `[db.<name>] host` may name a Unix domain socket. `Core\Net::connect`,
`Core\Db::open`'s program-supplied settings and `Core\Http\Client` may not, and a path reaching any of
them is refused **as a target this deployment has no way to authorize** — not as a file that could not
be opened.

The asymmetry is the address policy's own, read for what it is about. The denied-range table
(`rule:security/net-address-policy`) is not a list of unpleasant networks; it is the mechanism that
keeps a *program-supplied* endpoint off the local machine, which is why loopback heads it. A
program-supplied socket path is a way onto the local machine the table cannot see — there is no
address to match — and the reachable set on an ordinary host is worse than loopback's: it includes
a container daemon's socket, a database's local socket and whatever else a distribution puts in
`/run`. Admitting one would hand a program the exact capability the policy spends a resolution
and a pin to deny.

An operator-written endpoint is the case the policy already distinguishes, for the reason it already
gives (`rule:config/cache-shared-is-the-grant-over-the-configured-store`). A deny-list of socket paths
is not the alternative: the address ranges are few, closed and named by a standard, while the set of
sockets on a host is open, distribution-specific and grows when anything is installed. A check that
must enumerate what to refuse is wrong on the machine nobody tested. The grant a program-supplied
path would need is `rule:config/net-local-is-named-and-not-on-the-roster`.
