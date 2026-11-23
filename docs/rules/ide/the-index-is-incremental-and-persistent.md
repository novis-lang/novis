The language server keeps what it learned about every file of the tree for the whole session, and on disk
between sessions, so a keystroke costs what changed and not the project. A body-only keystroke in a library
that many programs autoload parses one file and checks one file's bodies, whatever the number of programs.
Any other keystroke checks that file and the files that can see what it changed, and no others.
A cold start with a valid index file analyses the open document's entry and the files that changed, and
nothing more. The editor still reports what `nvs check` reports for the open file: a cached analysis and a
fresh one agree after every kind of edit, which `a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit`
in `crates/nvs-lsp/tests/incremental.rs` pins.

**Work is counted, never timed.** An analysis reports `Work { parsed, checked }`, the files it parsed and the
files whose bodies it checked, and the index sums them over every analysis it ran, queued ones included,
because queued work is deferred work. Its `planned` is the most plans the workers ran at once, kept as the
larger of two counts rather than summed. The counters live in the analysis and the index, never in a global.
The counted tests run on a tree that `tests/incremental.rs` generates (`Blog`, `Shop`, then more programs
over one library `lib/Framework/`, or many one-file programs beside an extension), because a kept
fixture's only property is its shape.

**Five caches, each with one key.**

- **A configuration tree** (`crates/nvs-lsp/src/tree_cache.rs`) is kept per list of root files, which every
  analysis finds again as `nvs check` does: the `nvs.toml` beside the document, else the data folder's. It
  is kept with the size and modification time of every path its reader touched: each file it read, each
  file it looked for and did not find, and each directory it listed. An extension set is
  kept per list of `.nvsx` paths and their `sha256`, with each `.nvsx` file's size and time, and with the
  BLAKE3 digest every file's key hashes in place of the set. A lookup reads the metadata of those paths and
  uses the kept answer only when all of them match, so an edit to `nvs.toml` reaches the next analysis. The
  snapshot for the document is built from the kept tree each time, because an `[[app]]` block matches one
  file. It costs one kept tree per distinct list of root files and one kept set per distinct list of entries,
  for the session, and one or two existence tests per analysis. A cold index parses each settings file once, where it parsed it once per file
  of the tree. A file written twice within the file system's time resolution with the same size is the one
  edit a lookup misses, the trade the index file's stamp makes too.

- **A parse** (`crates/nvs-lsp/src/parse_cache.rs`) is kept per canonical path with the BLAKE3 hash of its
  text, and replays the diagnostics its parse and `check_declarations` reported. The `nvs_hir` walk takes it
  through `&mut dyn Parses`; `nvs check` passes none. A file whose text changed is parsed whole, so this is
  not the incremental reparse `rule:ide/one-grammar-one-tree` gave up. Every file keeps its trivia and
  syntax index, because any file may be the next analysis's entry.
- **A file's check output** (`nvs_types::CheckCache`) is its part of the expression table, its diagnostics
  and its rows of the routes, commands, links and codec tables. `check_program_partial` walks the bodies of
  the files it is given and folds every other file's cached output, then runs every whole-graph pass over
  the folded result. Signatures and the whole-graph tables are built every time. `check_program_loaded` is
  the same call with every file walked, so there is one code path. An output is valid for its file's
  id and content hash, the surface rows of every declaration its walk used, whether each name it failed
  to find is declared now, every row of every class above the classes it declares, the extension set and
  the configuration, and the anonymous-function count the file starts at (`crates/nvs-lsp/src/file_key.rs`).
  Which other files are in the graph is not in it, so a file read by many entries has one key.
