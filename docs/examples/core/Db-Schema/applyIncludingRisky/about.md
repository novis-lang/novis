Runs every step of the plan, including the ones that can lose data or hold a lock.

`Core\Db\Schema::applyIncludingRisky` plans the schema against the database and runs the whole plan.
`Core\Db\Schema::applySafe` is the same method with one check in front of it, and that check refuses a
plan holding a step which is not safe. This method runs that step.

The name is the point. Somebody reading the call sees what is being accepted, and a search across a
project finds every place where it was accepted. There is no option to pass: the two methods are the
choice itself.

It needs the `db.schema` capability for the connection's block, the same grant `applySafe` needs.

A table the database has and the schema does not describe is still never dropped. Accepting a risk is
not the same as accepting data loss.

**The examples below** run a change `applySafe` refuses, read the plan before deciding to run it, and
show that a table you never declared is kept.
