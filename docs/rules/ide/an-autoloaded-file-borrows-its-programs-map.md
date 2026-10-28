A file a program autoloads or requires is analysed through that program's `autoload` map. It is its own
entry point like every open document, an autoloaded file may not write an `autoload` of its own
(`rule:programs/autoload`) and a required one usually does not, and so without a map lent to it every
`use` of an autoloaded class would be reported as unresolved in a program that builds clean.

The server surveys the tree `nvs.check.scope` selects before its first analysis: a file whose text holds
`autoload` or `require` is walked as a program, and one whose `require` chain declares anything is a
**lender**. A file belongs to a lender when the lender's walk read it, when its path is under a root
one of the lender's *own* declarations names — wherever that root is, inside the workspace or not — or
when it lies under the directory of the lender's entry and is **plain**: its code writes neither
`require` nor `autoload`. The lexer's tokens decide that, so the word in a comment or a string, or a
member of that name, never counts, and a file without either word is plain without being lexed. The
last is the editor's half of `rule:testing/a-directory-of-programs-is-one-test-program`: a directory
`nvs test` runs is one program, so a plain test file beside the file that requires the bootstrap is
analysed through the bootstrap's map, and a file that requires or declares anything is a program of its
own. When such a file is analysed, the lender's declarations are placed behind the file's own, each still
resolving against the directory of the file that wrote it, so a name lands on the file the real build
lands it on.

- **The file's own declarations win.** A prefix both declare is the file's.
- **A lender never borrows.** A file that declares `autoload` has a map of its own, and the server
  analyses it through exactly that one, as `nvs check` does. A bootstrap file is read by the walk of
  every test program that requires it, and without this it would borrow copies of its own declarations.
- **A borrowed declaration reports nothing**, a duplicate and a malformed glob included. Its position is
  in a file this walk never read, and the lender's own analysis is where it is reported.
- **One lender lends, never a union.** Two programs over one source tree may give one prefix different
  roots, and the union is a map neither of them runs with.
- **A lender that owns the file outranks one that only holds it.** A lender owns a file when its walk read
  it or one of its own declarations names a root holding it. It only holds a file it reaches by the
  directory test. The first owner in entry-path order lends. When nothing owns the file, the nearest holder
  lends: the one whose entry's directory is deepest, and the first in entry-path order of two in one
  directory. A program at a project's root holds every plain file in the tree, and its map need not
  name their namespace at all, while a test directory's bootstrap is the program `nvs test` runs the
  directory's files with.
- **The lender's declaring files are dependencies of the borrowing analysis**, so an edit to one
  re-analyses and republishes every open document that borrowed from it. An edit to a file the lender
  read repeats its walk only when the edited text holds `require`, since only a `require` can change what
  the walk reads; an edit to any other file costs one search of its text for the two keywords. A file
  that stops holding `require` leaves the set of files the walk read stale until the lender's next
  walk, and the stale set lends a map only to a file no longer required.

Under `nvs.check.scope = open` only the open documents are surveyed, so a map is lent once the file that
declares it, or the entry that requires it, is open too.

