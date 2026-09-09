# Handoff

## State

**Goal `editor-surfaces` — the extension answers everything it contributes — has just started; nothing of it has landed
yet.** Goal `workspace-index`'s whole acceptance list is this goal's floor, and the server it leaves behind is what
stages 5 and 6 talk to.

Almost nothing here needs a design decision. `rule:ide/tasks-carry-a-problem-matcher`,
`rule:ide/the-ast-panel-shells-out-to-the-cli` and
`rule:ide/a-template-region-gets-services-but-no-second-formatter` are all landed and unimplemented —
stages 3, 4 and 6 implement them as written. The one record this goal opens covers stage 2 only: the
`check --json` schema and the test report's move to `schemaVersion: 2`.

Stage 0 is a failing test, not a fix: three command ids have been in the frozen roster since M4B and are
registered by nobody, so running *Novis: Run File* from the palette says **command not found**. The
setting `nvs.lsp.debounce` is the same shape of gap — declared, read by no one, never forwarded to the
server because the client passes no `initializationOptions`.

## Next group

**Stage 0 and stage 2 together**, then stages 3-5. Stage 0 is two assertions in a suite that already
exists; stage 2 is the CLI work every client stage after it queries, and both are Rust-and-manifest
rather than client code.

- [ ] **Write stage 0's two assertions** into `editors/vscode/test/contributions/contributions.test.ts`:
      every id in `contributes.commands` is registered by `activate`, and every key in
      `contributes.configuration` is read by the client or forwarded to the server. Four failures on the
      first run — three commands and `nvs.lsp.debounce`.
- [ ] **Do not remove an unanswered id to make it pass.**
      `rule:ide/contributions-are-frozen-and-only-ever-added` makes the roster public API; an id is
      answered or the test stays red.
- [ ] **Then `nvs check --json`** — a second renderer over `nvs-diagnostics`' existing `Diagnostic`, not
      a second pipeline. `nvs ast --json`'s frozen schema is the shape to copy.
- [ ] **Then the test report.** Adding the declaring file and line is the easy half; the listing mode
      that discovers without executing is the half a Test Explorer cannot work without, and it is what
      decides whether stage 5 is possible at all. Do not leave it for stage 5 to discover.
- [ ] **Coverage is out of scope and stays out.** The Clover/lcov exporters do not exist. Wire nothing
      to `FileCoverage`, and draw no gutter — `rule:ide/the-extension-builds-no-ui-the-editor-already-has`
      is the reason the second option is not a shortcut.
