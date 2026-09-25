- **`mysql_common`'s `Column` serializes `column_length` and `character_set` in the opposite order
  to its own deserializer.** The reader matches the wire; only a case asking about the charset or
  width notices — `rule:core-classes/db-column-types` reads `tainted string` against `tainted bytes`
  off the charset. Write the definition bytes by hand in wire order (`typed_column_def` in
  `crates/nvs-db/src/mysql.rs`), never through `MySerialize`. [until: gone crates/nvs-db/src/mysql.rs:typed_column_def]
