Says which version the database is running.

`Core\Db\Connection::serverVersion` returns the version as text, in the database's own wording.
PostgreSQL gives its `server_version` setting. MySQL and MariaDB give the banner they send when the
connection opens. SQL Server gives three numbers with dots between them. SQLite has no server, so it
gives the version of the database library this program is built with.

The database reports its version while the connection is opening, and the connection keeps that
text. Reading it therefore costs no statement, and the result is the same every time. Even a walk
over rows can read it, and a walk holds the connection so that no statement can run.

The text is never empty, and Novis never changes it. A version such as `14.10 (Debian 14.10-1)`
keeps the part in brackets, because that is what the other side sent. A closed connection throws a
`LogicError`.

The examples read the version, take the main number out of it to see what the database can do, and
store it beside a change to the schema.
