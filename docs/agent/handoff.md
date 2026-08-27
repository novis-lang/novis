# Handoff

## State

**ADR 0107's surface is finished in the tree, and Stage 0a is one doc sweep
from done.** `inout` parses before the type in all three binding positions and
again at a call site, `&` in a by-reference position is `E0237`, the checker
enforces the call-site marker both ways (`E0713`/`E0714`), and as of this
session the compiler's own identifiers and prose say `inout` too. Both trees
are green at **676** conformance and 174 differential.

- **What is left of Stage 0a is item 47b alone**: every `&$x` still in `docs/`.
  `grep -rn '&\$' docs/` is 65 lines over 17 files, and most of them are
  correct as they stand — **ADR 0107's own 17 are all deliberate quotations of
  the retired spelling, and need no edit**, and ADR 0031's 16 are `use (&$y)`,
  which ADR 0107 § *Consequences* says is deliberately not amended. The real
  worklist is the rest: `docs/implementation-plan.md` (8),
  `docs/agent/playbook.md` (6), `docs/spec/00-overview.md`, `docs/plan/m4.md`,
  and one line each in `docs/adr/{divergences,README,0089,0063,0023,0015,0007,0006}.md`.
  Then `docs/agent/loop-goal.md` § *Stage 0a* is deleted.
- **The rename stops at three spellings and that is the decision, not an
  oversight.** `$a = &$b` (`E0701`), `[&$x]` (`E0483`) and `use (&$y)`
  (`E0224`) are refused because MWL has no reference at all — ADR 0107 replaced
  a by-reference *parameter*'s marker, and none of those three has one — so
  `ExprKind::Assign::by_ref` (`crates/mwl-syntax/src/ast.rs:736`),
  `ArrayItem::by_ref` (`:386`) and `Parser::report_closure_use_clause`
  (`crates/mwl-syntax/src/parser/expr.rs:1629`) keep the old word. Every other
  `by_ref` in `crates/` is gone; `grep -rn by_ref --include=*.rs crates/`
  returns exactly those three families and nothing else.
- **Six diagnostic constants moved and four did not**, on the same line:
  `E_INOUT_ARG_NOT_A_PLACE`, `E_INOUT_ARG_TYPE_NOT_EXACT`,
  `E_CLOSURE_INOUT_PARAM`, `E_FOREACH_INOUT_ELEMENT_TY`,
  `E_FOREACH_INOUT_SUBJECT` and `E_GENERATOR_INOUT_PARAM` name an `inout`
  binding; `E_BY_REFERENCE_MARKER_RETIRED`, `E_ASSIGN_BY_REFERENCE`,
  `E_ARRAY_ELEMENT_BY_REFERENCE` and `E_CLOSURE_USE_BY_REF_UNSUPPORTED` each
  name a `&`. Numbers are untouched; next free is still `E0238`/`E0715`.
- **Two doc paragraphs were corrections rather than renames**, and both are
  recorded where they live: `ResolvedCall::inout`
  (`crates/mwl-types/src/expr_table.rs:91`) was recorded because "a call site's
  own syntax says nothing about it", which § 2 falsified, and
  `check_foreach_inout`'s two help strings
  (`crates/mwl-types/src/expr/iteration.rs:314`, `:336`) still told a user to
  drop a `&` their source no longer contains — so the handoff's old claim that
  every user-facing string was already done was one file short.
- **ADR 0107 § *Verification* now names two conformance cases, not one**, with
  the reason in its own body: a `.mwlt` has one verdict, so the accepted shapes
  and the three refusals cannot share a file.

## Next group

**Stage 0a item 47b, then Stage 0b's first hole. One session: 47b is a doc
sweep with no build in it, so it leaves nearly the whole budget for the hole
behind it.** The file set: `docs/`, then whatever the hole names.

- [ ] **Item 47b — the doc sweep.** Every `&$x` in `docs/` that names MWL's own
      by-reference parameter, from `grep -rn '&\$' docs/`. Skip ADR 0107
      (`docs/adr/0107-…md`, 17 sites, all deliberate) and ADR 0031
      (`docs/adr/0031-…md`, 16 sites, all `use (&$y)`). The pattern that worked
      on `crates/` and `tests/` is: mask the refused spellings (`use (&$y)`,
      `` `[&$x]` ``, `$a = &$b`, `int &$x`) first, then substitute `` `&$v` ``,
      `` `&$x` ``, `` `&$n` `` and `as &$v)`, then fix `a `inout` → `an `inout`.
      Finish by deleting `docs/agent/loop-goal.md` § *Stage 0a*.
- [ ] **Stage 0b's first item**, from `docs/agent/loop-goal.md` — read it after
      47b lands, since deleting § *Stage 0a* is what makes it the head.

## Backlog

- A variadic `inout` parameter is accepted and writes nothing back — `mwl-types`
  gap, playbook § *Writing MWL itself* has the repro.
- `tests/{conformance,codegen}` still hold files named `by_reference.rs`; the
  name describes the semantics, which are unchanged, so this is cosmetic only.
- `at_intersection_amp` survives ADR 0107 § 3's delete request — ADR 0107 § 3's
  body and `crates/mwl-syntax/src/parser/ty.rs:147` are the pair of homes.
- ADR 0089's tier-D rule for the `inout` conversion is M11, named in ADR 0107
  § *Verification*.
