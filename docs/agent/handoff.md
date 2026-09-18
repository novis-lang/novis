# Handoff

## State

**Goal `one-type-test`'s four stage-7 checks are green.** The absence gate was red on the playbook
bullet that describes it: `docs/agent/playbook.md` is on the gate's own path list, so the bullet
naming the refused word was the last spelling of it left in the tree.

**`rule:types/type-test` is `shipped`** — `docs/rules/types.json:327`. Every row of its *What may
appear on the right* table was walked with its own probe program and answers: the eight scalar tags,
`object`/a class/an interface, `array` and `array<int>`, a shape, `iterable`/`callable`/a callable
signature, the three literal forms, a class constant and an enum case, a bare enum, `mixed`, a union
and an intersection, and the value arm through both a local and `$this->cls`. The two refusals
(`E0813`, `E0811`) and the float-literal one (`E0120`) report as the rule states. `guardedBy` now
lists the six `tests/conformance/lang/an-is-test-*` cases that pin the table row by row.

**Two cells named spellings Novis does not have, and were corrected rather than built.**
`$x is self::Wild`: `self::Name` in type position parses nowhere — `crates/nvs-syntax/src/parser/ty.rs:461`
never looks past the keyword for a `::`, so `self::WILD $n = 7;` fails the same way — and
`rule:types/class-scoped-alias` § *Two spellings and no third* refuses the spelling. `$x is
Countable&Traversable`: neither interface exists (`rule:iteration/no-magic-collection-interfaces`).

**The codegen class-test guard is `CodegenError::Internal` now**, at
`crates/nvs-codegen/src/emit.rs:2684`. It stays an error return rather than an assertion — a unit
built wrong is reported like every other backend failure — but it is no longer an `Unsupported`,
whose sites are `tools/holes.py`'s inventory of shapes the language still refuses.

## Next group

**Stage 7 follow-on: what `self::` does in type position, one file set — the type grammar's atom
list and the two rules that state it.**

- [ ] **Decide and land what `self::CONST` and `self::Case` do in type position.** Today
      `crates/nvs-syntax/src/parser/ty.rs:461` bumps `self`/`static`/`parent` and never looks at a
      following `::`, so `$x is self::WILD` and `self::WILD $n = 7;` both die on `E0101 expected
      ';'` rather than on a refusal that names the spelling. `rule:types/class-scoped-alias` refuses
      it for the alias member and is the nearest home; `docs/rules/types/grammar.md:12` is the atom
      list that would state it either way.
- [ ] **Pin whichever way it goes with a reject case**, beside
      `tests/conformance/reject/a-value-that-is-not-a-class-reference-on-the-right-of-is-is-refused.nvst:1`,
      so the answer is a diagnostic with a code rather than a parser's guess at a missing `;`.

## Backlog

- `examples/type-test.nvs` walks a subset of the table; the rest is pinned by conformance cases
  only — `tests/conformance/lang/`.
- `tools/holes.py`'s `ENGINE` regex still excludes `Unsupported` sites by message wording; each one
  it names could be an `Internal` instead — `tools/holes.py:110`.
