The accepted dialect is **five-field POSIX cron** — minute, hour, day-of-month, month, day-of-week —
plus exactly five named shorthands: `@hourly`, `@daily`, `@weekly`, `@monthly`, `@yearly`. Each
shorthand expands to its five-field form before anything reads it, so `@daily` and `0 0 * * *` are the
same schedule and nothing downstream can tell them apart. There is no seconds field and none of
Quartz's `L`, `W`, `#` or `?`.

The exclusions are the point. A seconds field turns a scheduler into a timer, which is a different
feature with a different accuracy contract, and every dialect that adds one also adds the operators
nobody can read six months later. Work that needs sub-minute cadence is an `@hourly` script holding a
loop, which is visible and debuggable; "the last weekday of the month" is a `@daily` script with a
date check in it.

The expression is parsed and validated **at boot**, with the offending line named — a typo'd schedule
must not be discovered by its silence. The one parse also answers the next fire: the scheduler never
re-reads the string, because two readers would be two dialects, and the two disagreeing is an entry
that booted and fires at the wrong minute, which nothing observes. When day-of-month and day-of-week
both narrow, POSIX's own rule holds and the entry fires on either.
