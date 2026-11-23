- **A sweep that adds a sentence to a step's comment breaks its two-line bound.** A comment above
  a step in an example or an attack may have two lines, and most already have two, so a note a
  writing subagent appends makes it three and `bun nv proofs --comments` fails. Tell the subagent
  to leave a full step comment alone and change only code, and run that check over the changed
  files before committing. See `tools/nv/proofs/collect.ts`.
  `until: gone tools/nv/proofs/collect.ts:a comment above a step`
