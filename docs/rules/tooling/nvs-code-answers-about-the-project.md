`nvs code` has seventeen verbs over one project. Nine read: `show`, `refs` and `impls` take a name,
`calls` and `reach` take one or more method names, `outline` and `types` take a file (`types` also `file:line` or `file:first-last`),
`find` takes part of a name, and `unused` takes files or directories, or none for the whole project.
Eight write: `rename`, `move`, `delete`, `extract`, `implement`, `inline`, `signature` and `order`.

**`unused`** prints each declaration nothing uses, from `SymbolIndex::unused`
(`crates/nvs-lsp/src/index.rs`), one line each: position, kind, name. It exits 0. A declaration is
used when the index holds an occurrence of it, and also when the runtime or a tool may reach it with
none: a route handler, a test, a matched test request's handler, a declaration with an attribute, a
constructor, a method the call graph does not follow, a method of an override family where any method
is used, a method of a class with a type above it outside the index (a `Core` interface the runtime
calls), a public property (an encoder or a template reads it by name), a class with a type above it
in a project that calls `Core\Program::implementing`, `implementingWith` or `constructors`, an enum
case of an enum whose `cases` is called, a name a string literal or an `nvs.toml` writes, and a type
with a used member. In a project with a `package.toml`, every `public` and `protected` declaration is
the package's API and is never listed. Each doubtful case counts as a use, so a listed declaration is
safe to delete. **Cost:** the index keeps each file's distinct string-literal words and one flag per
declaration for an attribute, for as long as the file is indexed; a call reads each `nvs.toml` once.

**`delete`** takes a name and deletes that declaration, from `nvs_lsp::delete::delete`, the function
the editor's *Delete unused …* action (`refactor.rewrite`) calls at a greyed name. It deletes only
what `unused` lists, and refuses a declaration with an occurrence, a `@see` or `@throws` mention or
an import, a method that overrides or is overridden (deleting it changes which body runs), and one
that shares its statement with another. It removes the declaration's lines with its `///` and `#[`
lines and one blank line beside it. A type whose file holds nothing else but its `namespace` and
`use` lines deletes the file, refused while a `require` names it; the editor does not offer that
case, since an action edits one document's text. The edit passes
`rule:ide/a-refactoring-checks-its-edit-before-it-writes`, and the JSON lists each deleted file under
`deleted`, a key every write verb's JSON has. **Cost:** tooling only: one analysis of the
declaration's file, one of each reader of a deleted file, and the check.

**`extract variable`** and **`extract method`** take a range, `path:line:column-line:column`, whose
last column is part of the selection, and an optional `--name`. They write what the editor's two
`refactor.extract` actions write over that selection, from `nvs_lsp::extract::reasoned`, the form of
`extract::at` that returns the first refusal as a sentence: the editor offers only what works and so
shows none, and the command prints it and exits 1. A given name that is no identifier, or that the
file already writes, is refused. **`implement`** takes a class name and writes every method the
class's interfaces and `abstract` parents leave without a body, as the editor's *Implement the
missing methods* fix writes it on `E0449` (`nvs_lsp::generate::implement_class`); a class that owes
none is refused. Each edit passes `rule:ide/a-refactoring-checks-its-edit-before-it-writes`.
**Cost:** tooling only: the editor's analyses for the action, then the check.

**`inline`** takes a method's name, or a position, `path:line:column`, on a local, a call or a
method's name, from `nvs_lsp::query::inline`, the function the editor's three `refactor.inline`
actions share (`crates/nvs-lsp/src/inline.rs`). A local declared once with a value and only read
after it has each read replaced by the value, and its declaration deleted. A value with no call,
`new`, assignment, increment or property read replaces every read while no local it reads is
written between the declaration and the last read, or the end of a loop a read is in. Any other
value replaces one read only, in the next statement, where nothing before it in that statement runs
code and it runs every time the statement does. A typed declaration is inlined only where the value
has the same type. A method's name inlines every call and deletes the method; a position on one call
inlines that call and keeps it. The body must be one `return <expression>;`, or a `void` body with no
`return` and no variable it declares at a call that is a statement. Refused: a method with an
attribute (the inlined code would run without it), an abstract one, one that overrides or is
overridden, is taken as a callable value or calls itself, a body that changes a parameter or writes
`static::` or `parent::`, a `self::` where the call is in another class, a `$this` where the receiver
is no variable, a call with a diagnostic on it (a `[[restrict]]` entry's included, so inlining never
moves a restricted body to a place the entry forbids), and an argument, or a default filling a
missing one, that is not a variable or a literal, since it could run twice or not at all. Each read
or call is put in parentheses where the place it lands needs them, and each edit passes
`rule:ide/a-refactoring-checks-its-edit-before-it-writes`. **Cost:** tooling only: one walk of the
local's file and one analysis when its declaration writes a type, or one analysis of the method's
file and of each calling file, and the check. In the editor, each action analyses the rewritten
document once more, and is computed only with the cursor on a variable or a method's name.

