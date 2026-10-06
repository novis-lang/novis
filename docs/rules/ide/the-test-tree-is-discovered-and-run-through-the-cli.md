The Test Explorer is a projection of two documents the CLI writes: `nvs test --list --format=json`
fills the tree, and `nvs test --format=json --filter` runs a selection. A record is matched to a
`TestItem` by its `class` and `method`, and the file each item opens at is the `file`, `line` and
`column` the record carries — the client derives none of it.

Discovery must not run anything, which is why it is a flag on the runner rather than a run with every
test skipped: `--list` answers from the table the compile already built, so a workspace whose tests
fail, hang or `exit` populates a tree exactly as a passing one does.

Coverage is a third document from the same run. A run profile of kind `Coverage` beside the Run one
adds `--coverage-lcov` with a scratch file in the extension's storage directory and starts the run in
the program's workspace folder, which is what the tracefile's names are relative to
(`rule:testing/coverage-report`). Every `SF` record becomes one `FileCoverage` built from its `DA`
lines as statements, its `BRDA` sides as branches on the line they start on, and its `FN`/`FNDA` pairs
as declarations, so the totals VS Code shows are its own count of the runner's details. The extension
draws no gutter of its own (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). The `.nvst`
corpus runs unchanged under that profile, because the CLI refuses a coverage flag over a case tree.

This is the shape `rule:ide/the-ast-panel-shells-out-to-the-cli` already gives a view over compiler
tables: the binary answers, and the client renders. A client that scanned the workspace for test
classes itself would be a second implementation of the `#[Test]` table, free to disagree with the one
that runs.
