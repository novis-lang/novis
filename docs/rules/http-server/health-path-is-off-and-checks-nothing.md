`[server] health_path` is off by default, so no URL is silently reserved. When set it is matched ahead of every mount, answers `200` while the process is accepting and `503` while it is draining, with an empty body, and is skipped by the access log.

It performs **no dependency checks** — a health endpoint that pings the database converts a slow database into a simultaneous outage across every instance — and reports no version or build information. A built-in probe reports that the *process* is alive even when the application fails to compile, where an application-route probe would fail and produce a restart loop that cannot fix a compile error.

`Core\Server::isDraining()` gives an application the same bit for an endpoint of its own; a program that is not being served reads `false`, which is the answer rather than an error. What draining does to the connections still open is `rule:concurrency/a-drain-closes-a-connection-cleanly`.
