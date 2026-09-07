# Handoff

## State

**Goal 13 stage 3 is whole.** All four of ADR 0124's PHP 8.6 refusals now land in the parser:
`let`/`is` as a name is `E0247`, a constructor's returned value `E0248`, a `readonly` default
`E0249`, and this session's two — `E0250` for a `return` inside a `finally` and `E0251` for a
`break`/`continue` whose target lies outside one. Three `.nvst` cases under
`tests/conformance/reject/php86/` pin the refusals a user meets; the two reserved spellings stay
pinned by `-p nvs-syntax` unit tests, which is what the stage's two checks run.

**How the `finally` state is kept**, since it is the shape the next refusal of its kind copies:
`Parser::in_finally` is parked by `in_callable_body` exactly as `in_constructor` is, and
`Parser::finally_breakables` counts the loops and `switch`es opened *since* the innermost `finally`
began — every loop and `switch` raises it around its body, so a level above it names a target
outside the block and a loop written wholly inside keeps both spellings.

**Stage 4 is what is left, and only one of its two checks is red.** `python tools/reference.py
--check` passes (`docs/novis.md` is current, 282 of 282 examples hold). The dossier gate fails
earlier than the four proofs: `lang:expressions/the-pipeline-operator` is not on the roster at all,
because the roster is every `#` heading in `docs/reference/lang/*.md`
(`rule:testing/roster-is-derived`, `tools/dossier.py:475`) and that chapter has no pipeline heading
yet. Nothing is blocked.

## Next group

**The reference chapter and the pipeline's four proofs, one file set:**
`docs/reference/lang/30-expressions.md`, `docs/novis.md` (generated, never hand-edited),
`docs/examples/`, `tests/conformance/lang/`.

- [ ] **The pipeline operator gets its reference heading**, which is the same act as putting it on
      the dossier roster — `rule:testing/roster-is-derived`. Write `# The pipeline operator` beside
      the call forms at `docs/reference/lang/30-expressions.md:443`, and give the precedence table at
      `docs/reference/lang/30-expressions.md:8` its `|>` row
      (`rule:expressions/pipeline-precedence` is the ordering it states). Then `python
      tools/reference.py --check` must still say `docs/novis.md is current`; regenerate with `python
      tools/reference.py` if it does not.
- [ ] **Answer what the gate then names as owed** — `python tools/dossier.py --only
      lang:expressions/the-pipeline-operator --gate` at `docs/agent/loop-goal.toml:5098` is the
      stage's own check, and `rule:testing/four-proofs` is what it counts: a test from both sides,
      three examples, a measured figure, an attack. `examples/pipeline.nvs` and the cases under
      `tests/conformance/lang/` already exist and are attributed by path and by marker
      (`rule:testing/proof-attribution`), so read the gate's list before writing anything new.

## Backlog

- The seven rules this goal implements are still `status: designed` in `docs/rules/*.json` — the
  three `|>` rules and the four PHP 8.6 refusals. Flipping them, with `guardedBy` naming
  `tests/conformance/reject/php86/` and the pipeline cases, is a docs-only slice
  (`docs/agent/conventions.md` § *A rule fragment*, then `python tools/rules.py --render`).
- `verify.py` reports this clone has no hooks (`git config core.hooksPath tools/git-hooks`); left
  alone mid-run on purpose, since `session.py` strips trailers anyway.
- `.automode_decisions.jsonl` at the repo root is a harness artefact the driver committed once as
  `117615584`; it is not Novis's and nothing reads it.
