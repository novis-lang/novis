Runs the steps a schema needs, as long as every one of them is safe.

`Core\Db\Schema::applySafe` plans the schema against the database and then runs that plan. A step is
safe when it cannot lose data and cannot lock the table: creating a table, adding a column that may be
null, adding an index. If one step of the plan is not safe, nothing at all runs. The method throws a
`LogicError` that names the step it refused and the method that would run it.

This is the method a deployment calls. It needs the `db.schema` capability for the connection's block,
which is granted apart from `db.connect`. Reaching a database is not enough to change its shape.

A table the database has and the schema does not describe is never dropped. The plan reports it, and
`applySafe` leaves it where it is.

**The examples below** create the tables of a new database, show the refusal of a step that is not
safe, and add a column to a table that already holds rows.