- **A declaration surface** (`crates/nvs-lsp/src/surface.rs`) is what another file can see, one hashed
  row per declaration, keyed by its symbol as the index writes it (`App\User::greet`). The graph row is
  the namespace, imports, `require` and `autoload` lines. Each type has a header row, an enum a cases row,
  a class a properties row of its property names in order, and each method, property, constant and case
  a member row with its doc comment, modifiers, type, default and signature. Each row hashes its
  attributes apart. A row leaves out bodies, spans, whitespace and the `//`, `#` and `/* */` comments the
  grammar skips; each skipped run between two tokens is one gap byte, so two tokens never run together. A
  `///` doc comment stays, because the checker reads its tags. Where visibility is unclear, a thing goes
  in a row: too much costs time, too little gives a wrong answer. The parse cache cuts the rows once per
  parse, and the index keeps them, with what each file's check used and failed to find, in `Indexed` and
  the index file.
- **An edit reaches the files that can see what changed.** `surface::diff` gives each row an edit added,
  removed or changed, and `SymbolIndex::reached_entries` (`crates/nvs-lsp/src/index.rs`) gives the files
  each reaches, besides its own:

  | What changed | The files it reaches |
  |---|---|
  | a private method or constant | the subclasses, whose finality and redeclaration checks read it |
  | a private property, or the properties row | the subclasses, and every file whose JSON or database codec reaches the class |
  | a protected member | the subclasses, at any depth |
  | a public member or enum case | the files that used it, and the subclasses |
  | an added member, case or type | the files that failed to find its name on the class or a class below, and the subclasses |
  | a class header or the cases row | every file that used the class or a member of it, and the subclasses |
  | a removed symbol | the files that used it, and the subclasses |
  | an attribute, or the graph row | every reader of the file: routes, commands, links and the graph are read across it |

  A use of a member is a use of every type its signature names and every constant its defaults read, so
  an edit to a constant a default reads reaches every caller. An edit queues only the entries that own a
  reached file, and each such entry's analysis checks the reached files it owns and leaves out the rest,
  whose records stay. A body-only edit, and a blank line, an indent or a comment between members, move no
  row and reach no other file. An entry that owns a file with no recorded uses, which an index file
  written before the uses existed has, is analysed whole. A closed file changed on disk is diffed against
  its kept rows at a start's load, and a refresh of a closed file that no analysis parsed yet queues every
  reader. `crates/nvs-lsp/tests/reach.rs` counts each row on a generated tree of 200 entry points.
  Measured with `bun nv lsp-start --copies 12 --cold --edits 5 --kind <kind>` (972 files), five edits'
  runs check, before this reach and after it: `member` 4861 and 0, `private` 4861 and 0, `public` 4861
  and 11, `comment` 0 and 0. The memory it spends is in `crates/nvs-lsp/src/session.rs`.
- **The index file** (`crates/nvs-lsp/src/disk_index.rs`) is every `Indexed` field and the index's reads,
  under `nvs_config::data::Folder::lsp_index()`, named for the BLAKE3 hash of the canonical workspace root.

**Spans stay valid because ids are pinned.** The session's `Pins` table (`nvs-diagnostics`, beside
`SourceMap`) gives a path the `SourceId` it had in every earlier map, so every cached span, table key and row
is valid as it is. A pinned path an analysis does not load is an empty placeholder file.

**One type interner per session.** A cached `TypeId` stays valid because the server keeps one
`TypeInterner` (`crates/nvs-lsp/src/session.rs`). It grows with the distinct types the edits produce, so
the session drops it and every check output once it reaches twice its size at the last settle, and the idle
turns settle the tree again. An analysis shares it copy-on-write through `Rc::make_mut`. If that copy shows
in `crates/nvs-lsp/tests/latency.rs`, the fix is to drop the kept analysis's `Rc` before the check, never to
give each analysis its own interner.

