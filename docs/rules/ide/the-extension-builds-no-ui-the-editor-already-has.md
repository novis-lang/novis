Where the editor or an open format already renders something, the extension feeds it and draws nothing
of its own. Four surfaces follow that rule. Server health and version are a `LanguageStatusItem`, the
API VS Code sanctions for it, not a hand-rolled status-bar item. Coverage flows through VS Code's
finalized Testing API and its `FileCoverage` model — per-file statement, branch and declaration counts
from the Clover/lcov exporters `nvs test` already produces (`rule:testing/debug-probes`,
`rule:testing/report-formats`) — and VS Code's own gutter and summary UI shows it; no gutter renderer
is written. A profile from `nvs run --profile` is emitted in the open speedscope JSON format, and a
"View Profile" command opens it in speedscope.app or an embedded webview that speaks the same format,
because VS Code's built-in flame chart is V8-specific and no bespoke flamegraph is built. The debugger
is DAP's existing UI over `nvs dap` (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`).

That is four pieces of UI infrastructure Novis neither builds nor maintains, which is the simplicity
priority applied directly. The extension's own code is the descriptor factory, the schema contribution,
the test provider and the command that hands a file to a viewer.
