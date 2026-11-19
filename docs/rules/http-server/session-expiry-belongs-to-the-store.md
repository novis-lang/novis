`[session] ttl` is written onto the entry: `PEXPIRE` on the session's hash on the shared tier,
which `RENAMENX` keeps, and an `expires_at` column on the session's head row in the `db` store's
SQLite file. An expired session is absent at once, so `load` already
answers it and `start` already issues a fresh id
(`rule:http-server/a-session-store-answers-five-operations`). On the SQLite file `load` reads only
a row whose `expires_at` is still ahead, so correctness never waits for a sweep. `save` refreshes
the expiry, so `ttl` bounds how long an *untouched* record survives.

The SQLite file deletes its expired rows with one indexed `DELETE … WHERE expires_at <= now`, run
by a write at most once a minute per core. That sweep bounds the file at O(live sessions) rather
than O(sessions ever issued). It is driven by time, never by a chance per request, and it is one
statement on the blocking pool beside the write that ran it.

**Declined:**
- **A probabilistic collection, as PHP's `session.gc_probability`.** Which request pays for it is
  chance, and on a quiet site the expired rows stay for days. A sweep by time costs every core the
  same small delete once a minute.
- **A sweep run by a scheduled job.** It would need a `[[schedule]]` entry nobody wrote, so a
  deployment with no configuration would never sweep.
- **Moving the SQLite store's reads and writes off the core thread.** Each is one indexed statement
  on a local file, and no measured program waits on it. It is reconsidered when one does.
