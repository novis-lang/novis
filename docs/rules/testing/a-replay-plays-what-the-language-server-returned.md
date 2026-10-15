A replay on a website page shows an editor session that turns one snippet's starting text into the
snippet, and every answer it shows from the language server is one `nvs lsp` returned for those steps.
A hand-written `.replay` beside the snippet lists the steps, `bun nv site --bless` writes the recording
beside it as `.replay.out`, and `bun nv site --check snippets` fails when the server now returns
something else, or when the steps do not end on the snippet's own text. `tools/nv/cmd/site.ts`
§ *A snippet* is the home of both formats.

A replay has three kinds of step: typing text one character at a time, accepting the selected
completion row, and showing the diagnostic under the cursor. A completion list is recorded wherever an
editor asks the server for one: at a trigger character
(`rule:ide/completion-is-asked-where-a-spelling-ends`) and at the first letter of a word. Each list,
each diagnostic and each hover is recorded. Narrowing a list as more letters are typed is the editor's
own work, so the page imitates it: case-insensitive matching of the letters in order, the best match
first, the matched letters highlighted. The component's head comment says which parts are recorded
and which are imitated, and a page never presents the replay as VS Code's exact behaviour.

A replay is labelled as a demo: a "Recording" badge in the frame's title bar, and a caption above it
that says it was recorded from the editor extension, that the reader cannot type in it, and that links
to the editor install page. Its play, pause, previous and next buttons sit outside the frame and work
from the keyboard. The code never takes keyboard focus, and clicking it does nothing. It starts when at
least half of it is in view, pauses when it leaves, plays once and stops on the last frame with a
"Play again" button, and only one replay on a page plays at a time. Under `prefers-reduced-motion` it
does not start: it shows the last frame with its list open. A collapsed section under it gives the same
steps as text, for screen readers and search. Every caption and label follows AGENTS.md § *Text an end
user reads*.

A recording drawn by hand would show completions the server never offers, and nothing would notice
when the server changes. Recording the server's answers and checking them on every `--check` keeps the
page honest the same way a `.out` keeps a snippet's output honest. The snippet is the replay's last
frame, so the code a page shows and the code the replay ends on cannot disagree. The demo label and the
focus rules keep a reader from mistaking the replay for an editor they can type in.

**Memory:** nothing in the language or the binary. A page pays for one small script and the
recording of each replay it shows. `--bless` and `--check snippets` start `nvs lsp` once more per
replay.

**Declined:**
- **An editor library on the page.** The frame, the list and the squiggle are plain HTML and CSS, so
  the page weight is the recording and one script.
- **Recording the narrowing of a list.** The server returns one list per request, and the narrowing
  after it is the editor's, so recording it would claim the server did work it does not do.
- **Hover cards and quick fixes as steps before a page needs them.** Each kind of step is another
  format to check, and no page shows one yet.
