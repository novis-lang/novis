- **Another agent — another session of the loop, or the user — may be editing this repo right now,
  and `ls` will not tell you.** `tools/brief.py` prints a loud banner when `.loop/running` exists;
  if it does, stop and tell the user rather than editing alongside it. Otherwise claim a new
  numbered file with `git status --short <dir>` immediately before creating it, stage your own paths
  explicitly rather than `git commit -a`, check `git show --stat` after committing, and re-read a
  shared doc immediately before rewriting it. [until: gone tools/nv/cmd/orient.ts:A LOOP DRIVER HOLDS THIS TREE]
