# Handoff

## State

Goal `core-db-row` is complete. All fourteen members of `Core\Db\Row` carry the proofs
`rule:testing/feature-proofs` names, and `python tools/dossier.py --gate --group 'Core\Db\Row'`
reports nothing owed. `Core\Db\Row::instant` is excused from its three program proofs, with a
reason on each entry in `tools/data/dossier-policy.toml`; every other member has all five.

`toArray` landed this session: `about.md`, three examples, one attack, one bench, a `.nvst` case
and a Rust case. The bench measures 0 allocations and 0 calls per round, which is the member's
claim — it answers the row's own array under a second reference rather than a copy.

The three goal-end gates are green: `python tools/verify.py --doc`, `python tools/owners.py
--closes core-db-row` and `python tools/playbook.py --closes core-db-row`, the last two reporting
that this goal owns no module-doc gap and no carried-gaps row. Nothing is blocked.

## Next group

The next goal in the chain is `core-db-rows-and-1-more`, and a goal switch overwrites this file
with its own seed. If the switch has not happened, these are its first three slices, in file
order. One slice is one feature with all of `rule:testing/feature-proofs`'s proofs. One file set:
`crates/nvs-stdlib/src/db/registry.rs` (the reference card each one is read from), `nvs.toml` (one
`[[app]]` grant per proof program) and the four trees under `core/Db-Rows/<member>/`.

- [ ] **`Core\Db\Rows::all`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1212`
- [ ] **`Core\Db\Rows::column`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1245`
- [ ] **`Core\Db\Rows::columns`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1268`

`python tools/dossier.py --id '<feature>'` prints the path each proof belongs at, and
`--group 'Core\Db\Rows'` is the board.

## Backlog

- The landed proof comments of the whole `Core\Db\Row` group are goal `plain-comments`'s to sweep;
  the ones written this session already follow AGENTS.md § *Text an end user reads* —
  `docs/agent/goals/174-plain-comments.md`.
- `Core\Db\Row::instant` keeps its three program proofs excused rather than owed —
  `tools/data/dossier-policy.toml`.