**`signature`** takes a method's name and its whole new parameter list, as source writes it, with
`--value '$name=<expression>'` once per added parameter that needs one, from
`nvs_lsp::query::signature` (`crates/nvs-lsp/src/signature_change.rs`). Parameters match the old
list by name, so a kept one may change its type, default or place; a parameter's rename is
`rename`'s. An added one needs a default or a value, which is written at every call; with both,
calls keep the default. A removed one deletes its argument at every call. Every method of the
override family gets the list, each keeping its own parameter names and, for what the list does not
change, its own modifiers and spelling; a changed or added type is written as the list writes it, and
a file that needs another spelling is refused by the check. An argument after the first one a call
leaves to its default, or after a named one, is written with its name, and each removed parameter's
`@param` line goes. Refused, since each could change what runs or miss a call: a removed argument
that may run code (a call, `new`, an assignment, a property read), a removed parameter a body of the
family reads, a reorder that swaps two arguments that may run code, a change of `inout`, a variadic
parameter, a family whose methods differ in parameter count, a method of it taken as a callable
value, a type above it outside the workspace, a call that spreads its arguments or nests in another
call of the family, and a constructor, whose `new` calls the index records as uses of the class.
Each edit passes `rule:ide/a-refactoring-checks-its-edit-before-it-writes`. **Cost:** tooling only:
one parse of each file that declares or calls a method of the family, then the check.

**`move`** with a `from` that contains `::` moves that member into the class `to`, from
`nvs_lsp::query::move_member` (`crates/nvs-lsp/src/move_member.rs`), the function the editor's *Move
… to another class…* action runs through `workspace/executeCommand` (`nvs.moveMember`). No path
contains `::`, so the two forms never meet. A static method, a static property, a constant, or a type
alias or an enum a class declares moves to any class of the project: each `Old::member`, `self::`,
`static::` and `parent::` that names it becomes the new class, with a `use` line where the file needs
one, and inside the moved code each `self::` and `static::` that reads another member of the old class
becomes the old class's name. A method or a property that is not static moves only to a class above
its class or below it, and its uses, written on an object, do not change. The moved code keeps its
`///` and `#[` lines, lands after the last member of its group (constants, then properties, then
methods), and each type it writes by a short name gets a `use` line in the new file. Refused, since
the checker cannot see each: a private or protected static member the old class still uses ("make it
public first"), a property whose name a property callback compares as a string
(`rule:classes/a-compared-property-name-is-a-use`), moved code that writes `parent` or a `self` or
`static` not in front of `::`, a name the new class already has, a member that shares its declaration
with another, and an enum case. The rest is the check's: a pull up whose body reads what the class
above lacks, a push down that something still calls through the class above or a sibling, and a moved
body that reads a private member of the old class each add an error, so each is refused with it
(`rule:ide/a-refactoring-checks-its-edit-before-it-writes`). **Cost:** tooling only: one parse of each
file with a use of the member, one tokenizing of the old file, then the check.

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
  is moved last. A qualified new name, `Shop\Billing\Cart`, moves a type to that namespace with its
  `use` lines and its file (`rule:ide/a-type-moves-with-its-namespace`), and the missing directories
  of the file's new path are created just before the move, each judged inside the root first.
  Without `--dry-run` it prints one line per changed file with its number of edits
  and one per moved file; with it, one line per edit with the position, the old text and the new
  text, and nothing is written. `--json` gives `edits`, `moved` and `written`. A debug build reads
  one test variable, `NVS_TEST_CODE_CHANGE_BEFORE_WRITE`, that changes a file between the two reads;
  a release build has no such hook.
- **`rename --from <file>`** applies a list of renames as one plan, `nvs_lsp::query::rename_all`
  over `rename::renamed_all`, and conflicts with the two positional arguments. Each line of the
  file is `<target> <new-name>`, the two separated by white space; a blank line and a line that
  starts with `#` are skipped, and a line that names nothing fails the list with its number. Every
  rename is computed against the one index, so each position is read in the files as they were.
  A name another rename of the list takes away is free for the clash checks, so two names swap,
  and two files that swap paths are each moved aside first. Two renames that give one new name,
  or that edit one byte range, refuse the list and name both lines. The merged edit is checked
  once (`rule:ide/a-refactoring-checks-its-edit-before-it-writes`) and written in one write. The
  output and `--dry-run` are `rename`'s, and each `--json` edit carries `fromLine`, the line of
  the list it came from, since `line` is its position's. A message about one rename starts with
  `line N:`. **Cost:** tooling only, one plan per line and one check for the whole list.
- **`move <from> <to>`** is the editor's file move, `rename::relocated`, and writes through the
  same path as `rename` with one move. `<from>` is a file or a directory that exists inside the project
  root, and a directory moves with every file under it. `<to>` is inside the root, does not exist,
  is not inside `<from>`, and its directory exists. Both are judged before anything is computed, and
  a failure writes nothing. Each `require` literal that names a moved file, or that a moved file
  writes, is written again first, keyed by the file's current path, and then `fs::rename` makes the
  move; a move that fails after the edits names the files already changed. The output and `--json`
  are `rename`'s, with the move as the one entry of `moved`.
  - **A moved file takes the name its path gives.** Under an `autoload` root the path is the name
    (`rule:programs/one-declaration-per-autoloaded-file`), so a move that kept the old name would
    always break the program. For each moved `.nvs` file a program's map claims, the name that map
    finds the new path by (`AutoloadMap::name_at`, the deepest root that holds it) is the name its
    sole declaration takes, through the plan `rename --from` builds, with the file move already
    decided. A new folder gives a new namespace, a new file name a new last segment, and a folder
    moves every type under it in one plan, where two types that move into one namespace need no
    `use` line for each other. A file only a `require` reaches keeps its names. An autoloaded file
    that no root reaches at its new path is refused, and so are programs whose maps give it
    different names. The editor's `workspace/willRenameFiles` returns the same plan, and only the
    `require` edits where the plan is refused, because a drag cannot be refused; the diagnostics
    then name what broke. **Cost:** tooling only, one analysis per program that reads a moved file
    and one rename plan per renamed type.
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
- **`move` creating a missing directory.** The user types the new path, so a typo in it would
  create a folder nobody asked for. `rename` to a full name creates the folders its new name needs,
  because there the autoload map gives the path and nobody typed it.
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
