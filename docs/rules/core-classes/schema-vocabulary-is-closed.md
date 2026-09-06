The vocabulary is tables, columns, primary keys, unique constraints and indexes — the constructs all
five backends genuinely share. A column carries a name, a type, nullability, a default, and whether
it is the table's **identity**, which every backend has and spells differently. A unique constraint
is the canonical spelling of uniqueness, so an index in the vocabulary is never unique: admitting
both would make one schema expressible two ways.

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
