- **`bun nv lsp-start` keeps the server's standard error to itself, and prints it only when the
  server fails.** A temporary `eprintln!` trace in the index printed nothing on a green run. Write
  a trace line to a file a variable names, append it under `.agent-tmp/`, and remove it before the
  commit. Rebuild first with `bun nv relink --build`, since `lsp-start` runs the release binary. See
  `tools/nv/cmd/lsp-start.ts` `measure`.
