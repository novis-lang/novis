Gives a database connection back before the program has finished.

A server keeps a small number of connections for each database. Your program borrows one when it
opens the database, and gives it back when the request ends. `close` gives it back earlier, so
another request can use it while your program does the rest of its work.

After `close`, the connection is no longer usable. Every method that needs it throws a
`LogicError`. `isOpen` still answers, and it answers `false`. A second `close` does nothing and
is not an error, so a cleanup step may run twice.

The name is free again after a close. `Core\Db::connect` with the same name opens the database
and gives you a new connection.
