# Handoff

## State

**Goal 67 — `is` is the one type test, and `instanceof` is gone — has just started; nothing of it has
landed yet.** Goal `decided-closures`'s whole list is this goal's Stage 1 floor.

Everything is settled before the first session, and the first session writes the settlement down
before any code moves: the decision record and the rulebook are stage 2, and the goal's prose spells
out what the record's `Context`, `Decision`, `Alternatives rejected` and `Revisiting` hold, which rule
it creates, which rule it deletes and which six it modifies. The one thing no session re-decides is
the standing decision at the top of § *Standing decisions*: Novis diverges from PHP on `is` and
`instanceof` for good, and no session compares either keyword with PHP again — the RFC was read once,
its reading is frozen in the record, and it is not read again.

Goal `core-class-tests`'s descriptor work is on `main` (stage 0's last bullet), so `is Core\Time\Date`
already has a path to run on and nothing here builds a second one.

## Next group

**Stage 2: the record and the rulebook** — one file set: `docs/decisions/` (one new record at the
next free number), `docs/rules/php-migration.json`, `docs/rules/types.json`,
`docs/rules/php-migration/one-type-test.md` (new), and the six fragments the record modifies:
`docs/rules/types/type-test.md`, `docs/rules/types/narrowing.md`,
`docs/rules/types/class-reference-sites.md`, `docs/rules/php-migration/let-and-is-are-reserved.md`,
`docs/rules/php-migration/a-declared-type-answers-before-the-program-runs.md`,
`docs/rules/php-migration/every-divergence-is-deliberate-and-listed.md`.

- [ ] **The record** — written from `conventions.md` § *A decision record*; `changes.creates` is
      `php-migration/one-type-test`, `changes.modifies` is the six above; § *Context* freezes the one
      reading of PHP's RFC and the 8.6 deprecation; § *Revisiting* names one Novis trigger and no PHP
      one.
- [ ] **The new fragment and its JSON entry**, `status: designed`, placed beside
      `let-and-is-are-reserved` in `php-migration.json`'s `rules` array, its `divergesFromPhp` the one
      sentence `divergences.md` prints.
- [ ] **`is-takes-pattern-matchings-type-patterns` deleted** — fragment and entry, every `seeAlso`
      that names it swept, and if `changes:` has no key for a removal, one sentence in § *Decision*.
- [ ] **The six fragments rewritten** to the language this goal ships, each `because` gaining the
      record's number, then `python tools/rules.py --render`.
- [ ] Stage 2's five checks green: `rules.py --check`, `rules.py --render --check`,
      `records.py --check`, and the two `git grep`s.

## Backlog

- Stage 3 (the front end: `nvs-syntax`, `nvs-types`, `nvs-diagnostics`) and stage 4 (the lowering,
  the codegen, the runtime rename), each its own file set; stage 4 cannot start before stage 3's
  `ExprKind::TypeTest` carries the value arm.
- Stage 5 (`nvs-stdlib`'s `Core\Ast` roster, the LSP, the formatter) and stage 6 (the test sweep and
  the renames, each rename patched everywhere it is named in the same slice).
- Stage 7 (the prose sweep, the rule flipped to `shipped`, the absence gate).
- When this goal's last check goes green the driver takes goal `gap-zero`.
