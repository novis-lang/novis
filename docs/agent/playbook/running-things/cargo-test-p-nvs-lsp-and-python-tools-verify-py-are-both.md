- **`cargo test -p nvs-lsp` and `bun nv verify` are both green while the `.lspt` corpus is
  red.** Nothing under `tools/` runs `nvs lsp-test`, so the corpus is gated only by the loop's own
  acceptance checks and a session that changes what a request *answers* hands the driver a failure it
  did not cause. Run `target/debug/nvs.exe lsp-test tests/lsp/` and `--coverage` after touching an
  answer, and expect the breakage at a cursor mid-word, where a case froze a list the new arm adds
  to. [until: exists tools/nv/cmd/verify.ts:lsp-test]
