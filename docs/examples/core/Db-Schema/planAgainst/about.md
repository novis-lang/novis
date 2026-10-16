Compares a schema with a database and returns the changes that would make the database match.

`Core\Db\Schema::planAgainst` reads the tables that the database has and compares them with the
schema you wrote. It returns a `Core\Db\Plan`. Its `steps()` are the changes, in the order they
must run. An empty plan means that the database already matches.

Making a plan only reads. Nothing in the database changes, so a program can make a plan as often as
it needs. Each step has the SQL for the change, a grade that says how risky it is, and a sentence
that explains the grade. `Core\Db\Schema::applySafe` runs a plan.

**Good to know:** a plan never removes a table or a column. When the database has one that the
schema does not have, the step is in the plan and its `isRefused()` is `true`. No call runs that
step.

**The examples below** show what a schema would change in an empty database, read the grade of each
step, and run the check that a program does when it starts.
