---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Schemas and convergence"
description: "A schema is a value, a plan is its difference from a live database, and something the schema omits is reported, never dropped."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/running-a-statement/
  label: "Running a statement"
next:
  link: /docs/rules/core-classes/codecs-sessions-and-signatures/
  label: "Codecs, sessions, rate limits and signatures"
---

<p class="nv-section-lead">A schema is a value, a plan is its difference from a live database, and something the schema omits is reported, never dropped.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">1</span><span class="nv-count-label">differs from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#schema-is-a-value">A schema is a value with three spellings, and its array form is the canonical one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#schema-vocabulary-is-closed">The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-unique-key-reads-nulls-as-distinct">A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#schema-converges">A plan is the difference between a schema value and a live database, and <code>Core</code> knows no migrations</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#schema-introspection">An existing database is read by catalog query, and no DDL parser exists at any tier</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#schema-plan">Every plan step carries a grade and its complete SQL, and an unknown grade grades up</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#schema-absence-never-destroys">Something in the database and not in the schema value is reported with its SQL, and never dropped</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#schema-apply-capability">Planning is an ordinary read, and applying takes <code>db.schema</code> under a member named for its risk</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="schema-is-a-value">

## A schema is a value with three spellings, and its array form is the canonical one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#schema-is-a-value"><code>core-classes/schema-is-a-value</code></a>
</div>

