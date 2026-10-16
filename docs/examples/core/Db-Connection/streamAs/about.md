Runs one statement that reads rows, and gives them to you one at a time as objects of the class you
name.

`Core\Db\Connection::streamAs` is `Core\Db\Connection::stream` with the rows built into a class. You
write the class in angle brackets, as in `streamAs<Person>`, and each step of a `foreach` reads one
row from the database and builds one `Person` from it. Only the current object is in memory, so a
table of ten million rows costs about as much as a table of ten.

The class says how a row is read, exactly as it does for `Core\Db\Connection::queryAs`. A class
marked `#[Db\Derive]` takes one property per column of the same name. A class that declares a
`fromRow` method builds the object itself.

The connection is held for the whole walk. A second statement on the same connection while rows
remain is a `LogicError`, and a loop you leave early with `break` keeps the connection busy until
the request ends. Write the bound into the statement with `limit` instead.

The examples walk a table as objects, hand a typed walk to the rest of a program, and report on a
table that is larger than memory.
