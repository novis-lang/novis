- **Adding a request that takes a cursor obliges a case at every construct the corpus has ever
  reached, not one case.** `every_request_answers_every_construct` holds each row whose
  `Request::takes_cursor()` is true to the whole vocabulary — 26 columns today — so `references`,
  `documentHighlight` and `typeHierarchy` are 26 cases each, while `codeLens`, asked of the whole
  document, owed one. Price the row with `nvs lsp-test tests/lsp/ --coverage` before landing the
  variant, and take the cursor placements from `tests/lsp/actions/`, which is already one case per
  construct. [until: gone crates/nvs-lsp/src/coverage.rs:which is the ratchet]
