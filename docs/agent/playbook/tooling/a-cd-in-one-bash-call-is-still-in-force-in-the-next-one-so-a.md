- **A `cd` in one Bash call is still in force in the next one, so a later `git status` or `grep` answers
  about the wrong directory instead of failing.** The tool keeps its working directory between calls
  even though shell state does not, so `cd website && npm run sync:rules` leaves every following call
  rooted in `website/`, where `git status --porcelain -- website` reports zero changes and a `grep -r`
  finds nothing. Prefix the next call with `cd /d/mwl &&`, or run the one-off as
  `cd <dir> && <cmd>` knowing the move sticks. [until: gone AGENTS.md:One shell call runs one command]
