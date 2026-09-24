- **A `[db.<name>]` field has a second home in the reference chapter, and `docs/novis.md` is
  generated from it.** `docs/reference/tools/20-config.md`'s block table lists every key each block
  accepts, and nothing checks it against `nvs_config::tree`: `deny_unknown_fields` refuses a key the
  struct lacks, no test refuses a struct field the table lacks, so a field added to the struct alone
  leaves the user-facing list wrong with nothing failing. Edit the chapter row in the same slice as
  the field, and let `nv verify` write `docs/novis.md`.
  [until: gone docs/reference/tools/20-config.md:password_file]