**The index file is untrusted input, and only an editor answer and `nvs code` read it.** `nvs check`,
compiling and running never read it, because they are the authoritative paths. `nvs code` loads and saves
the same file at the same path, and checks every entry and every input by its content hash rather than by
the stamp below (`rule:tooling/nvs-code-answers-about-the-project`). The editor reads and writes it only at
the `Workspace` scope of `rule:ide/check-scope-defaults-to-the-workspace`, and either reader only in a build
with a stamp (`nvs_config::cache::build_stamp`). The layout is the magic `NVSIDX`, a `u32` format version, the
32-byte build stamp, the canonical root, the entries, and a BLAKE3 checksum of everything before it. A wrong
magic, version, stamp or checksum discards the whole file, and the decoder checks each length against the
bytes that remain before it allocates. At the editor's load, an entry whose file's size and modification time match is
trusted. Otherwise the file is hashed, and a match is trusted and takes the new time. Otherwise the file is
parsed for its surface: its entry waits, and so does every entry that read it when the surface differs. A
file that is gone leaves the index and queues its readers, a file the index does not hold is queued, and
an entry that indexed a file with an error is queued, because a class it could not find may exist now. A
stamp is recorded only for a file that is not an open buffer and whose modification time is more than two
seconds before the analysis started, so a write inside one tick of a coarse clock is hashed at the next
start rather than trusted.

The server writes the file in its loop's at-rest arm, once the queue is empty and the index's `generation`
differs from the last write: a temporary `<name>.<pid>.<n>.tmp` beside it, renamed over it, with no `fsync`
because a torn file fails its checksum. A failed rename deletes the temporary file and keeps the old
generation, so the next idle period tries again. Two servers on one root, or a server and an `nvs code`
call, each write a whole file, and the last rename wins. The compiled-code cache sweeps only `.nvsc` files, so it never deletes an index file.

**A start shows its progress.** The survey, the index load and the queue's settling are seconds on a large
tree in which the editor would otherwise show nothing. When the client declared `window.workDoneProgress`, a
start at the `Workspace` scope with a root sends `window/workDoneProgress/create` with a token of its own,
then `$/progress` `begin` titled "Indexing" with "Reading the workspace" before the survey. Once the index is
loaded or planned, and at every turn of the loop, a `report` gives "N of M files" and a percentage, sent
only when the whole percent rises, so a run is at most a hundred reports. `end` follows at the first turn
with no entry waiting, whether the idle turns, a whole-tree request's `settle` or a rebuild emptied it, and
at once when nothing was stale. A scope change that rebuilds the index begins a fresh run, ending one still
going first, so a client never sees two `begin`s without an `end`. An edit that queues entries after the
start's run ended begins a run of its own, and its `end` logs the index's work since the start. A client that did not declare the
capability is sent nothing, since a token it never created is a protocol error to it. `begin` follows the
`create` without waiting for its response, as rust-analyzer does, and the loop and `shut_down` drop the
response. `crates/nvs-lsp/src/progress.rs` is the mechanism and `crates/nvs-lsp/tests/progress.rs` pins
the order.

**Memory is spent to buy latency on every keystroke.** For every file of the tree the session keeps its
statements, trivia and syntax index, its parse diagnostics, its check output and its `Indexed` entry, which
is O(project) with no growth from edits beyond the interner's bound. A file's entries are freed when it
leaves the tree. `crates/nvs-lsp/src/session.rs` gives the measured bytes per file and how they were taken.

**Declined:**
- **A full rebuild at every start.** It is the cost this rule exists to remove.
- **A dependency for serialisation.** The format is written by hand over the index's own types
  (`rule:ide/dependencies-are-allowlisted`).
- **A background thread for analysis, load or write** (`rule:ide/the-server-is-synchronous`). The write is
  one idle turn's work.
- **Parsed statements on disk.** The format would mirror the whole syntax tree, and the open document's
  graph parses quickly enough to be the cold path.
- **A stat-based skip of the file read in the parse cache.** The read is not the measured cost.
- **A key per body.** A file's walk is one synthesized frame, and the anonymous-function count threads
  from file to file, so a key per body would need a second numbering. A reached file is one file's check,
  a few milliseconds. Reopen only if `bun nv lsp-start` shows one file's check again over 20 ms on the
  972-file tree.
