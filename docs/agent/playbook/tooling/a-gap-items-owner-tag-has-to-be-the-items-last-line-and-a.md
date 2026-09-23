- **A gap item's `— owner:` tag has to be the item's *last* line, and a paragraph continuing that
  item under it reads as untagged.** `tools/owners.py` takes the tag off the last line, so a gap
  re-owned by writing the tag under the sentence that named the new owner failed
  `--untagged-is-an-error` on the goal-end sweep, which the floor's one-session-in-ten schedule had
  not run since. Write the tag under the item's final paragraph, and run `python tools/owners.py
  --check --untagged-is-an-error` in the same session as any edit to a `# Known gaps` block.
  [until: reviewed 2026-09-15]
