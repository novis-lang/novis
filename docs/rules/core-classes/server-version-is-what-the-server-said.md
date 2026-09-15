`serverVersion` answers what the server said during the handshake, kept on the connection, and no
driver ever issues a statement to learn it.

| Driver | What is answered | Where it comes from |
|---|---|---|
| PostgreSQL | the `server_version` string verbatim | the `ParameterStatus` at startup |
| MySQL | the greeting's banner verbatim | the handshake packet's version field |
| MariaDB | the greeting's banner verbatim, which is how a MariaDB server says it is one | the same field |
| SQL Server | `major.minor.build`, decimal and unpadded — `16.0.4125` | `LOGINACK`'s numeric triple |
| SQLite | the library version, as `sqlite3_libversion` reports it | the linked library, not a server |

A server's own string is answered **unchanged**, suffix and all: the value exists to say what is on
the other end, and a driver that tidies it hides the thing being asked about. Where the server sends
numbers rather than a string, the table above fixes the one spelling, so a program comparing versions
across drivers compares one shape. The member never answers `null` — each of the five has an answer —
and it costs one short string per open connection.

A `SELECT version()` is refused: it spends a round trip on the request path to learn something the
connection was already told. `rule:core-classes/schema-plan`'s grader is keyed on the server version
and reads it off the connection it already holds, so the same refusal keeps a grade from costing a
statement of its own.
