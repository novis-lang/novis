The local tier is per-core with no coherence between cores: a write on one core is not visible on
another, and any entry may be absent at any time. The process tier is coherent across the cores of
one process and no further, so a second process on the same machine answers nothing the first wrote.
So anything a program **relies on the value of** goes to the shared tier or the database — sessions,
locks, idempotency keys, and any counter whose value is acted upon.

This is enforced rather than documented. `Core\Session`'s configurable backends carry an entry for
neither weak tier, so pointing a session at one is a configuration-time error naming the file the key
was written in, rather than a race that appears under load on a second core — or, for the process
tier, on the second process a deployment scales to. Rate limits and the lease a scheduled job takes
have their own members over the shared tier for the same reason, so an application is not left to
arrange coherence for itself.

The test is what a program relies on, not where bytes live
(`rule:concurrency/cross-request-state-is-explicit`), and one thing that looks like a violation is
not one: a per-core metrics registry outlives requests and passes, because no program can read a
metric at all and its values are approximate aggregates merged arithmetically at scrape.
