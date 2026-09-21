Compares a schema with a database and returns the steps that would close the difference.

`Core\Db\Schema::planAgainst` reads the tables a database really has and compares them with the schema
you describe. It returns a `Core\Db\Plan`, whose `steps()` are the changes needed to make the database
match, in the order they have to run. An empty plan means the database already matches.

A plan is a document. Nothing in the database changes while one is computed, so a program can plan as
often as it likes. Every step carries the SQL that makes it, a grade saying what it risks, and a
sentence explaining the grade. `Core\Db\Schema::applySafe` is what runs a plan.

Planning needs only the `db.connect` the connection was opened with. It is an ordinary read, so a
program that may talk to a database may also ask what a schema would change there.

A table or a column the database has and the schema does not is reported, not removed. The step is in
the plan, its `isRefused()` is `true`, and no apply will ever run it.

**The examples below** show what a schema would change in an empty database, read the grade of each
step, and run the check a program does at start-up.
