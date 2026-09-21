A file a program autoloads is analysed through that program's `autoload` map. It is its own entry point
like every open document, it may not write an `autoload` of its own (`rule:programs/autoload`), and so
without a map lent to it every `use` of an autoloaded class would be reported as unresolved in a program
that builds clean.

The server surveys the tree `nvs.check.scope` selects before its first analysis: a file whose text holds
the keyword is walked as a program, and one whose `require` chain declares anything is a **lender**. A
file lies under a lender when its path is under a root one of the lender's *own* declarations names —
wherever that root is, inside the workspace or not. When such a file is analysed, the lender's
declarations are placed behind the file's own, each still resolving against the directory of the file
that wrote it, so a name lands on the file the real build lands it on.

- **The file's own declarations win.** A prefix both declare is the file's, which is what keeps a
  bootstrap file that happens to sit under a root analysed as itself.
- **A borrowed declaration reports nothing**, a duplicate and a malformed glob included. Its position is
  in a file this walk never read, and the lender's own analysis is where it is reported.
- **The first lender in entry-path order lends, never a union.** Two programs over one source tree may
  give one prefix different roots, and the union is a map neither of them runs with.
- **A program never lends to its own entry point**, which `nvs check` and the server analyse identically.
- **The lender's declaring files are dependencies of the borrowing analysis**, so an edit to one
  re-analyses and republishes every open document that borrowed from it. An edit to any other file costs
  one search of its text for the keyword.

Under `nvs.check.scope = open` only the open documents are surveyed, so a map is lent once the file that
declares it is open too. `nvs check` never borrows: it is handed the file a program starts from, and a
program has one map.
