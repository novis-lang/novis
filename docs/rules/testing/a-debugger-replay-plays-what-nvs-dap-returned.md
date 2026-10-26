A debugger replay on a website page shows a debug session over one snippet: where it stopped, the call
stack, and the variables with their values. A hand-written `.dap` beside the snippet lists the
breakpoints and the steps: `continue`, `next`, `in` and `out`. `bun nv site --bless` runs `nvs dap`
over the snippet, sends those requests, and writes each stop's answers to `.dap.out`: the line and the
reason, the frames by name and line, and each variable's name, type and value.
`bun nv site --check snippets` fails when `nvs dap` now returns something else.
`tools/nv/cmd/site.ts` § *A snippet* is the home of both formats.

A number that only names an object for the length of one session, such as a variables reference, is
left out of the recording, so two runs of the same session record the same file.

The panel has the replay's look and its rules for play, pause and focus
(`rule:testing/a-replay-plays-what-the-language-server-returned`). The editor window shows the
breakpoints and the stopped line, and a panel under the stopped line shows the call stack and the
variables. A debugger replay is a `<Replay>` of a snippet with a `.dap` and no `.replay`. A drawn debugger
would show values the program never had, and recording `nvs dap`'s answers is what keeps the page true.

- **Memory:** nothing in the language or the binary. `--bless` and `--check snippets` start `nvs dap`
  once per replay.

**Declined:**
- **A screenshot of VS Code's debugger.** It needs a real editor in CI, and the picture would show
  VS Code's look, which changes with every release.
