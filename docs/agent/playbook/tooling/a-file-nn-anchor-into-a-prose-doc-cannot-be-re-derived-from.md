- **A `file:NN` anchor into a prose doc cannot be re-derived from a symbol, and a doc that indexes
  work loses lines every time an entry goes green.** A goal's stage 0 named an index row as
  `<doc>:62`'s owner cell and the anchor landed in the middle of the next section's prose, because
  rows had been struck from the table since the goal was written and three of the rows still there
  were plausible candidates. Date the anchor rather than guessing which one: `git log
  --diff-filter=A --format=%H -1 -- <the file that wrote the anchor>` names the commit it was
  written at, and `git show <sha>:<doc> | sed -n '<NN-14>,<NN+2>p'` prints the line it meant.
  [until: reviewed 2026-09-15]
