`nvs code` has five read-only verbs over one project: `show`, `refs` and `impls` take a name, and
`outline` and `types` take a file (`types` also `file:line` or `file:first-last`), and `rename` writes.
Each answers from `nvs_lsp::workspace` through `nvs_lsp::query`, the functions the editor's references,
implementation and rename requests call, so the command and the editor give one answer to one question
(`rule:ide/five-features-are-one-reference-index`). It is a command of its own beside `nvs agent`,
which answers about the language with no project open; this one needs a project and answers about code
that may not compile.

- **The name** is the index's own spelling: `Shop\Cart`, `Shop\Cart::addItem`, `Shop\Cart::$total`,
  `Shop\Cart::MAX`, `Shop\Order::Status`. There is no second spelling. A name the project does not
  have fails with exit status 1 and the nearest names, compared without case and without backslashes,
  so a name the shell stripped finds the one it meant. A `Core` name is valid for `refs` and `impls`,
  which list the project's uses of it, and `show` of one prints the card `nvs agent show` prints, from
  the same function.
- **The project** is the directory of the nearest `nvs.toml` at or above the working directory, and
  the working directory where there is none. Every `.nvs` file under it is analysed, as the editor
  analyses a workspace, except what the project walk skips. That walk is `nvs_hir::lenders::sources`,
  the one walk `nvs code`, the editor's index, `nvs check`'s lender search and `nvs fmt` share. It
  skips a path a `.gitignore` or `.ignore` names, read in every directory at and under the root and
  also outside a git repository. It reads no global git excludes file, no `.git/info/exclude` and no
  ignore file above the root, so one tree gives one file list on every machine and in CI. It never
  enters `target/`, `vendor/` or a dot-directory, reads a dot-file, follows no symbolic link, and
  always walks the root it is given.
- **A position** is `path:line:column`: the path relative to the working directory, the line and the
  column from 1, the column in characters. JSON carries the byte `start` and `end` beside it.
- **A `refs` line** is the position, the kind (`declared`, `import`, `use`) and the source line without
  its indentation: the declaration first, then the rest in path and line order. `show` prints the
  signature `nvs meta` writes for a program member, from the one builder both call, and the `///` run
  as hover renders it.
- **The JSON** follows `rule:ide/check-json-is-the-diagnostic-record-as-a-document` as `nvs agent
  --json` does: `schemaVersion: 1` at the root, every key present, and a failure prints `error` and
  `nearest` on standard output while the exit status is 1.
- **Code with errors still answers**, from what the analysis recovered, with exit status 0. Standard
  error gets one line with the number of files that have errors, because a use inside one may be
  missing. An empty answer is exit status 0 too.
- **Text from the project** — a source line, a doc comment, a signature, a type, a path — is printed
  with every control character but tab written as `\u{..}`, in the text and the JSON. A file's bytes
  reach a terminal or an agent's context here, and must reach it as text and nothing else.
- **`rename <name|path:line:column> <new-name>`** is the editor's rename, `nvs_lsp::query::rename`
  over `rename::rename`. A name renames at its declaration; a position is the only way to name a
  local. It refuses with the editor's own message, and a `$` on the new name is optional. It writes
  by default, in this order, and a failure at any step writes nothing: every edit is computed; every
  path and every moved file's target is judged inside the project root before anything is joined;
  each file is read again and one whose bytes changed since the call first read it fails, named;
  then each file is written with `fs::write`, as `nvs fmt` writes, and a type's file named after it
  is moved last. Without `--dry-run` it prints one line per changed file with its number of edits
  and one per moved file; with it, one line per edit with the position, the old text and the new
  text, and nothing is written. `--json` gives `edits`, `moved` and `written`. A debug build reads
  one test variable, `NVS_TEST_CODE_CHANGE_BEFORE_WRITE`, that changes a file between the two reads;
  a release build has no such hook.

A call costs one cold analysis of the project, one `analyse_file` per `.nvs` file under its root, and
nothing outlives the call. No limit and no setting is added. The walk reads each ignore file once and
keeps its parsed patterns, a few KiB, until it ends.

- **Performance:** nothing on the request path; this is tooling.
- **Memory:** one index over the project for the length of one call, and nothing per request.
- **Usability:** an agent finds a method's callers, an interface's classes and a variable's type in
  one call, without the wrong answers `grep` gives for a method name two classes share or a class
  reached through `autoload`.
- **Simplicity:** one command over functions the editor already has, and `nvs agent` keeps its four
  verbs.

**Declined:**
- **An exclude key in `nvs.toml`.** A project already lists what it does not write in `.gitignore`,
  and every project would repeat that list in a second place that drifts from the first.
- **Reading the global git excludes file and `.git/info/exclude`.** Both are one machine's settings,
  so the same tree would give a different file list on a colleague's machine or in CI.
- **A journal or a staged write for `rename`.** A crash between two writes leaves the project half
  renamed, and the next `nvs check` names every site that no longer resolves. `nvs fmt` writes the
  same way, and nothing on the request path is involved.
- **A disk cache of the index.** It is a second copy of the project that can be stale, and a stale
  answer is worse than a slow one. A call measured too slow on a large project is a gap against
  `nvs-lsp`, not a cache.
- **A background daemon.** It is the same stale copy, kept in memory, with a process to manage.
- **A fifth `nvs agent` verb.** `nvs agent` answers about the installed language with no project, and
  every answer it gives is true of the binary; an answer about a project's code is a different kind
  of answer (`rule:tooling/an-agent-asks-the-binary`).
- **Call hierarchy.** `rule:ide/five-features-are-one-reference-index` declines it for the editor, and
  this command answers what the editor answers.
