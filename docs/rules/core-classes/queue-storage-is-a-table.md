Durable background jobs live in one jobs table and one dead-letter table, in a connection the
operator names in `[queue]`. The runtime owns that schema **as a value** — a `Core\Db\Schema`
converged by `nvs queue migrate`, an explicit operator command — which is one description of the two
tables rather than one per dialect. A hand-written DDL list per backend is one chance to drift per
backend, and the backend nobody wrote is indistinguishable from one nobody supports.

DDL is an injection sink and a privileged act, so **the runtime never issues it implicitly**, not at
boot and not from a request; applying the plan takes `db.schema` like any other DDL
(`rule:core-classes/schema-apply-capability`).

It may be the application's own database, and that is the recommended configuration, because a
transactional enqueue — the property the whole design rests on — requires it. A separate queue
database is permitted and silently gives up that property, which is why the documentation says so at
the point the option is offered.
