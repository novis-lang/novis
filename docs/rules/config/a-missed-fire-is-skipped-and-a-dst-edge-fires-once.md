Every fire is computed from **now**, never counted from the entry's last fire, and that one choice is
the whole implementation of three rules.

**A missed fire is never caught up.** A host that was down, a process that restarted, a machine that
was suspended, a clock that jumped forward — in each case the missed interval is skipped and logged,
and the next scheduled instant fires normally. Catch-up needs a durable record of what has and has not
run, which is a queue, which this is deliberately not
(`rule:concurrency/after-response-outlives-the-connection` draws the same boundary for deferred work).
A wall-clock step forwards costs the intervals it stepped over rather than firing them in a burst.

**`timezone` defaults to `"UTC"`.** There is no ambient timezone anywhere in Novis, so an absent key
is the documented default rather than the host's setting, and a name no IANA database knows refuses
the boot.

**A DST edge fires exactly once.** A local-time schedule landing in a spring-forward *gap* fires once,
at the first valid instant after the gap. One landing in a fall-back *repeat* fires once, on the first
occurrence. Both are decided in the single place a civil minute becomes an instant, and both are what
make "runs once a day" true, which is what the operator wrote down.
