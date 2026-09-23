- **A `-p nvs-db` test cannot build a `PgConn`, because its `wire` field is `Wire` at the default
  type parameter.** Anything reachable only through an inherent method on `PgConn` needs a socket
  and a certificate, and a unit test has neither. Write the sequencing as a free function generic in
  the stream (`start_statement(wire, state, ...)`, `reset_session(wire, state)`) with `PgConn`'s
  method a two-line delegation; `pg.rs`'s `Peer` then scripts a server for it with no socket, as
  `authenticate` already does. [until: gone crates/nvs-db/src/conn.rs:pub(crate) wire: Wire,]
