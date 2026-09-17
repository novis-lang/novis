# Handoff

## State

**Goal `decided-closures` is reached, and its floor is green.** Session 0003's DONE claim fell to one
check — `python tools/plan.py --check`, on M7's `Carried by` cell, which had gone stale when the last
goal carrying M7 walked. `python tools/plan.py --sync` rewrote it to `done`; `--check` is exit 0 and
`--past` reports 11 of 11 past milestones complete.

The register is empty from every side: `python tools/owners.py --closes decided-closures` and `python
tools/playbook.py --closes decided-closures` both answer "owns no …". `python tools/verify.py` is
green and `python tools/verify.py --doc` resolves every link. Nothing else in the goal's acceptance
list was red in that sweep — 670 of 671 checks passed.

## Next group

**Goal `one-type-test`, stage 2: the record and the rulebook** — the driver installs
`docs/agent/goals/67-one-type-test.handoff.md` over this file at the switch, and that file is
authoritative for the stage; these are its first three items, unchanged.

- [ ] **The record** — one new record at the next free number, to the shape
      `docs/agent/goals/67-one-type-test.md:67` spells out: `changes.creates` is
      `php-migration/one-type-test`, `changes.modifies` is the six fragments the goal names, §
      *Context* freezes the one reading of PHP's RFC, § *Revisiting* names one Novis trigger and no
      PHP one.
- [ ] **The new fragment and its JSON entry** — `docs/rules/php-migration/one-type-test.md` at
      `status: designed`, placed beside the `let-and-is-are-reserved` entry at
      `docs/rules/php-migration.json:176`, its `divergesFromPhp` the one sentence `divergences.md`
      prints.
- [ ] **The six fragments rewritten** to the language that goal ships —
      `docs/rules/types/type-test.md:1`, `docs/rules/types/narrowing.md:1`,
      `docs/rules/types/class-reference-sites.md:1` and the three under `docs/rules/php-migration/`
      the goal names — each `because` gaining the record's number, then `python tools/rules.py
      --render`.

## Backlog

- `crates/nvs-cli/src/bundle.rs`'s remaining known gap: § 6's `.nvsx` entries are not embedded, owned
  by M9 and blocked on Tier 1 extensions loading at all.
- A bundle grows by every `.nvs` file under a declared root, stated in `bundle::build`'s doc comment;
  nothing measures it and no cost guard names a bundle's size.
- Nothing derives a milestone's `Carried by` cell at wrap time, so it can only go stale between a goal
  being reached and the next `plan.py --check` (`tools/plan.py --sync` is the repair).
- Goal `gap-zero` follows `one-type-test`; its gate declares the tree clean of owed work, so a
  construct deleted after it would reopen sites it passed over (`docs/agent/goals/68-gap-zero.md`).
