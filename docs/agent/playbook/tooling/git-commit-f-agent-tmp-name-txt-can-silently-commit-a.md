- **`git commit -F .agent-tmp/<name>.txt` can silently commit a *previous* session's message.**
  `.agent-tmp/` is not cleaned between sessions and every session reaches for the same obvious file
  names, so a `-F` naming a file you did not write this session succeeds with someone else's subject
  line, and `git commit -F msg.txt 2>/dev/null || true` hides even the missing-file case. Write the
  message file in the same call sequence you commit it in, under a name new to this session, and
  never mask a `git commit`'s exit status; `session.py --wrap` is immune.
  [until: gone AGENTS.md:.agent-tmp/]
