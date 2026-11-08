- **A bare `git commit` takes everything in the index, and another session in the same tree may
  have staged its own work there.** A session that ran `git add <its paths>` and then
  `git commit -F <file>` committed three `.gitignore` deletions it never touched. Commit with
  `git commit -F <file> -- <the slice's paths>`, which leaves the rest of the index staged. If it
  happened, `git reset --soft HEAD~1` and commit again with the paths. See `tools/git-hooks/`.
