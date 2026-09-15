The vocabulary is tables, columns, primary keys, unique constraints and indexes — the constructs all
five backends genuinely share. A column carries a name, a type, nullability, a default, and whether
it is the table's **identity**, which every backend has and spells differently. A unique constraint
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
