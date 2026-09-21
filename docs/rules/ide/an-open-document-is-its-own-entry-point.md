Document sync is `Full` at M4B: a whole-document push per keystroke over a pipe is not the cost that
matters when the analysis behind it is a full reparse anyway. Analysis is debounced (150 ms,
`nvs.lsp.debounce`) and cancelled by the next keystroke.

The unit of analysis is one open document as its own entry point. Its `require`/`autoload` graph is
resolved exactly as `nvs check` resolves it, with open buffers overlaid on what is on disk, so a class
edited in one tab and used in another resolves to the unsaved text. Diagnostics are published only for
**open** documents — publishing for a file nobody opened is workspace-wide analysis, which is M10's.
Go-to-definition may still land in a closed file; the editor opens it. The one thing such a walk takes
from outside its own graph is the `autoload` map of the program that autoloads the document
(`rule:ide/an-autoloaded-file-borrows-its-programs-map`).

Editing one document re-analyses every open document whose graph contains it. Otherwise an open `A.nvs`
that requires an edited `B.nvs` is stale until touched, which reads as the server being wrong. The resolved
graph is already in hand from the analysis that produced `A`'s diagnostics, so this is a reverse index
rather than new work.
