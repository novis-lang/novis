- **A `NOT IMPLEMENTED` note in `crates/nvs-config/src/default.toml` stands over every key after it,
  not only the one below it.** `tools/nv/cmd/directives.ts`'s template parser clears the prose block on a
  blank line or a block header and nowhere else, and `--check-template` never notices, because it
  reads that prose only for a key the tree declares unread. Leave a blank line after the last key a
  note covers. [until: gone crates/nvs-config/src/default.toml:NOT IMPLEMENTED]
