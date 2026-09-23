- **A rule cited beside a claim does not always state that claim.**
  `rule:core-classes/queue-storage-is-a-table` cited `rule:core-classes/schema-plan` for keeping a
  filtered index out of v1, but that exclusion is `rule:core-classes/schema-vocabulary-is-closed`'s, so
  a record that builds its `changes.modifies` list from the citation edits a fragment that never made
  the claim. Grep the claim itself across `docs/rules/*/*.md` before naming a rule in a `changes:`
  block, and fix the misdirected citation in the same commit. [until: reviewed 2026-09-15]
