The Test Explorer is a projection of two documents the CLI writes: `nvs test --list --format=json`
fills the tree, and `nvs test --format=json --filter` runs a selection. A record is matched to a
`TestItem` by its `class` and `method`, and the file each item opens at is the `file`, `line` and
`column` the record carries — the client derives none of it.

Discovery must not run anything, which is why it is a flag on the runner rather than a run with every
test skipped: `--list` answers from the table the compile already built, so a workspace whose tests
fail, hang or `exit` populates a tree exactly as a passing one does.

Coverage is not wired into the explorer. `nvs test --coverage-lcov` and `--coverage-clover` write a
run's line coverage (`rule:testing/coverage-report`), and nothing in the extension reads either file
yet. When it does, the counts reach VS Code's own `FileCoverage` model and the extension still draws
no gutter of its own (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). An explorer
without coverage is the whole feature minus one column, not a stub.

This is the shape `rule:ide/the-ast-panel-shells-out-to-the-cli` already gives a view over compiler
tables: the binary answers, and the client renders. A client that scanned the workspace for test
classes itself would be a second implementation of the `#[Test]` table, free to disagree with the one
that runs.
