# Handoff

## State

**Goal 14 stage 5 is closed.** `crates/nvs-lsp/src/server.rs`'s `apply` says what a document-sync
notification changed and `publish` sends `textDocument/publishDiagnostics` for every URI
`Documents::to_republish` names — the edited document and every open document whose last analysis read
it, each analysed as its own entry point, each asked `is_current` again immediately before the send.
An empty list is sent rather than skipped, because that is the only thing that clears a squiggle the
edit fixed, and a closed document is retracted with one last empty publish at no version.

**`crates/nvs-lsp/src/diagnostics.rs`'s `for_document` is the one call the server and the suite both
make**: the gate, narrowed to the entry file, crossed to the wire. The narrowing is the half that is
easy to miss — one analysis reports over a whole `require` graph, and a notification is about one URI.
`Phases::All` is the same walk with the filter off, which is `diagnostics phase=all`.

**`nvs lsp-test tests/lsp/` reports `5 passed, 0 failed`.** `crates/nvs-lsp/src/suite.rs`'s `answer`
has its first arm; a case is materialised into a scratch directory and opened as buffers over it
(the playbook bullet says why). Columns in a `--EXPECT--` line are UTF-8, which that module's doc owns.

**Nothing answers a request yet.** `crates/nvs-lsp/src/server.rs:111` still refuses every one with
`MethodNotFound`, and eight of the nine `Response` variants have no producer.

## Next group

**Stage 4: the requests, opening with `documentSymbol`** — one file set:
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/src/document.rs`, with
`crates/nvs-lsp/src/render.rs` read only. `documentSymbol` first because it takes no cursor and no
type: the outline is a walk of `Analysed::loaded`'s statements and nothing else.

- [ ] **The outline, from the statements the analysis already parsed.** A walk over
      `crates/nvs-lsp/src/document.rs:266`'s `Analysed::loaded` entry file, building the
      `lsp_types::DocumentSymbol` tree `crates/nvs-lsp/src/render.rs:370`'s `outline` renders —
      classes, their members, functions and constants, each with the range its span names.
      `rule:ide/the-request-set-is-closed`.
- [ ] **The request seam, and `documentSymbol` as its first arm.**
      `crates/nvs-lsp/src/server.rs:111`'s blanket refusal becomes a dispatch that answers the methods
      it has and refuses the rest by the same `MethodNotFound` — a request for a document that is not
      open is an answer of nothing, never an error. `rule:ide/the-request-set-is-closed`.
- [ ] **Its `.lspt` cases, under `tests/lsp/symbols/`.** `crates/nvs-lsp/src/suite.rs:178`'s `answer`
      takes the second arm; the cases include a document with an unclosed brace, because an outline of
      code that parses is not what the resilient tree exists for.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.

## Backlog

- `nvs lsp-test --coverage` and `every_request_answers_every_construct` — `rule:ide/lspt-coverage-is-inferred`.
- The `lsp cases` check wants 160 passing cases; five are on disk, and each request slice ships its own.
- The analysis thread `rule:ide/the-server-is-synchronous` names is still unspawned: publishing runs on
  the message loop, and the second `is_current` is what will make that swap safe.
- A `require` to an unsaved new file cannot resolve — the playbook bullet's `[until:]`.
- `render.rs`'s `Place`/`Link` need the case-relative name of a materialised path; nothing maps one back
  yet, and `definition` is the slice that will need it.
