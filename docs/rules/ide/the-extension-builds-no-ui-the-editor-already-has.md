Where the editor or an open format already renders something, the extension feeds it and draws nothing
of its own. Four surfaces follow that rule. Server health and version are a `LanguageStatusItem`, the
API VS Code sanctions for it, not a hand-rolled status-bar item. Coverage flows through VS Code's
finalized Testing API and its `FileCoverage` model — per-file statement, branch and declaration counts
from the Clover/lcov exporters `nvs test` already produces (`rule:testing/debug-probes`,
`rule:testing/report-formats`) — and VS Code's own gutter and summary UI shows it; no gutter renderer
is written. A profile from `nvs run --profile` is emitted in the open speedscope JSON format, and a
"View Profile" command opens it in a webview that runs speedscope's built viewer, bundled with the
extension. VS Code's built-in flame chart is V8-specific, and no bespoke flamegraph is built. The
bundled viewer keeps a profile on the machine and works in a remote window. The debugger
is DAP's existing UI over `nvs dap` (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`).

That is four pieces of UI infrastructure Novis neither builds nor maintains, which is the simplicity
priority applied directly. The extension's own code is the descriptor factory, the schema contribution,
the test provider and the command that hands a file to a viewer.

**Built:** the `LanguageStatusItem` (`editors/vscode/src/extension.ts`), coverage as `FileCoverage`
from the lcov tracefile (`editors/vscode/src/tests.ts`), and *View Profile* (`editors/vscode/src/profile.ts`).
The viewer is the `speedscope` devDependency's built release, which `scripts/viewer.mjs` copies into
`out/speedscope/` at bundle time. The webview's one resource root is that folder, and its content
security policy (`editors/vscode/src/viewer.ts`) admits the viewer's own files and one nonce script.
Nothing may connect. speedscope loads a `profileURL` only from an `http:` or `https:` page, so the
profile's bytes go into the page and reach the viewer through `loadFileFromBase64`, the door its own
CLI uses. The profile's folder is therefore never served. Spends: the file and its base64 copy per
open panel. The debugger is a `debuggers` contribution in `editors/vscode/package.json`, whose
`launch.json` schema names every field `nvs dap`'s `launch` reads, and the
`DebugAdapterDescriptorFactory` in `editors/vscode/src/debug.ts`, which starts `nvs dap` and draws
nothing. The task tree is the `nvs.tasks` tree view in the `debug` container: VS Code's own
`TreeView`, fed with `nvs dap`'s `nvsTaskTree` answer by a debug adapter tracker, the nesting
computed in `editors/vscode/src/tasktree.ts`
(`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`).

**Declined:**
- **A Novis coverage gutter and a Novis flamegraph renderer.** VS Code's Test Coverage API and the
  speedscope format already exist and are maintained elsewhere; building equivalents is scope the
  simplicity priority does not pay for.
- **Opening a profile in speedscope.app.** It needs a browser and the network outside the editor, and
  the page cannot reach a file in a remote window.
- **A hand-rolled status-bar item for server health.** `LanguageStatusItem` is the API VS Code provides
  for exactly that.
