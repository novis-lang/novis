Every refactoring applies its edit in memory and analyses it before anything is written, and an edit
that adds an error is refused with that error's message and position.

One function does it, `nvs_lsp::refactor::checked`, and both clients call it: `nvs code` before it
writes a file and the server before it answers. So the command and the editor refuse the same edit
with the same message. It takes the refactoring's rewrites and file moves, overlays the changed texts
on the open buffers, and analyses every entry point whose analysis read a changed or moved file. The
index already records those readers (`rule:ide/the-index-is-incremental-and-persistent`). Each entry
is analysed twice, as it is now and with the edit, and errors are counted per file and per code. The
first error whose count grew is the refusal. A message and a position are not compared: an error that
names a renamed symbol changes its text without being new, and the edit moves the text.

- **What passes through it:** rename, in both clients, with the file a type's rename moves.
  `nvs code move`. The four generators of
  `rule:ide/a-code-action-writes-only-what-is-already-determined`, which check only the edited
  document, since their edit adds a declaration to it and nothing else. That form analyses the
  rewritten document once and counts against the analysis the request already has.
- **A warning never refuses an edit,** and neither does an error the program already had. A
  refactoring in a project with an unrelated error still works.
- **The check sees only what the compiler sees.** A break no error reports, such as a string a
  callback compares with a property's name, is a site the index must hold, so the rename rewrites it.
- **A moved file is analysed at its new path,** and its old path stays readable on disk during the
  check, so a use the edit missed that names the old place adds no error.
- **Not checked:** the editor's `willRenameFiles`. The client has already moved the file and cannot
  undo a drag, so a refusal would only drop the `require` edits. The diagnostics after the move name
  what broke. An override completion row checks for an error inside the text it writes: in a
  half-written document, the parser recovers somewhere else once the text is in, so a count before and
  after would move.

**Memory:** nothing at runtime, since only source is edited. Per refactoring, two analyses of each
entry that reads a changed file, on the caller's thread, dropped with the answer. A rename in a large
project is slower by those analyses. That is accepted, because a wrong edit is worse than a slow one.
If the editor's rename on the largest corpus takes over 5 seconds, the editor checks only the changed
files, `nvs code` keeps the full check, and this rule says so.

**Declined:**
- **Refusing an edit on any error after it.** A project that already has an error could then not be
  refactored at all, and the error the edit is meant to fix would refuse it.
- **Comparing errors by message.** A rename changes the names inside a message, so every error that
  names the renamed symbol would look new.
