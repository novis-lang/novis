- **`/tmp` is not the same directory to Bash and to Python here.** A file written by `>` in the Bash
  tool is invisible to a `python -` heredoc in the same call, which resolves `/tmp` to `%TEMP%`.
  Stage a scratch file under `.agent-tmp/` — both halves agree on a repo-relative path.
  [until: reviewed 2026-09-06]
