A terminal block shows commands run in a fixture directory and what `nvs` printed for each, with its
exit code. A hand-written `.term` beside the fixture under `website/snippets/` lists the commands, one
per line. `bun nv site --bless` runs them in a copy of the fixture and writes the output beside it as
`.term.out`, and `bun nv site --check snippets` fails when the binary now prints something else.
The fixture is the directory beside the `.term` with its name, and a program in it is not a snippet.
`tools/nv/cmd/site.ts` § *A terminal block* is the home of both formats.

A value that differs from run to run is masked in the comparison by a fixed pattern on both sides: a
duration, a date, a trace id, a temporary path and a port. The page shows the masked value as the
binary printed it on the run that was blessed, so the reader sees real output. The two paths that name
the machine are the exception: the copy's own path is recorded as `~/project` and the temporary
directory as `/tmp`, so no page shows the blessing machine's user name or drive. Only the commands are
copied by the block's copy button.

A terminal block is to a command what a snippet's `.out` is to a program
(`rule:testing/website-pages-are-stamped`): output on a page that the binary is known to print.

- **Memory:** nothing in the language or the binary. `--check snippets` runs each block's commands once.

**Declined:**
- **A typed screen recording of a terminal.** It would be a video to regenerate, and its text could not
  be compared or searched.
