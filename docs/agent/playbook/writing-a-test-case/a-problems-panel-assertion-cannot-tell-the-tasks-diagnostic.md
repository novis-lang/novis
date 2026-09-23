- **A Problems-panel assertion cannot tell the Task's diagnostic from the server's.** `nvs lsp`
  publishes with `source: "nvs"` (`crates/nvs-lsp/src/diagnostics.rs:216`) and the `$nvs` problemMatcher
  sets the same source and the same code, so `languages.getDiagnostics` answers entries the API gives no
  way to separate. Withhold the server for that assertion — `nvs.lsp.enable` false, wait for its entries
  to clear, run the task, restore it in a `finally` — as
  `editors/vscode/test/host/surfaces.test.ts:112` does.
  [until: gone crates/nvs-lsp/src/diagnostics.rs:code_description: None]
