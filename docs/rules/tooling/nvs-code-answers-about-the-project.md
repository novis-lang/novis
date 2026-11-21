`nvs code` has eleven verbs over one project. Eight read: `show`, `refs` and `impls` take a name,
`calls` and `reach` take one or more method names, `outline` and `types` take a file (`types` also `file:line` or `file:first-last`),
and `find` takes part of a name. Three write: `rename`, `move` and `order`.
Each answers from `nvs_lsp::workspace` through the functions the editor's references, implementation,
rename, file move, workspace symbol, call hierarchy and used-by requests call, mostly through
`nvs_lsp::query`, so the command and the editor give one answer to one question
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
  always walks the root it is given. A search for the program of one file also walks the skipped
  folder that file is in (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).
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
- **`move <from> <to>`** is the editor's file move, `rename::moved`, and writes through the same
  path as `rename` with one move. `<from>` is a file or a directory that exists inside the project
  root, and a directory moves with every file under it. `<to>` is inside the root, does not exist,
  is not inside `<from>`, and its directory exists. Both are judged before anything is computed, and
  a failure writes nothing. Each `require` literal that names a moved file, or that a moved file
  writes, is written again first, keyed by the file's current path, and then `fs::rename` makes the
  move; a move that fails after the edits names the files already changed. The output and `--json`
  are `rename`'s, with the move as the one entry of `moved`.
- **`order [paths...]`** is the editor's *Order class members* action, `nvs_lsp::order::of`, run
  over each file a path names (a file or a directory under the root) or every file of the project
  (`rule:tooling/class-members-have-one-order`). Each changed file is one whole-file edit through the
  same write path as `rename`, so a file changed since the call read it fails, named, and nothing is
  written. It prints one line per changed file with its number of moved classes, then one note per
  derived class whose JSON key order changes or that `Core\Json\KeepKeyOrder` leaves as written.
  `--dry-run` prints one line per class it would move, then the notes. `--check` prints the same
  class lines and no notes, writes nothing, and exits 1 when there is one. A file with a syntax
  error, or whose moved text does not format, is skipped with one line on standard error, and the
  others are still ordered. `--json` gives `files` (each a `path` and its `classes`), `notes` (each
  a class and its `note`), `skipped` and `written`.
