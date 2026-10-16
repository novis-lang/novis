Says whether this connection can still be used.

`Core\Db\Connection::isOpen` returns `true` for a connection that is still yours, and `false` once
`Core\Db\Connection::close` has given it back. It takes no arguments and never throws.

It is the one method a closed connection still answers. Every other method of `Core\Db\Connection`
throws a `LogicError` after the connection is closed, because the connection it needs is gone. So a
program that is not sure asks this first, instead of running a statement and catching the error.

Most programs never need it. It is for the step that may run after another step already finished
with the database: a cleanup, a last log line, a handler that receives a connection it did not open.

The examples ask before and after a close, show the error every other method throws, and write a
last row only when the database is still there.
