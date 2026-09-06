Durable background jobs live in one jobs table and one dead-letter table, in a connection the
operator names in `[queue]`. The runtime owns that schema **as a value** — a `Core\Db\Schema`
converged by `nvs queue migrate`, an explicit operator command — which is one description of the two
tables rather than one per dialect. A hand-written DDL list per backend is one chance to drift per
backend, and the backend nobody wrote is indistinguishable from one nobody supports.

**At most one *pending* job per dedupe key, and what enforces it is a plain column under a plain
unique key.** The jobs table carries `dedupe_pending`, which the statements maintain: it holds the
job's `dedupe_key` while the job is pending and `null` once it is claimed, succeeded, cancelled or
dead-lettered. A partial index (`… where state = 0`) and a stored generated column each say the same
thing, and neither is in `rule:core-classes/schema-is-a-value`'s vocabulary — they are two dialects'
answers to one requirement, which is exactly what a schema value exists to stop being. A null
collides with nothing on the four backends whose unique keys read nulls as distinct, so the
guarantee is the same one on every backend `Core\Queue` runs a statement against.

SQL Server reads two nulls as **equal**, and so admits one released row rather than any number of
them. It has no queue statements at all, so nothing runs against that shape today; the day it gains
them, the spelling it needs is the filtered index `rule:core-classes/schema-plan` keeps out of v1,
and the vocabulary grows before the queue does. The alternative — a `not null` column with a
generated token per released row — is refused for a reason that outlives SQL Server: such a column
cannot be added to a table that already holds rows, so it would be a schema no existing deployment
could converge to, while a nullable one arrives as a `Safe` step.

DDL is an injection sink and a privileged act, so **the runtime never issues it implicitly**, not at
boot and not from a request; applying the plan takes `db.schema` like any other DDL
(`rule:core-classes/schema-apply-capability`).

It may be the application's own database, and that is the recommended configuration, because a
transactional enqueue — the property the whole design rests on — requires it. A separate queue
database is permitted and silently gives up that property, which is why the documentation says so at
the point the option is offered.
