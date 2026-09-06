A table, column, constraint or index present in the database and **not** present in the schema value
is reported by the plan, with the complete SQL that would drop it, and is **never dropped**.

This is not conservatism about a risky default; it is the only reading that survives a shared
database. A schema value describes what its author knows about, and an application legitimately
shares a server with another application, a reporting view, an operator's own table and the job
queue's two tables (`rule:core-classes/queue-storage-is-a-table`). A converger treating its own value
as the complete truth of the database would drop every one of those on the first run.

Dropping is therefore something someone **writes**, and it is `Destructive`. There is no `--prune`,
no `allowDrops` and no strict mode: the escape hatch is the SQL the plan already printed, in the same
`Core\Db::execute` that is where a statement a human wrote goes.

A report is a step the plan carries and never applies, which is what keeps `applySafe` usable at all:
every plan against a shared database holds reports, and a rule reading *every* step's grade would
refuse every plan ever computed against a real database. `applySafe` reads the grades of the steps it
would run.
