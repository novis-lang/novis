A file a program autoloads or requires is analysed through that program's `autoload` map. It is its own
entry point like every open document, an autoloaded file may not write an `autoload` of its own
(`rule:programs/autoload`) and a required one usually does not, and so without a map lent to it every
`use` of an autoloaded class would be reported as unresolved in a program that builds clean.

The server surveys the tree `nvs.check.scope` selects before its first analysis: a file whose text holds
`autoload` or `require` is walked as a program, and one whose `require` chain declares anything is a
**lender**. A file belongs to a lender when the lender's walk read it, or when its path is under a root
one of the lender's *own* declarations names — wherever that root is, inside the workspace or not. When
such a file is analysed, the lender's declarations are placed behind the file's own, each still resolving
against the directory of the file that wrote it, so a name lands on the file the real build lands it on.

- **The file's own declarations win.** A prefix both declare is the file's.
- **A lender never borrows.** A file that declares `autoload` has a map of its own, and the server
  analyses it through exactly that one, as `nvs check` does.
- **A borrowed declaration reports nothing**, a duplicate and a malformed glob included. Its position is
  in a file this walk never read, and the lender's own analysis is where it is reported.
- **The first lender in entry-path order lends, never a union.** Two programs over one source tree may
  give one prefix different roots, and the union is a map neither of them runs with.
- **The lender's declaring files are dependencies of the borrowing analysis**, so an edit to one
  re-analyses and republishes every open document that borrowed from it. An edit to a file the lender
  read repeats its walk only when the edited text holds `require`, since only a `require` can change what
  the walk reads; an edit to any other file costs one search of its text for the two keywords.

Under `nvs.check.scope = open` only the open documents are surveyed, so a map is lent once the file that
declares it, or the entry that requires it, is open too. `nvs check` never borrows: it is handed the file
a program starts from, and a program has one map.
