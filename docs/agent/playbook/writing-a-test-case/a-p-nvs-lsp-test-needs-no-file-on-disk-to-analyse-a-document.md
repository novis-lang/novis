- **A `-p nvs-lsp` test needs no file on disk to analyse a document.** `Documents::open` registers
  the buffer as an overlay and `SourceMap::load` hands that back before it reaches the filesystem,
  so `analyse` answers for a URI naming a path that does not exist. `crates/nvs-lsp/tests/publish.rs`'s `TempDir`
  is there for `require` resolution and republish-by-path, not for the analysis, so copy it only
  when a case has a second file. [until: reviewed 2026-09-08]
