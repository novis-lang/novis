- **Git runs a hook with `LC_CTYPE=C.UTF-8`, and gawk's `[^a-z]` does not match an emoji under it.**
  A pattern anchored as `^[^a-z]*(generated|created)…` passes every test run from the Bash tool,
  which sets no locale, and silently fails to match `🤖 Generated with [...]` when git itself runs
  it. Run a hook's test the way *git* invokes it, and locate a phrase with `match()` and ask a
  pure-ASCII question about the text before it rather than stepping a character class across
  multibyte input. [until: reviewed 2026-09-06]
