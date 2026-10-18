- **A `[db.<name>]` field has a second home in the reference chapter, and `/llms-full.txt` is
  generated from it.** `docs/reference/tools/20-config.md`'s block table lists every key each block
  accepts, and nothing checks it against `nvs_config::tree`: `deny_unknown_fields` refuses a key the
  struct lacks, no test refuses a struct field the table lacks, so a field added to the struct alone
  leaves the user-facing list wrong with nothing failing. Edit the chapter row in the same slice as
  the field, and run `bun nv render --website` to rewrite `website/public/llms-full.txt`.
  `until: gone docs/reference/tools/20-config.md:password_file`
