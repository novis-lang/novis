- **A commit message carries no attribution trailer, and the hook enforces it.**
  `tools/git-hooks/commit-msg` rejects `Co-Authored-By`, `Signed-off-by`, `Generated-with` and the
  prose `🤖 Generated with …` line arriving by `-m`, `-F`, an editor or a merge. The hook is off until `git config core.hooksPath
  tools/git-hooks` has been run once per clone — `nv verify` says so when it is not set — and
  conventions.md § *A commit message* is the rule's home.
  `until: gone tools/git-hooks/commit-msg:git config core.hooksPath tools/git-hooks`
