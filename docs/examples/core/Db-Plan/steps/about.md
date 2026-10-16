Gives you every step of a plan, in the order they must run.

A plan is the difference between a schema you wrote and a database as it is now.
`Core\Db\Schema::planAgainst` computes that difference, and `Core\Db\Plan::steps` is how you read it.
Each step is a `Core\Db\Plan\Step`. It says what the change can cost, why it is there, and the
complete SQL it is.

The list is a document. Reading it asks the database nothing, and reading it twice gives the same
steps. An empty list means the database already matches the schema.

Steps that will never run are in the list too. A table the database has and the schema does not
declare is reported, together with the SQL that would drop it, and nothing drops it for you.

The examples show what a schema would change, how to tell that a database is up to date, and how a
deployment stops before a step that needs a person.