- **`find <text>`** is the editor's workspace symbol search, `SymbolIndex::matching`: one line per
  declaration whose own name (a type's last segment, a member without its `$`) contains the text,
  upper and lower case alike, with its position, its kind as `outline` prints it, and its full name,
  in name order. No match prints nothing and exits 0. `Core` is not searched. `--json` gives `query`
  and `declarations`.
- **`calls <method>...`** is the editor's call hierarchy, `calls::incoming` and then `calls::outgoing`:
  each method on a line of its own with `caller` or `callee`, and under it one indented line per call
  site with its position and source line. A `Core\Test::request` the route table matched is a call
  to the route's handler. `--json` gives `callers` and `callees`, each a `symbol` and its `sites`.
- **`reach <method>...`** is the used-by lens, `reach::reach`
  (`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`): a `route` line per route, a
  `test` line per test, and an `at least` line last when `Reach::at_least` is set. `--json` gives
  `routes`, `tests` and `atLeast`. For `calls` and `reach`, a name that is not one of the project's
  methods fails as an unknown name does, with the nearest method names from `query::nearest_methods`.
- **Several methods** in one `calls` or `reach` call answer each name once, in the order given. Each
  gets a line with its name, then the lines one name prints, indented by two spaces. With one name the
  output is byte for byte the one-name output. `--json` with several prints `schemaVersion` and
  `methods`, a list of the one-name documents without their `schemaVersion`. Any unknown name fails the
  whole call with exit status 1 before anything is printed: the text names each unknown name with its
  nearest names, and the JSON gives `error` and `unknown`, a list of `symbol` and `nearest`. One unknown
  name alone keeps the one-name failure. `show`, `refs` and `impls` keep one name, since each answer is
  long and a second name is a second call.

**A call answers from the editor's index file, and never from a stale entry.** `nvs code` loads the
file the editor keeps for the same root (`index_file` in `crates/nvs-lsp/src/server.rs`, under
`nvs_config::data::Folder::lsp_index()`), settles every stale entry on the workers, answers, and saves
the file through `SymbolIndex::save_to` when it analysed anything. A loaded entry is trusted only where
all five guarantees hold:

1. **Content, not time.** Every indexed file's BLAKE3 content hash is compared with the file's bytes. A
   matching size and modification time is never trusted. The hash pass runs on the workers.
2. **The file list is walked fresh**, by the project walk above. A new file is analysed. A file that is
   gone or newly ignored leaves the index, and every entry that read it is analysed again.
3. **A surface change re-analyses every reader**, as the editor does
   (`rule:ide/the-index-is-incremental-and-persistent`).
4. **Another build discards the whole file**, through the build stamp in its header.
5. **Every input outside the `.nvs` files is checked by content:** each `nvs.toml` and settings file an
   analysis read, each `.nvsx` extension and each ignore file. The format records each with its content
   hash. An input the call cannot prove unchanged makes it run cold.

The proof is a comparison with a cold build. `a_content_checked_load_answers_as_a_cold_build_after_every_kind_of_edit`
in `crates/nvs-lsp/tests/index.rs` edits a generated tree in every way (a body, a signature, adding,
deleting and moving a file, an ignore file, `nvs.toml`, an extension, and new content with the old size
and time), and after each edit the loaded index equals the one `SymbolIndex::build` gives cold. A slow
answer is acceptable and a stale one is not.

The workers are `use_workers(0)`, the cores minus one, and they are stopped before the answer prints.
Two writers, an editor and a call or two calls, each rename a whole file over the index file, and the
last rename wins. A failed save prints nothing and changes no answer. A call with no data folder or a
build with no stamp runs cold, on the workers. The tests pass `--data <scratch>`, so no index file
reaches the binary's own data folder. `nvs check`, compiling and running never read the file. The walk
reads each ignore file once and keeps its parsed patterns, a few KiB, until it ends.

- **Performance:** nothing on the request path; this is tooling. A call on an unchanged tree hashes
  its files and analyses nothing.
- **Memory:** one index over the project for the length of one call, and nothing per request. The peak
  is a cold index's, and a loaded index holds less while it settles. On disk, one index file per
  project root, the one the editor already writes, so it grows with projects and never with calls.
- **Usability:** an agent finds a method's callers, an interface's classes and a variable's type in
  one call, without the wrong answers `grep` gives for a method name two classes share or a class
  reached through `autoload`.
- **Simplicity:** one command over functions the editor already has, and `nvs agent` keeps its four
  verbs.

**Declined:**
- **An exclude key in `nvs.toml`.** A project already lists what it does not write in `.gitignore`,
  and every project would repeat that list in a second place that drifts from the first. An include
  key is declined for the same reason: a `!` line in `.ignore` brings a folder back for every tool
  that reads it, and a file in an ignored folder already finds its program.
- **Reading the global git excludes file and `.git/info/exclude`.** Both are one machine's settings,
  so the same tree would give a different file list on a colleague's machine or in CI.
- **A journal or a staged write for `rename` and `move`.** A crash between two writes leaves the
  project half changed, and the next `nvs check` names every site that no longer resolves. `nvs fmt`
  writes the same way, and nothing on the request path is involved.
- **`move` renaming the type a moved file declares, or creating a missing directory.** The editor's
  file move does neither. A type is renamed with `rename`, which moves its file too, and after a
  move `nvs check` names any other name that no longer resolves.
- **Trusting a file's size and modification time, as the editor does.** A stale editor answer lasts
  until the next keystroke, but a stale `nvs code` answer is acted on by an agent and outlives the
  call. Hashing every file is a fraction of a second on the workers.
- **A background daemon.** It is a copy of the project kept in memory between calls, with a process
  to manage, and the index file already makes a call on a settled tree cheap.
- **A fifth `nvs agent` verb.** `nvs agent` answers about the installed language with no project, and
  every answer it gives is true of the binary; an answer about a project's code is a different kind
  of answer (`rule:tooling/an-agent-asks-the-binary`).
- **An `--in` or `--out` flag on `calls`.** One call answers who uses a method and what it uses, and
  both directions together are short.
- **`find` searching `Core`.** `nvs agent find` already searches the installed standard library, and
  a second search would be a second answer to one question.
