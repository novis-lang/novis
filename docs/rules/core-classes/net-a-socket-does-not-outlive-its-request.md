A socket opened through `Core\Net` closes with the request that opened it. There is no persistent
connection, no pool held across requests and no handle a later request can find, which is
`rule:security/no-cross-request-state` applied to sockets, the subsystem where a persistent connection
is the most tempting shape: no member opens one.

What this spends, per `rule:programs/memory-priority`: one reactor registration per open socket,
attributable to the request that opened it and released with its arena. The process therefore holds
one registration per socket **in flight** and nothing per socket served, which is the O(in-flight)
bound that separates a cost from a leak.

A program that wants a connection to survive a request wants a store, and the stores are the ones
already granted by name — `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s shared
cache and `rule:core-classes/db-one-api`'s database — where the endpoint is an operator's and the
lifetime is the runtime's.
