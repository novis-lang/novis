Tells you whether a plan reports one step instead of running it.

`Core\Db\Plan\Step::isRefused` returns `true` for a step the plan will never run for you. That is
always a step that would delete something: a table, a column or a key the database has and your
schema value does not name. The step is in the plan, with the statement that would do it, and the
decision is yours.

A schema value describes what its author knows about. Another program, another release or a person
may own that table, so deleting it because one schema does not name it would lose data nobody asked
to lose. `Core\Db\Schema::applySafe` and `applyIncludingRisky` both leave these steps alone.

`true` therefore means: read this and decide. Your program can print the step, run its statement with
`Core\Db\Connection::execute`, or do nothing at all.

The examples separate the steps a plan runs from the ones it reports, delete a table on purpose, and
warn an operator about what an earlier release left behind.
