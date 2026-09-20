# Handoff

## State

Goal `core-arr-4-4` is met. All fourteen members it names carry their feature proofs, and its own
stage-2 check — `dossier.py --verify --only <the 14>` — prints `nothing owed`, `0 failed`, `0 failed`.
The proofs landed under the earlier `core-arr-*` goals, whose fan-out covered the whole `Core\Arr`
group rather than each goal's own `--only` list, so this session found the item list already
satisfied on arrival.

What this session added is the half that check does not count: `about.md` for `shapeAs`, `sum` and
`unique` ran 180, 166 and 184 words, outside the 40-to-160 band `tools/dossier.py`'s `ABOUT_WORDS`
enforces once goal `the-description-is-owed` switches the description on. All three now sit inside it.

## Next group

**The next goal is the driver's to install** — this one is reached. The one piece of `Core\Arr` work
left over shares the file set and is listed first if the next group is picked by hand.

- [ ] **Five `Core\Arr` descriptions from the earlier goals are outside the word band** — `average`
      166, `countBy` 180, `max` 176, `min` 170, `product` 181. Same edit as this session's three:
      trim to the 40-to-160 band `docs/examples/README.md` § *The description* states.
      `docs/examples/core/Arr/average/about.md:1`

## Backlog

- The five over-band descriptions above are goal `the-description-is-owed`'s to sweep tree-wide; it
  is where `about_problem` becomes a red check (`tools/dossier.py:624`).
- 8 of 56 `Core\Arr` descriptions were outside the band, so the drift is roughly one in seven across
  the dossier program rather than local to this goal.
