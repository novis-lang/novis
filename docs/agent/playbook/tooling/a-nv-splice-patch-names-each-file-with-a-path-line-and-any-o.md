- **A `nv splice` patch names each file with a `--- <path>` line, and any other header is refused before
  a block is read.** A patch written with `=== <path>` headers fails with "does not start with
  `<<<<<<< OLD` or `--- <path>`", and the message reads as a broken first block. Start every file's run of
  blocks with `--- <path>`. See `tools/nv/cmd/splice.ts`. `until: gone tools/nv/cmd/splice.ts:<<<<<<< OLD`
