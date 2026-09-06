A statement is a `query` trace event, beside `call`, `gc` and `spawn`, instrumented in the driver's
own statement routine and off the hot path exactly as those two are. The event carries **duration,
driver, connection name, truncated SQL text, rows returned and rows affected — and never a bound
parameter.** The span type is handed the SQL and the clock and is never handed the values, so there
is no parameter in scope for a later field, a later rendering or a later driver to leak; the SQL is
safe to carry because no driver ever interpolates a value into it, so what the span holds is the
statement as written, placeholders still placeholders. `rule:core-classes/db-error` states the same
rule for the error path.

The driver is a field so the five backends contribute one event and a trace reads across them. The
duration is measured from the moment the statement went out, not from its first row, so a span
reports what the caller waited. The same span feeds
`rule:observability/a-slow-query-is-logged-past-a-threshold`, rendered at most once however many
readers there are, and it is what becomes a query span in
`rule:observability/four-kinds-become-a-span`. Both outputs are inert unless asked for.
