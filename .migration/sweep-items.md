# Sweep items — the C8 hand-fix list

One bullet per item, `file:line — what is wrong there`, grouped by the unit whose transaction found
it. `--sweep` (unit C8) works this list down to nothing; a bullet leaves when its site is fixed.
`.migration/assemble.py` appends a unit's `## sweep:` lines here at assembly, so a hand-fix is
recorded once, in one file, instead of in the unit's commit body. The items below B16 were lifted
from the commit bodies of c7a9afca6 through 44ba60ce9, re-anchored to the lines they sit on today.

## Worked down at C8 — 2026-09-06

The 191 items this file held are closed; `git log -p -- .migration/sweep-items.md` has each one's
text. How they closed, by kind:

| Kind | Items | What closed it |
|---|---|---|
| Site fix | 117 | An edit at the site: a citation re-pointed to the rule that holds it, a stale claim corrected, a gap recorded in the owning module doc. 171 blocks over 99 files, applied as one `splice.py` patch in C8's commit |
| Record defect | 50 | Nothing. A frozen record that states something false is history; the rule fragment its `changes:` block names states the truth, and every one was checked to exist |
| Already fixed | 9 | The site read correctly by the time the sweep reached it |
| Owned elsewhere | 11 | A code gap or an open design question already recorded in the owning module doc's *Known gaps*, a rule's `status`, or `docs/agent/carried-gaps.md` |
| Moot | 3 | An edit to `.migration/topic-map.json` that changes nothing a citation resolves to; the file goes with C9 |
| Routed | 1 | A missing conformance fixture for `rule:routing/a-shared-name-is-one-endpoint-everywhere`, recorded nowhere: now a `docs/agent/carried-gaps.md` § *Unowned* bullet with the fixture that retires it |

Nothing is left for a session to do here.
