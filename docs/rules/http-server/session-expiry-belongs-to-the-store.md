`[session] ttl` is written onto the entry — `SET … EX` on the shared tier; an `expires_at` column
and a `DELETE … WHERE expires_at < now()` in the database backend's own schema, run by the worker
`rule:concurrency/who-runs-a-job-is-configuration` already names. There is no probabilistic
collection and no sweep on a percentage of requests: such a sweep exists only for a store that
cannot expire anything by itself, such as a directory of files, and both backends here can.

An expired record is absent, so `load` already answers it and `start` already issues a fresh id
(`rule:http-server/a-session-store-answers-four-operations`). `save` refreshes the expiry, so
`ttl` bounds how long an *untouched* record survives. Expiry needs no code path of its own, which
is the point of choosing backends that expire.
