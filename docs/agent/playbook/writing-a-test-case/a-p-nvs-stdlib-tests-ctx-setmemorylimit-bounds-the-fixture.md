- **A `-p nvs-stdlib` test's `Ctx::set_memory_limit` bounds the fixture too, and the failure blames
  the member.** The ceiling covers everything the context allocates from `Ctx::buffered()` onward,
  so a limit set under what the multipart parse itself costs makes `files()` throw first and the
  assertion read as the walk being broken. Keep the limit roomy in absolute terms (1 MiB) and put
  the distance into the number the member is asked for (4 MiB). [until: gone crates/nvs-runtime/src/ctx/limits.rs:fn set_memory_limit]
