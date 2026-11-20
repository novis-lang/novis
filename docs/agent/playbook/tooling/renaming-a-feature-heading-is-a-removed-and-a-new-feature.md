- **Renaming a reference chapter's `#` heading makes a new feature and removes the old one, and no
  tool moves anything for you.** `bun nv proofs --id <new id>` answers "no feature" until the
  heading in `chapter.md` is renamed and `bun nv render` has run. Then `git mv` the example, attack
  and bench paths, and rewrite every `// covers:` line and page `covers:`/`examples=` that names the
  old slug. `site --check changes` then owes a `removed` entry for the old id and a `new` one for the
  new id, and every text under the new paths owes a review. `--record-perf` per changed chapter's group
  clears the stale perf records `--gate` names.
  See `tools/nv/lib/changes.ts` `changesProblems`.
