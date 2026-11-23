A session record has two clocks, and the store keeps both
(`rule:http-server/a-session-value-is-short-lived-or-long-lived`). **The short clock** runs for
`[session] short_lived`, two hours by default, and a short-lived value is visible only while it runs.
**The long clock** runs for `[session] long_lived`, 180 days by default. The record expires when the
long clock ends, or when the short clock ends where the record has no long-lived value.

The record's expiry is written onto the entry: `PEXPIRE` on the session's hash on the shared tier,
which `RENAMENX` keeps, and an `expires_at` column on the session's head row in the `db` store's
SQLite file. The short clock is a field of the record, `x` on the hash and `short_expires_at` on the
head row, as epoch milliseconds. An expired record is absent at once, so `load` already answers it and
the first use opens a fresh session (`rule:http-server/a-session-store-answers-six-operations`). On
the SQLite file `load` reads only a row whose `expires_at` is still ahead, so correctness never waits
for a sweep.

**A write-back always moves the short clock.** The long clock moves only where the identifier
cookie is sent with it, so the store and the browser forget the record together: at the first
long-lived write, at a long-lived write past half of its time, and by the half rule below. A save
moves the long clock forward only, so a request that loaded an older record never shortens one
another request refreshed. **A save also holds each clock to now plus its lifetime**: the short
clock becomes now plus `short_lived`, and the long clock is never left past now plus `long_lived`.
A clock stored far ahead, by a host whose clock ran fast or a lifetime later made shorter, comes down
at the next save rather than keeping the record past its lifetime. **A read refreshes a clock when less than half of its time is
left**, and then the session counts as changed, so the end of the request writes it back. A signed-in
user who only reads stays signed in while they keep reading. The half rule costs at most one write per
clock per half-lifetime of an active session, and a request that reads a fresh session still makes one
round trip and no write. A refresh that needs a cookie and cannot send its header, because the first
use was inside `Core\Html::later` or after the response, is skipped, and a later request does it.

The SQLite file deletes its expired rows with one indexed `DELETE … WHERE expires_at <= now`, run
by a write at most once a minute per core. That sweep bounds the file at O(live sessions) rather
than O(sessions ever issued). It is driven by time, never by a chance per request, and it is one
statement on the blocking pool beside the write that ran it. A record with a long-lived value is live
for `long_lived`, so the file holds one row per browser that kept a value within that time: it grows
with visitors, and the long clock bounds it.

**Declined:**
- **A refresh on writes only.** A user who only reads would be signed out two hours after their last
  write, however active they are.
- **A refresh on every read.** Every request of a session would write, and a read-only request would
  make two round trips where one does.
- **A probabilistic collection, as PHP's `session.gc_probability`.** Which request pays for it is
  chance, and on a quiet site the expired rows stay for days. A sweep by time costs every core the
  same small delete once a minute.
- **A sweep run by a scheduled job.** It would need a `[[schedule]]` entry nobody wrote, so a
  deployment with no configuration would never sweep.
- **Moving the SQLite store's reads and writes off the core thread.** Each is one indexed statement
  on a local file, and no measured program waits on it. It is reconsidered when one does.
