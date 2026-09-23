- **A commit message carries no attribution trailer, and two gates enforce it.** `tools/session.py`
  strips `Co-Authored-By`, `Signed-off-by`, `Generated-with` and the prose `🤖 Generated with …` line
  out of any message it commits; `tools/git-hooks/commit-msg` rejects one arriving by `-m`, `-F`, an
  editor or a merge. The hook is off until `git config core.hooksPath tools/git-hooks` has been run
  once per clone — `verify.py` says so when it is not set — and conventions.md § *A commit message*
  is the rule's home. [until: reviewed 2026-09-06]