`nvs check <file>` borrows by the same test, and searches only when it must. It checks the file first;
when the walk found no declaration of the file's own program, and the walk or the type phase after it
reported `E0303` or `E0306`, it walks the `.nvs` files under the project root — `--root <dir>`, else
the directory of the first `--config` file, else the working directory, skipping what the server
skips — in path order,
stops at the first lender that owns the file, else takes the nearest that holds it, and checks the file
again with that lender's declarations behind its own. The type phase counts because it is where a
name written in a type, in `new` or before `::class` is reported: a class file that names a class of
its own namespace without a `use` reports nothing until then. A file that declares a
map, or names nothing undeclared, is checked once and searches nothing; a file no lender reaches keeps
the first check's diagnostics, and so does a file checked from a directory outside its project. When
the search ran and found no lender, the first `E0303` or `E0306` carries a note naming the folder it
searched, absolute, and `--root`, so a check run from the wrong folder says so. `E0303`
and `E0306` are the two diagnostics a borrowed map can change, so no other finding starts a search.
Neither does one for a name of one segment, such as `Countable`, or a name under `Core`: an
`autoload` prefix is at least one segment followed by `\`, so no map declares either, and a note
about a map would send the reader to the wrong fix.
The second check starts from a fresh source map, and its diagnostics are the ones reported, in the same
text and `--json` formats; since a borrowed declaration reports nothing, every diagnostic is about a
file the check read for the checked file. `nvs_hir::lenders` is the one implementation of what a lender
is and whom it lends to, for the server and the command both, so the two cannot disagree.

The command borrows because of how it is used. A coding agent is told to run `nvs check` on the file it
just edited, and that file is most often a class file under an `autoload` root; without a lent map the
command it was told to trust reports `E0303` and `E0306` on a program that builds clean, and the false
error sends the agent to fix code that is correct. For such a file the command costs one read of each
`.nvs` file under the root, one name-resolution walk per file holding `autoload` or `require` up to the
lender, and a second check of the checked file; a program's entry point pays nothing new. `--root`
and the note cost a check that succeeds nothing.

`nvs agent hook` checks the edited file as `nvs check` does, in the payload's working directory, but
its root is the agent's project folder: `CLAUDE_PROJECT_DIR`, which Claude Code gives every hook,
else Cursor's first `workspace_roots` entry, else the working directory. That is the tree the
editor's server surveys, so the hook and the editor agree, and an agent whose shell sits in a
subfolder still has its class files lent their map.

Nothing is configured for this. Where two lenders own one file, or hold it from one directory, the first
in entry-path order lends, and nothing says which. The survey costs one read of each file under the root at startup,
which the index makes anyway, and one name-resolution walk for each file that holds a keyword. Each
analysis then costs one `canonicalize` and a prefix test per lender. Nothing is on a request path of the
runtime.

**Memory:** one resolved map and a few short path lists per lender, held for the session. It grows with
the number of programs in the workspace, not with edits. `nvs check` holds the first lender's resolved
map for the length of one command.

**Declined:**
- **A setting naming the entry point.** It is exact, but a false error is the default until somebody
  finds the setting, and a workspace holding several applications needs a list of them.
- **Analysing the program's entry point and publishing for the open file out of that walk.** The cursor
  tables, the trivia and the syntax index would be the entry document's, and a class the entry point never
  names would not be reached at all.
- **Holding `E0306` back for a file with no map.** Hover, completion and go-to-definition would still lack
  the type, and a real misspelt import would be hidden with it.
- **Reporting the file's own `autoload` as an error when it borrows**, as the build does for an
  autoloaded file. A bootstrap file reached by `require` may sit under a root too, and the build gives it
  no error.
- **Repeating a lender's walk on every edit to a file it read.** That set is the whole application, so
  every keystroke anywhere would repeat the walk.
- **The required file's own `require` as the link.** A test file requires nothing. The entry requires
  it, so the edge points away from the open document a walk starts at.
- **A test layout the editor already handles, recommended in place of the usual one.** An entry that
  requires each test file and then the bootstrap is the layout the testing reference describes, and a
  test file that uses a class is the ordinary case.
- **Deciding the search after the name-resolution walk alone.** It saves one type phase for a file
  that borrows, and a same-namespace class named only in a type, in `new` or before `::class` kept a
  false `E0303`, the error this search exists to remove.
- **The first holder in entry-path order.** A program at the root sorts before `tests/bootstrap.nvs`,
  and its map lacks every root the bootstrap adds, so a test file that builds clean reported `E0306`.
- **Surveying by parsing every file.** The keyword search excludes almost every file for the cost of a
  read the index makes anyway.
- **`nvs check` surveying before its first walk.** It costs a tree read on every check of every file,
  the programs that need nothing included.
- **`nvs check` or the hook searching upwards from the checked file for a lender.** In a workspace of
  modules with no marker per module, the lender sits in a sibling folder (`public/`) or in another
  module, so the walk reads everything above the file on every real typo, with nothing to stop it. A
  shared framework tree is also reached by a program outside its own directories, and the server, which
  surveys the workspace, would then disagree with the command. The note and `--root` say where to look.
- **Plain decided by a text search for the two words.** A test file whose comment says `require` was a
  program of its own, and every class its bootstrap maps was reported as unresolved.
