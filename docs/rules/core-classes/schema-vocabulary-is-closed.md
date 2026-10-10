The vocabulary is tables, columns, primary keys, unique constraints and indexes — the constructs all
five backends genuinely share. A column carries a name, a type, nullability, a default, and whether
it is the table's **identity**, which every backend has and spells differently. Identity is in because
it meets the admission test exactly: every backend has it, it is introspectable, and no schema anybody
writes fits a vocabulary in which a table cannot have an `id`. It carries the three rules the five
spellings share: an integer type, at most one per table, and a member of the primary key. A unique constraint
is the canonical spelling of uniqueness, so an index in the vocabulary is never unique: admitting
both would make one schema expressible two ways. Its nulls are distinct on every backend
(`rule:core-classes/a-unique-key-reads-nulls-as-distinct`), which is a property of the vocabulary and
not of whichever dialect is emitting it.

The column type enum is `rule:core-classes/db-column-types`'s map read in the **write** direction —
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

The vocabulary grows only when a construct exists on **all five backends** *and* something concrete
needs it, never because a release note appeared. Foreign keys are the exclusion most likely to be lifted,
and they are harder than they look: the backends agree on the syntax and disagree on the behaviour.
SQLite enforces them only under a pragma, adding one is `Locking` because the server validates existing
rows, and several backends create an implicit index for one, which is the normalization trap the
empty-plan property has to survive. They come in once that property holds on all five without them.

Identifiers are **validated, never delimited**: a table or column name that is not a plain identifier is
refused where it enters a schema value, by the same judgement `Core\Db::quoteIdentifier` makes, and is
never escaped into safety by a second answer.

**Declined:**
- **A raw SQL fragment inside a schema value.** It is the one change that would let the vocabulary cover
  everything, and it ends the diff's totality: a plan over an opaque fragment is best-effort, and every
  guarantee above becomes a hedge.
