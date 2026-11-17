The Test Explorer is a projection of two documents the CLI writes: `nvs test --list --format=json`
fills the tree, and `nvs test --format=json --filter` runs a selection. A record is matched to a
`TestItem` by its `class` and `method`, and the file each item opens at is the `file`, `line` and
`column` the record carries — the client derives none of it.

**A project is listed and run as one program, the way `nvs test .` runs it.** Each `.nvs` file
belongs to the deepest workspace folder, or folder holding `nvs.toml` or `package.toml`, at or above
it, the project bound `nvs check` uses (`rule:ide/an-autoloaded-file-borrows-its-programs-map`). That
folder is the path `nvs test` is given and the folder it runs in, so its `nvs.toml` is read, a test
file gets its bootstrap's `autoload` map, and a test request runs the folder's `public/index.nvs`
(`rule:testing/a-directory-of-programs-is-one-test-program`). The tree shows the folder, then each
file that holds tests. A folder program also compiles the files of a project nested in it, so its
tree keeps only the tests of files it owns. A folder whose listing is a case tree, or whose files do
not compile as one program, such as a folder of separate examples, lists each of its files alone, as
its own program run in that folder. Cost: one compile per project folder, and one per file only in a
folder that does not compile as one program.

Discovery must not run anything, which is why it is a flag on the runner rather than a run with every
test skipped: `--list` answers from the table the compile already built, so a workspace whose tests
fail, hang or `exit` populates a tree exactly as a passing one does. The `.nvst` corpus is the same
flag over each workspace folder: every `case` record is one leaf, and the client searches for no case
file itself. A workspace folder with programs and no case pays one compile of the folder as a
program for that listing, and the client reads none of its records.

Coverage is a third document from the same run. A run profile of kind `Coverage` beside the Run one
adds `--coverage-lcov` with a scratch file in the extension's storage directory and starts the run in
the program's project folder, which is what the tracefile's names are relative to
(`rule:testing/coverage-report`). Every `SF` record becomes one `FileCoverage` built from its `DA`
lines as statements, its `BRDA` sides as branches on the line they start on, and its `FN`/`FNDA` pairs
as declarations, so the totals VS Code shows are its own count of the runner's details. The extension
draws no gutter of its own (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). The `.nvst`
corpus runs unchanged under that profile, because the CLI refuses a coverage flag over a case tree.

This is the shape `rule:ide/the-ast-panel-shells-out-to-the-cli` already gives a view over compiler
tables: the binary answers, and the client renders. A client that scanned the workspace for test
classes itself would be a second implementation of the `#[Test]` table, free to disagree with the one
that runs.

**Declined:**
- **Listing and running each `.nvs` file as a program of its own.** A test file that uses its
  project's classes does not compile alone and drops out of the tree, and a test request runs the
  test file's own empty script, so a page test fails in under a millisecond with an empty body.
- **Showing a folder's compile error as the folder's item.** A folder of separate examples does not
  compile as one program and is not wrong, so every such folder would show an error.
- **Asking the language server where a test is declared.** It is the client reconstructing what the
  CLI's document should carry, and a client feature whose CLI surface is missing is the CLI's to fix
  (`rule:ide/every-feature-is-staged-behind-its-dependency`).
