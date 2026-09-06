A `[db.<name>]` block may write `slow_query`, a duration such as `"200ms"` or `"1s"`; a statement on
that connection that runs longer writes the same facts its trace event carries — duration, driver,
connection, truncated SQL, row counts, never a parameter — to `Core\Log`
(`rule:observability/a-query-is-a-trace-event`).

**Unwritten is off**, and off is silent: a deployment gets no slow-query log it did not ask for. A
written `0` is a threshold every statement passes, not a second spelling of off. A connection with
no block at all — one opened by the program with settings no operator named
(`rule:core-classes/db-connection-is-named`) — has no threshold either. A value that is not a
duration is refused at boot, where it is written.

The threshold and the trace are asked once, *before* a statement borrows the context, so a request
that turns either on midway through a statement gets a whole event or none, never half of one.
