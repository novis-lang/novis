A replay on a website page shows an editor session that turns one snippet's starting text into the
snippet, and every answer it shows from the language server is one `nvs lsp` returned for those steps.
A hand-written `.replay` beside the snippet lists the steps, `bun nv site --bless` writes the recording
beside it as `.replay.out`, and `bun nv site --check snippets` fails when the server now returns
something else, or when the steps do not end on the snippet's own text. `tools/nv/cmd/site.ts`
§ *A snippet* is the home of both formats.

A replay has three kinds of step for completion: typing text one character at a time, accepting the
selected completion row, and showing the diagnostic under the cursor. A rename, a code action, the code
lenses, the call hierarchy and a hover card are five more
(`rule:testing/a-replay-renames-applies-an-action-and-shows-a-lens`). A completion list is recorded wherever an
editor asks the server for one: at a trigger character
(`rule:ide/completion-is-asked-where-a-spelling-ends`) and at the first letter of a word. Each list,
each diagnostic and each hover is recorded. Narrowing a list as more letters are typed is the editor's
own work, so the page imitates it: case-insensitive matching of the letters in order, the best match
first, the matched letters highlighted. The component's head comment says which parts are recorded
and which are imitated, and a page never presents the replay as VS Code's exact behaviour.

A replay is one panel with a look of its own, tinted with the site's accent, that holds everything
in order: a header "How it looks in your editor", a notice that says it was recorded from the editor
extension, that the reader cannot type in it, and that links to the editor page under Tools, then the
editor window, then one button, then a collapsed section that gives the same steps as text, for
screen readers and search. The window has a title bar with the snippet's file name, and a bar along
its bottom that shows how far the replay has played. The panel's root carries Starlight's
`not-content` class, so the page's text styles add no space between the code's lines, and a list
opens on the line under the cursor, at the column where the word it completes starts. The code's font
never goes below 12px. A line wider than the window does not wrap: the code scrolls sideways, as an
editor does, so after each frame the cursor and the open list or error card are in view, and a frame
already in view does not move it. The line numbers stay in place while it scrolls.

A replay never starts by itself. Before it plays, it shows the last frame with a list open, or else
the last frame with an error card or another step's panel. The
button plays it from the first frame, pauses it while it plays, and plays it again from the first
frame once it has ended; its icon and its label say which it does next. Before the first play and
while paused, the label says what the recording shows: "Preview the error message" when the
recording has a diagnostic, and "Preview the autocompletion" when it has only completion lists. A
recording with one of the five other steps names the first of them it has, in the order rename, code
action, call hierarchy, code lenses and hover card: "Preview the rename", for one. The
button works from the keyboard. The code never takes keyboard focus, and clicking it does nothing.
A replay pauses when its window leaves the view, plays once and stops on the last frame, and only
one replay on a page plays at a time. Under `prefers-reduced-motion` the cursor does not blink and the
progress bar does not slide. Every caption and label follows AGENTS.md § *Text an end user reads*.

A recording drawn by hand would show completions the server never offers, and nothing would notice
when the server changes. Recording the server's answers and checking them on every `--check` keeps the
page honest the same way a `.out` keeps a snippet's output honest. The snippet is the replay's last
frame, so the code a page shows and the code the replay ends on cannot disagree, and a page needs no
`<Snippet>` of the same file beside it. The header, the notice, the panel's own look and the focus
rules keep a reader from mistaking the replay for an editor they can type in, or for code to copy. A
replay that plays only when asked does not move while somebody reads the text around it.

**Memory:** nothing in the language or the binary. A page pays for one small script and the
recording of each replay it shows. `--bless` and `--check snippets` start `nvs lsp` once more per
replay.

**Declined:**
- **An editor library on the page.** The frame, the list and the squiggle are plain HTML and CSS, so
  the page weight is the recording and one script.
- **Recording the narrowing of a list.** The server returns one list per request, and the narrowing
  after it is the editor's, so recording it would claim the server did work it does not do.
- **A hover card that is drawn by the page and not recorded.** The card is the server's documentation,
  so it is recorded and checked as a `card` step, the same as a list
  (`rule:testing/a-replay-renames-applies-an-action-and-shows-a-lens`).
- **Starting when the replay scrolls into view.** Motion the reader did not ask for pulls the eye
  from the text they are reading, and the poster frame already shows the list.
- **A font that shrinks until every line fits.** On a phone the code became too small to read, and
  a long line still did not fit.
- **Buttons that step one frame back or forward.** The steps as text give the same detail, and one
  button is simpler to read.
