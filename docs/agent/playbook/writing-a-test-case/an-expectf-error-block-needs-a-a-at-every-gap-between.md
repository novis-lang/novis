- **An `--EXPECTF-ERROR--` block needs a `%A` at every gap between literal lines, including after
  the last one.** A block ending `%A` / `error: aborting due to N errors` matches because the `%A`
  swallows the caret excerpt and the blank line, but a pinned `   = help: ...` line before the abort
  leaves nothing to absorb the blank line between them, and the diff reads as identical text
  refused. Pin a help line when it is the claim (`no-client-member-accepts-an-unbounded-wait.nvst`)
  and put a `%A` on the line after it. [until: reviewed 2026-09-06]
