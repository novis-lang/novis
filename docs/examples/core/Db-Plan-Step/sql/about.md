Gives you the complete SQL one step of a schema plan is.

`Core\Db\Plan\Step::sql` returns the statement the step would run, written for the database this plan
was computed against. It is never shortened and never summarised. A program can print it for a
person to review, save it to a file, or run it with `Core\Db::execute` as it stands.

A few steps are more than one statement. SQLite has to rebuild a table to change a column, which is
four statements, and `sql()` returns them joined by a newline. `Core\Db::execute` runs one statement
per call, so a step of that kind is one for a person rather than for a program.

Steps the plan will never run carry their SQL too. A table the database has and the schema does not
name is reported with the statement that would drop it, and nothing drops it for you. Running that
statement is a decision a program makes for itself.

The examples read the statement a step is, run one step by hand, and write a whole plan out for a
review.
