- **A `.lspt` case's second `--FILE--` cannot exist as a buffer alone.** `nvs_hir::requires`
  canonicalizes a `require` target against the filesystem before any source map is consulted, so a
  file that exists only as an overlay is reported `E0311` "cannot be loaded", which reads as a case
  that named the wrong path rather than as a runner that never wrote it. `nvs_lsp::suite`'s
  `Materialised` writes every section into a scratch directory and opens the buffers over those real
  paths — anything else that drives the front end from text alone owes the same.
  [until: gone crates/nvs-lsp/src/suite.rs:Materialised]
