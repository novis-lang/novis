A replay (`rule:testing/a-replay-plays-what-the-language-server-returned`) has five more kinds of step.
`rename NAME` renames the name under the cursor, and the frame shows the edit in the open file and how
many other files it changed. `action TITLE` opens the code actions at the cursor, shows their titles,
and applies the one with that title. `lens` shows the code lenses of the open file above their lines.
`calls` shows the call hierarchy at the cursor, its incoming and its outgoing calls
(`rule:ide/the-request-set-is-closed`). `card` shows the hover at the cursor. Each answer is what
`nvs lsp` returned, recorded into the `.replay.out` in a block of its own, and
`bun nv site --check snippets` fails when the server now returns something else.

The steps still end on the snippet's own text. A rename or an action changes the text, so the snippet
is the text after the edit, and the start text is the text before it.

A rename, an action, a lens, a call hierarchy and a hover card are what the editor's own features look
like, and the Tools area shows each of them. Recording them keeps a page from showing an edit or an
answer the server does not give.

- **Memory:** nothing in the language or the binary. A page pays for the recording of each replay it
  shows.

**Declined:**
- **Showing the other files a rename changed.** The frame is one editor window, and the count says
  what the reader needs.