`Core\Db\Schema` is a value with three interchangeable spellings and no privileged one: **built**
with typed builders, **serialized** to and from a plain array, and **introspected** off a live
connection ([`core-classes/schema-introspection`](/docs/rules/core-classes/schemas/#schema-introspection "An existing database is read by catalog query, and no DDL parser exists at any tier")).

The array form is **canonical**. Two schema values are the same schema exactly when their array forms
agree, and `toArray(fromArray(a)) == a` holds over every construct in the vocabulary. It is stated
over the array rather than over the objects because `==` on two objects is identity, there is no
equality hook to override, and a schema has no natural total order. The canonical form is therefore
**ordered**: declaration order for columns, since a `CREATE TABLE` must reproduce it, and name order
for everything else, since nothing observable depends on it — an introspector returning the server's
catalog order would fail the round trip on a database that is not wrong in any way.

The diff runs over that canonical form after normalization, and **never over SQL text**.
Normalization is where this class of tool lives or dies: a unique constraint's implicit index,
integer display widths, `varchar` promotion, column order, identifier case-folding, and the server's
own spelling of a default are each normalized out on both sides. The acceptance criterion is one
property — apply a schema, introspect it back, and the resulting plan is **empty**, on all five
backends. Every normalization rule exists because that property failed without it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-vocabulary-is-closed" title="The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch"><code>core-classes/schema-vocabulary-is-closed</code></a> <a href="/docs/rules/core-classes/schemas/#schema-introspection" title="An existing database is read by catalog query, and no DDL parser exists at any tier"><code>core-classes/schema-introspection</code></a> <a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0090.md">record 0090</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-round-trips-its-canonical-array-form.nvst"><code>tests/conformance/core/db-schema-round-trips-its-canonical-array-form.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-normalizes-every-spelling-of-one-schema.nvst"><code>tests/conformance/core/db-schema-normalizes-every-spelling-of-one-schema.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="schema-vocabulary-is-closed">

## The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#schema-vocabulary-is-closed"><code>core-classes/schema-vocabulary-is-closed</code></a>
</div>

The vocabulary is tables, columns, primary keys, unique constraints and indexes — the constructs all
five backends genuinely share. A column carries a name, a type, nullability, a default, and whether
it is the table's **identity**, which every backend has and spells differently. A unique constraint
is the canonical spelling of uniqueness, so an index in the vocabulary is never unique: admitting
both would make one schema expressible two ways. Its nulls are distinct on every backend
([`core-classes/a-unique-key-reads-nulls-as-distinct`](/docs/rules/core-classes/schemas/#a-unique-key-reads-nulls-as-distinct "A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index")), which is a property of the vocabulary and
not of whichever dialect is emitting it.

The column type enum is [`core-classes/db-column-types`](/docs/rules/core-classes/running-a-statement/#db-column-types "Every column has one natural Novis type, and the requested type converts losslessly or throws")'s map read in the **write** direction —
one canonical SQL type per Novis type per dialect, chosen so that introspecting the result maps back
to the type that was written. One table used twice, rather than a second table to keep in step.

Defaults are a closed set: a literal of a vocabulary scalar, or the current timestamp. An expression
default would be both an unbindable string reaching a DDL sink and the value a server is most likely
to spell back differently.

**There is no raw escape hatch** — no opaque fragment anywhere inside a schema value. Two things
follow and both are the point: the diff is total, so "this tool does not understand part of your
schema" is not a state that exists; and a vendor shipping a new feature costs this project nothing,
because it is reachable through `Core\Db::execute` exactly as it is today. Foreign keys, partial
indexes, index types, collations, check constraints, triggers, views and partitioning are all out of
v1 for the same reason: no portable spelling, and admitting one would break the empty-plan property.
What is out of v1 is what a schema *value* may say. An emitter still writes whatever its dialect needs
to mean a construct that is in the vocabulary — SQLite's `CREATE UNIQUE INDEX` where the others write
a constraint, SQL Server's filtered index for a nullable unique key — and the introspector reads that
spelling back as the construct it stands for.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-is-a-value" title="A schema is a value with three spellings, and its array form is the canonical one"><code>core-classes/schema-is-a-value</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-column-types" title="Every column has one natural Novis type, and the requested type converts losslessly or throws"><code>core-classes/db-column-types</code></a> <a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a> <a href="/docs/rules/core-classes/schemas/#a-unique-key-reads-nulls-as-distinct" title="A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index"><code>core-classes/a-unique-key-reads-nulls-as-distinct</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-refuses-what-the-closed-vocabulary-cannot-say.nvst"><code>tests/conformance/core/db-schema-refuses-what-the-closed-vocabulary-cannot-say.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-unique-key-reads-nulls-as-distinct">

## A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-unique-key-reads-nulls-as-distinct"><code>core-classes/a-unique-key-reads-nulls-as-distinct</code></a>
</div>

A unique key in the schema vocabulary means the standard's unique key on all five backends: rows whose
key columns are all non-null are distinct, and a row with a null in any key column collides with
nothing.

Four backends give that directly. **SQL Server's emitter writes a filtered unique index** for a key
over any nullable column —

```sql
CREATE UNIQUE INDEX <key> ON <table> (<columns>) WHERE <column> IS NOT NULL;
```

— with one `IS NOT NULL` conjunct per nullable key column, and the plain `ADD CONSTRAINT … UNIQUE`
where every key column is `not null`.

**That is a spelling, not a vocabulary growth.** A `Core\Db\Schema` declares a unique key over columns
and nothing else; the `WHERE` is the emitter's, the way SQLite's emitter already writes
`CREATE UNIQUE INDEX` where the others write a constraint, and
[`core-classes/schema-vocabulary-is-closed`](/docs/rules/core-classes/schemas/#schema-vocabulary-is-closed "The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch") still keeps a partial index out of what a program may
express. The catalog reader matches the index back to the key that asked for it, predicate and all, so
a second `plan` over a converged database is empty. **Any other predicate leaves the index out of the
read value entirely**, as a partial index always has: one arriving with its `WHERE` discarded would
put a key in the value the server does not hold, and a diff that agrees with a database it does not
match is the failure the whole schema half rests on not having. The cost falls the other way — a plan
proposing a key whose name a partial index already owns fails on the server rather than in the plan,
which `crates/nvs-db/src/catalog.rs`'s module doc owns. A build over an existing table is `Locking`
and inside a `CREATE TABLE` it is `Safe` ([`core-classes/schema-plan`](/docs/rules/core-classes/schemas/#schema-plan "Every plan step carries a grade and its complete SQL, and an unknown grade grades up")).

The alternative is refused for a reason that outlives SQL Server: a `not null` column carrying a
generated token per row cannot be added to a table that already holds rows, so it describes a schema
no existing deployment can converge to, while a nullable column arrives as a `Safe` step. Uniform null
semantics are what [`core-classes/db-one-api`](/docs/rules/core-classes/connecting-to-a-database/#db-one-api "Core\Db is the only database API, and every statement it runs is prepared")'s one-API promise means for a program's own schema,
and [`core-classes/queue-storage-is-a-table`](/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table "The job queue is two tables in a connection the operator names, converged by an explicit command")'s `dedupe_pending` is one reader of them rather than
the reason for them.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-vocabulary-is-closed" title="The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch"><code>core-classes/schema-vocabulary-is-closed</code></a> <a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/ddl.rs"><code>crates/nvs-db/src/ddl.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/catalog.rs"><code>crates/nvs-db/src/catalog.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/queue.rs"><code>crates/nvs-stdlib/tests/queue.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="schema-converges">

## A plan is the difference between a schema value and a live database, and `Core` knows no migrations

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#schema-converges"><code>core-classes/schema-converges</code></a>
</div>

A schema value says what the tables **should be**. A plan is the difference between that and what a
connection **has**. That is the entire model, and everything a versioned migrator carries is absent
by construction: no version number, no history table in the target database, no ordering between
changes, no `up`/`down` pair, no fleet lock, and no notion of which migrations have run.

Convergence is what makes that possible. A plan is derived from the current state every time it is
computed, so it is correct after a manual change, after a partially applied earlier plan, and against
a database this tool has never seen — the three situations in which a history table is exactly wrong,
because it records what a tool believes rather than what is true.

**`Core` therefore holds no notion of a migration.** Ordering, reversibility, dependency between
changes and fleet locking stay a blocked gap in the first-party framework
([`programs/no-migration-runner`](/docs/rules/programs/the-framework/#no-migration-runner "No migration runner ships until migration semantics are decided")), and this is the layer such a runner would one day sit on. It
stops there deliberately: every one of those open questions is a question convergence does not have
to answer.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no version number, no history table, no <code>up</code>/<code>down</code> pair and no fleet lock — the plan is recomputed from the database every time</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a> <a href="/docs/rules/core-classes/schemas/#schema-introspection" title="An existing database is read by catalog query, and no DDL parser exists at any tier"><code>core-classes/schema-introspection</code></a> <a href="/docs/rules/core-classes/schemas/#schema-absence-never-destroys" title="Something in the database and not in the schema value is reported with its SQL, and never dropped"><code>core-classes/schema-absence-never-destroys</code></a> <a href="/docs/rules/programs/the-framework/#no-migration-runner" title="No migration runner ships until migration semantics are decided"><code>programs/no-migration-runner</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0082.md">record 0082</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst"><code>tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="schema-introspection">

## An existing database is read by catalog query, and no DDL parser exists at any tier

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#schema-introspection"><code>core-classes/schema-introspection</code></a>
</div>

Reading an existing database is one catalog reader per driver, answering the same schema value a
builder produces: `information_schema` for MySQL, MariaDB and SQL Server, `pg_catalog` for
PostgreSQL, and `sqlite_master` with the table and index pragmas for SQLite.

There is **no DDL parser, at any tier, in any form** — not "just for the CLI", not "best-effort", not
"just for `CREATE TABLE`". [`core-classes/db-literal-query-checking`](/docs/rules/core-classes/running-a-statement/#db-literal-query-checking "A literal query is checked while compiling, and no vendor's SQL grammar is ever parsed") already refuses to maintain
four vendors' query grammars, and that reasoning does not weaken for DDL, where the dialects diverge
more rather than less.

Introspection is also the *better* answer, not merely the cheaper one: a catalog is structured tables
rather than a language, the `CREATE TABLE` a server prints came from that server's own catalog
anyway, and the introspector must exist for the diff regardless — so a parser would be a second,
worse implementation of a job already done. Offline `.sql` input, if it is ever wanted, is a separate
tool with its own decision.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-is-a-value" title="A schema is a value with three spellings, and its array form is the canonical one"><code>core-classes/schema-is-a-value</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-literal-query-checking" title="A literal query is checked while compiling, and no vendor's SQL grammar is ever parsed"><code>core-classes/db-literal-query-checking</code></a> <a href="/docs/rules/core-classes/schemas/#schema-converges" title="A plan is the difference between a schema value and a live database, and Core knows no migrations"><code>core-classes/schema-converges</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst"><code>tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-normalizes-every-spelling-of-one-schema.nvst"><code>tests/conformance/core/db-schema-normalizes-every-spelling-of-one-schema.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="schema-plan">

## Every plan step carries a grade and its complete SQL, and an unknown grade grades up

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#schema-plan"><code>core-classes/schema-plan</code></a>
</div>

Every step of a plan carries a grade, and there are three rather than two. **`Safe`** cannot lose
data, fail on existing rows, or hold a long lock. **`Locking`** cannot lose data but can fail or
block writes for a long time — a unique index over data that already contains duplicates, `not null`
on a populated column, a type change that rewrites the table. **`Destructive`** can lose data. Two
grades would have to merge the middle into an outer one, and both merges are wrong in production:
calling a table rewrite `Safe` is how an automatic tool takes a site down without losing a byte, and
calling it `Destructive` makes the dangerous class so large that operators stop reading it.

A grade is computed by the dialect emitter, keyed on driver **and** server version, because adding a
column with a default is instant on a recent server and a full rewrite on an older one. **When the
emitter does not know the version, it grades up.** An out-of-date grader over-reports risk, which
costs a confirmation; an out-of-date optimist costs an outage. A grade is a floor, never a promise.

Each step also exposes **complete, terminated, dialect-correct SQL**, including the steps the tool
will refuse to run. The common case in a serious deployment is that the application's own credentials
cannot issue DDL at all and a DBA applies the change from a ticket — a plan whose risky steps are
elided into "3 unsafe changes" is useless to that person, and a plan they can paste is the whole
product. Emitters follow the four dialects, not the five drivers.

A dialect's spelling of a vocabulary construct is graded as that construct, not as the SQL it happens
to be: SQL Server's filtered unique index for a nullable unique key
([`core-classes/a-unique-key-reads-nulls-as-distinct`](/docs/rules/core-classes/schemas/#a-unique-key-reads-nulls-as-distinct "A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index")) is `Locking` built over an existing table
and `Safe` inside a `CREATE TABLE`, exactly as the constraint form is on the other dialects.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-apply-capability" title="Planning is an ordinary read, and applying takes db.schema under a member named for its risk"><code>core-classes/schema-apply-capability</code></a> <a href="/docs/rules/core-classes/schemas/#schema-absence-never-destroys" title="Something in the database and not in the schema value is reported with its SQL, and never dropped"><code>core-classes/schema-absence-never-destroys</code></a> <a href="/docs/rules/core-classes/schemas/#schema-converges" title="A plan is the difference between a schema value and a live database, and Core knows no migrations"><code>core-classes/schema-converges</code></a> <a href="/docs/rules/core-classes/schemas/#a-unique-key-reads-nulls-as-distinct" title="A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index"><code>core-classes/a-unique-key-reads-nulls-as-distinct</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-apply-safe-refuses-what-apply-including-risky-runs.nvst"><code>tests/conformance/core/db-schema-apply-safe-refuses-what-apply-including-risky-runs.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst"><code>tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="schema-absence-never-destroys">

## Something in the database and not in the schema value is reported with its SQL, and never dropped

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#schema-absence-never-destroys"><code>core-classes/schema-absence-never-destroys</code></a>
</div>

A table, column, constraint or index present in the database and **not** present in the schema value
is reported by the plan, with the complete SQL that would drop it, and is **never dropped**.

This is not conservatism about a risky default; it is the only reading that survives a shared
database. A schema value describes what its author knows about, and an application legitimately
shares a server with another application, a reporting view, an operator's own table and the job
queue's two tables ([`core-classes/queue-storage-is-a-table`](/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table "The job queue is two tables in a connection the operator names, converged by an explicit command")). A converger treating its own value
as the complete truth of the database would drop every one of those on the first run.

Dropping is therefore something someone **writes**, and it is `Destructive`. There is no `--prune`,
no `allowDrops` and no strict mode: the escape hatch is the SQL the plan already printed, in the same
`Core\Db::execute` that is where a statement a human wrote goes.

A report is a step the plan carries and never applies, which is what keeps `applySafe` usable at all:
every plan against a shared database holds reports, and a rule reading *every* step's grade would
refuse every plan ever computed against a real database. `applySafe` reads the grades of the steps it
would run.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a> <a href="/docs/rules/core-classes/schemas/#schema-converges" title="A plan is the difference between a schema value and a live database, and Core knows no migrations"><code>core-classes/schema-converges</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-reports-a-table-it-does-not-declare-and-never-drops-it.nvst"><code>tests/conformance/core/db-schema-reports-a-table-it-does-not-declare-and-never-drops-it.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="schema-apply-capability">

## Planning is an ordinary read, and applying takes `db.schema` under a member named for its risk

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#schema-apply-capability"><code>core-classes/schema-apply-capability</code></a>
</div>

Computing a plan is an ordinary read. It issues catalog queries through the connection a program
already holds, under the `db.connect` it already has, and nothing about it is privileged.

Applying is not. It takes the deny-by-default `db.schema` capability
([`core-classes/db-capabilities`](/docs/rules/core-classes/connecting-to-a-database/#db-capabilities "Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it")), which gates a different thing from `db.connect` and `db.open`:
not which database may be reached, but whether this program may issue DDL to it at all. It names
connection blocks, and an ungranted name throws naming the capability.

The two entry points are named for what they risk, at the call site. **`applySafe()`** refuses if any
step it would run is not `Safe`, and throws naming the first that is not.
**`applyIncludingRisky()`** says so where it is written, so a reviewer reading the call sees the
claim being made. `nvs schema plan|apply|dump` is the operator's spelling of the same thing.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-capabilities" title="Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it"><code>core-classes/db-capabilities</code></a> <a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-schema-apply-safe-refuses-what-apply-including-risky-runs.nvst"><code>tests/conformance/core/db-schema-apply-safe-refuses-what-apply-including-risky-runs.nvst</code></a></dd></div></dl>

</div>
