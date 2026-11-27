- **Read a CI job's failures with `gh run view --job <id> --log-failed`, piped through `tr -cd '[:print:]\n\t'`.**
  `gh api .../jobs/<id>/logs` refuses to print a log with terminal escape sequences, and a `>` into the
  tree is refused by the guard, so a sweep's logs cost several calls before either works. Grep the
  piped text for `FAIL|panicked at` and read the lines around each; see `tools/nv/cmd/affected.ts`.
