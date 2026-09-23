- **`echo a, b, c` writes each argument as it reaches it, so a throw in a later argument leaves the
  earlier text on stdout.** The arguments are evaluated and written one at a time, so a `try` block
  whose `echo` opens with a `"BAD="` label prints that label and *then* throws, and the expected
  output no longer matches. Put the call that can throw in its own statement and echo the variable.
  [until: reviewed 2026-09-19]
