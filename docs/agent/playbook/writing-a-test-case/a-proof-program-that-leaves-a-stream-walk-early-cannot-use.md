- **A proof program that leaves a `stream` walk early cannot use that connection again.** `break` out
  of a `foreach` over `Core\Db\Connection::stream` and the next statement on the same connection
  throws `LogicError`, because the walk holds the connection until the request ends — the module doc
  at `crates/nvs-stdlib/src/db/stream.rs` says so, and
  `rule:core-classes/a-stream-parks-its-read-on-the-connection`'s "returns to `Idle`" is the pooled
  reset. Bound the result with `limit` in the statement
  instead, and where a case really has to abandon a walk, make that its last step.
  `until: gone crates/nvs-stdlib/src/db/stream.rs:Core\Db\Connection::stream`
