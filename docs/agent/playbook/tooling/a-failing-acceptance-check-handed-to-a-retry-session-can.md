- **A failing acceptance check handed to a retry session can already be green, because its cause was
  a commit that landed inside the sweep's own window from a writer that is not the loop.** Here a
  by-hand rename left the handoff citing a rule id that no longer existed, and the ledger keeps only
  stderr's first line — the `N finding(s):` header, never the finding — while `git status --short`
  is clean by the time the retry session looks. Re-run the check's own argv before reading `loop.py`
  or the goal file, and when it passes compare `git log --date=iso` against the sweep's window in
  `.loop/logs/*-console.log`. [until: reviewed 2026-09-19]
