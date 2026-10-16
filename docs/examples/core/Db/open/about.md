Opens a database connection from settings the program itself writes.

Most programs name a connection instead. An operator puts the address and the password in a
`[db.<name>]` block in `nvs.toml`, and `Core\Db::connect` opens it by name. `Core\Db::open` is for
what that block cannot cover: a customer whose database is chosen while the program runs, or an
administration tool a person types a host into.

The settings are one of two shapes, and `driver` decides which. MySQL, MariaDB, PostgreSQL and SQL
Server take a host, a user and a password. SQLite takes a file path instead.

Two calls with the same settings give you the same connection. The `shared` option turns that off,
and then every call opens a new one.

**Good to know:** the operator still decides what you may reach. A program needs the `db.open`
capability for the host or the path it asks for, and for a host the address is checked against the
ranges a program may not connect to.
