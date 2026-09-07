# Handoff

## State

**Goal 14 stage 3 is landed and whole.** A `.lspt` case is read
(`crates/nvs-lsp/src/case.rs`), every answer it can freeze is rendered by one module
(`crates/nvs-lsp/src/render.rs`, `rule:ide/the-rendering-has-one-home`), and `nvs lsp-test <paths>`
walks a tree and prints `N passed, M failed` (`crates/nvs-lsp/src/suite.rs`). `tests/lsp/` exists
with its README and no cases.

**No request is answered.** `crates/nvs-lsp/src/suite.rs:157`'s `answer` is the seam every request
slice lands one arm in; until then a case fails naming the request it asked, which is why
`nvs lsp-test tests/lsp/` prints `0 passed, 0 failed` rather than a number that means nothing.
`crates/nvs-lsp/src/server.rs:60` still answers everything but `shutdown` with `MethodNotFound`.

The case reader is the driver's `wip(loop)` commit from the cut-off session; it was unverified when
written and this session's `verify.py` run covers it unchanged.

Three spellings a case will meet are decided and live in `crates/nvs-lsp/src/render.rs`'s module
doc: an answer with nothing in it renders `none`, an absent optional field renders `-`, and
completion's columns are fixed at 8 and 10 rather than computed from the rows, so one added
completion does not rewrite every frozen expectation.

## Next group

**Stage 4: the document store and the positions under it** — one file set: a new
`crates/nvs-lsp/src/document.rs`, `crates/nvs-lsp/src/server.rs`, and
`crates/nvs-diagnostics/src/source.rs` read only.

- [ ] **The open-document store.** `didOpen`/`didChange`/`didClose` hold a buffer per URI with a
      version, overlaid on disk for everything the graph reads, and diagnostics are published only
      for open documents. `rule:ide/an-open-document-is-its-own-entry-point`; the dispatch it hangs
      off is `crates/nvs-lsp/src/server.rs:60`. Tests `an_unsaved_buffer_shadows_the_file_on_disk`
      and `editing_a_required_file_republishes_the_requiring_document`.
- [ ] **One home for a position.** Every offset conversion goes through `nvs-diagnostics` and none
      is written in `nvs-lsp`: `crates/nvs-diagnostics/src/source.rs:173`'s `offset_of` takes the
      negotiated encoding already, `:122` is `line_col` and `:137` `utf16_col`. A document's bytes
      are never normalised, so a BOM and CRLF answer what LF does.
      `rule:ide/positions-have-one-home`. Test
      `a_bom_and_a_crlf_document_answer_the_same_offsets_as_an_lf_one`.
- [ ] **A superseded analysis is dropped.** A second edit arriving while the first is being analysed
      makes the first answer unwanted, and the version it was started for is what says so —
      `crates/nvs-lsp/src/server.rs:60` is single-threaded today, so this is the slice that decides
      whether an analysis thread joins it. `rule:ide/the-server-is-synchronous` bounds the answer.
      Test `an_analysis_for_a_superseded_version_is_cancelled`.

## Backlog

- `docs/reference/tools/10-cli.md:34` still says `nvs lsp`, `nvs serve` and `nvs service` are
  unrecognized subcommands; all three exist. That chapter owns the fix.
- Stage 4's `lsp cases` check wants 160 passing cases (`docs/agent/loop-goal.toml:5133`); the corpus
  arrives with the handlers, one slice's cases at a time.
- `nvs lsp-test --coverage` and `every_request_answers_every_construct` are unwritten — they need
  the cursor's resolved node (`rule:ide/lspt-coverage-is-inferred`).
- The `.lspt`/`.nvst` grammar for case files is goal 15's
  (`rule:ide/case-files-have-their-own-grammar`).
