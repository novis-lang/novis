- **A `splice.py` block that deletes a whole line leaves the line behind as a blank one.** The empty
  NEW half replaces only what OLD matched, and the newline after the last matched character is not
  part of that match, so striking a row from `carried-gaps.md` § *Owned* left an empty line mid-table,
  which splits one rendered table into two. Put the *following* line inside both halves of the block —
  anchor on the row plus the line after it, and write that following line back alone.
  [until: reviewed 2026-09-10]
