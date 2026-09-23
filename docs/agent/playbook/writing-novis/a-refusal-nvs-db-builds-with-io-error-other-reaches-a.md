- **A refusal `nvs-db` builds with `io::Error::other` reaches a program as `Core\Db\DbError`,
  whatever its wording says.** That crate builds no fault of its own, so the `io::ErrorKind` is the
  only thing carrying a class across the boundary: `statement_failure`
  (`crates/nvs-stdlib/src/db/bind.rs:186`) reads `InvalidInput` as a mistake in the call and
  everything else as the engine's own refusal. Build a refusal about the *call* — a busy connection,
  a value with no bound form — with `io::ErrorKind::InvalidInput`, and prove it with a `.nvst` case
  catching the class the rule names. [until: gone crates/nvs-stdlib/src/db/bind.rs:ErrorKind::InvalidInput]
