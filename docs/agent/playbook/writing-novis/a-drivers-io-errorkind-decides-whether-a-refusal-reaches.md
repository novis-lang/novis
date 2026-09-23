- **A driver's `io::ErrorKind` decides whether a refusal reaches Novis as a `Db\DbError` at all.**
  `nvs_stdlib::db`'s `statement_failure` builds the `rule:core-classes/db-error` error from its
  `ErrorKind::Other` arm alone and answers an `IOError` for every other kind, so a driver wording a
  server `ERR` packet as `PermissionDenied` strips `kind`, `sqlState` and `driverCode` with every
  driver unit test green. A server-worded refusal is `io::Error::other(ServerError { … })`;
  `ServerError::of` tells it from a wire failure, never the `ErrorKind`.
  [until: gone crates/nvs-stdlib/src/db/bind.rs:std::io::ErrorKind::Other =>]
