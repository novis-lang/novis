- **Deleting a rule strands `rule:` citations in files nothing may rewrite — a frozen record's body,
  a retired goal's `.md` — and `bun nv records --check` also refuses the dead id in the creating record's
  `changes.creates`.** `bun nv rules --check` scans `docs/**/*.md` whole, so "frozen history is never
  edited" and "no citation dangles" cannot both hold. Drop the `rule:` prefix in the frozen prose —
  the id still reads as a name — and delete it from `changes.creates`, which is the machine-read
  relation, not the reasoning. [until: gone tools/nv/cmd/records.ts:creates]
